use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex, OnceLock, RwLock,
    },
};

use anyhow::{anyhow, bail, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use der::Encode;
use kmr_common::{
    crypto::{ec, rsa, KeyMaterial, Sha256},
    Error,
};
use kmr_crypto_boring::{ec::BoringEc, mldsa::BoringMlDsa, rsa::BoringRsa, sha256::BoringSha256};
use kmr_ta::device::{
    RetrieveCertSigningInfo, SigningAlgorithm, SigningInfoSnapshot, SigningKeyType,
};
use kmr_wire::keymint;
use log::{debug, error, info, warn};
use quick_xml::events::Event;
use quick_xml::Reader;
use x509_cert::der as x509_der;
use x509_cert::Certificate;

pub const KEYBOX_PATH: &str = "/data/misc/keystore/omk/keybox.xml";

const BUNDLED_KEYBOX_XML: &str = include_str!("../template/keybox.xml");

lazy_static::lazy_static! {
    pub static ref KEYBOX: RwLock<KeyBox> = RwLock::new(KeyBox::new());
    static ref KEYBOX_IO_LOCK: Mutex<()> = Mutex::new(());
}

static KEYBOX_WATCHER: OnceLock<()> = OnceLock::new();
static KEYBOX_DB_RETIRE_ALLOWED: AtomicBool = AtomicBool::new(false);
static KEYBOX_RUNTIME_LOADED: AtomicBool = AtomicBool::new(false);

#[derive(Clone)]
pub struct CertSignAlgoInfo {
    key: KeyMaterial,
    key_der: Vec<u8>,
    chain: Vec<keymint::Certificate>,
}

#[derive(Clone)]
struct KeyEntry {
    algorithm: SigningAlgorithm,
    info: CertSignAlgoInfo,
}

#[derive(Clone)]
pub struct KeyBox {
    entries: Vec<KeyEntry>,
    identity_digest: [u8; 32],
}

struct ParsedKeyEntry {
    key_der: Vec<u8>,
    chain: Vec<Vec<u8>>,
}

impl KeyBox {
    pub fn new() -> Self {
        Self::from_xml_str(BUNDLED_KEYBOX_XML).expect("bundled keybox.xml must be valid")
    }

    pub fn from_xml_str(xml: &str) -> Result<Self> {
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut entries = Vec::new();
        let mut current_private_key_pem = String::new();
        let mut current_cert_pems: Vec<String> = Vec::new();
        let mut in_private_key = false;
        let mut in_certificate = false;

        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) => match e.name().as_ref() {
                    b"Key" => {
                        current_private_key_pem.clear();
                        current_cert_pems.clear();
                    }
                    b"PrivateKey" => in_private_key = true,
                    b"Certificate" => in_certificate = true,
                    _ => {}
                },
                Ok(Event::Text(e)) => {
                    if let Ok(text) = e.unescape() {
                        if in_private_key {
                            current_private_key_pem = text.to_string();
                        } else if in_certificate {
                            current_cert_pems.push(text.to_string());
                        }
                    }
                }
                Ok(Event::End(e)) => match e.name().as_ref() {
                    b"PrivateKey" => in_private_key = false,
                    b"Certificate" => in_certificate = false,
                    b"Key" => {
                        if !current_private_key_pem.is_empty() && !current_cert_pems.is_empty() {
                            match Self::build_entry_from_pems(
                                &current_private_key_pem,
                                &current_cert_pems,
                            ) {
                                Ok(entry) => entries.push(entry),
                                Err(err) => {
                                    warn!("skipping key entry in keybox.xml: {err:#}");
                                }
                            }
                        }
                    }
                    _ => {}
                },
                Ok(Event::Eof) => break,
                Err(err) => {
                    bail!("failed to parse keybox.xml: {err}");
                }
                _ => {}
            }
        }

        if entries.is_empty() {
            bail!("no valid key entries found in keybox.xml");
        }

        let identity_digest = Self::compute_identity_digest(&entries)?;
        Ok(Self {
            entries,
            identity_digest,
        })
    }

    fn build_entry_from_pems(
        private_key_pem: &str,
        cert_pems: &[String],
    ) -> Result<KeyEntry> {
        let key_der = decode_pem(private_key_pem)?;
        let chain_raw: Vec<Vec<u8>> = cert_pems
            .iter()
            .map(|pem| decode_pem(pem))
            .collect::<Result<Vec<_>>>()?;

        if chain_raw.is_empty() {
            bail!("certificate chain is empty");
        }

        let (key, algorithm) = infer_algorithm_and_import(&key_der)
            .context("failed to import private key")?;

        let chain: Vec<keymint::Certificate> = chain_raw
            .into_iter()
            .map(|encoded_certificate| keymint::Certificate {
                encoded_certificate,
            })
            .collect();

        validate_chain_matches_key(&key, &chain, algorithm)?;

        Ok(KeyEntry {
            algorithm,
            info: CertSignAlgoInfo {
                key,
                key_der,
                chain,
            },
        })
    }

    fn compute_identity_digest(entries: &[KeyEntry]) -> Result<[u8; 32]> {
        let mut material = Vec::new();
        for (i, entry) in entries.iter().enumerate() {
            let label = algorithm_label(entry.algorithm);
            let indexed_label = format!("{label}-{i}");
            append_labeled_bytes(&mut material, indexed_label.as_bytes(), &entry.info.key_der);
            append_labeled_chain(&mut material, indexed_label.as_bytes(), &entry.info.chain);
        }

        BoringSha256 {}
            .hash(&material)
            .map_err(|e| anyhow!("failed to hash keybox identity: {e:?}"))
    }

    fn refresh_identity_digest(&mut self) -> Result<()> {
        self.identity_digest = Self::compute_identity_digest(&self.entries)?;
        Ok(())
    }

    pub fn identity_digest(&self) -> [u8; 32] {
        self.identity_digest
    }

    fn signing_info(&self, key_type: SigningKeyType) -> Result<SigningInfoSnapshot, Error> {
        let entry = self
            .entries
            .iter()
            .find(|e| e.algorithm == key_type.algo_hint)
            .ok_or_else(|| {
                kmr_common::km_err!(
                    UnknownError,
                    "no {} key in keybox",
                    algorithm_label(key_type.algo_hint)
                )
            })?;

        Ok(SigningInfoSnapshot {
            signing_key: entry.info.key.clone(),
            cert_chain: entry.info.chain.clone(),
            identity_digest: self.identity_digest,
        })
    }

    pub fn update_rsa_keybox(
        &mut self,
        key_der: Vec<u8>,
        chain: Vec<keymint::Certificate>,
    ) -> Result<()> {
        self.update_keybox(SigningAlgorithm::Rsa, key_der, chain)
    }

    pub fn update_ec_keybox(
        &mut self,
        key_der: Vec<u8>,
        chain: Vec<keymint::Certificate>,
    ) -> Result<()> {
        self.update_keybox(SigningAlgorithm::Ec, key_der, chain)
    }

    fn update_keybox(
        &mut self,
        algorithm: SigningAlgorithm,
        key_der: Vec<u8>,
        chain: Vec<keymint::Certificate>,
    ) -> Result<()> {
        let entry = ParsedKeyEntry {
            key_der,
            chain: chain
                .into_iter()
                .map(|certificate| certificate.encoded_certificate)
                .collect(),
        };
        let (key, inferred_algo) = infer_algorithm_and_import(&entry.key_der)?;
        if inferred_algo != algorithm {
            warn!(
                "keybox update: algorithm mismatch (expected {:?}, inferred {:?})",
                algorithm_label(algorithm),
                algorithm_label(inferred_algo)
            );
        }
        let new_info = build_info_from_entry(entry, inferred_algo)?;

        self.entries.retain(|e| e.algorithm != algorithm);
        self.entries.push(KeyEntry {
            algorithm: inferred_algo,
            info: new_info,
        });
        self.entries.sort_by_key(|e| e.algorithm);
        self.refresh_identity_digest()
    }

    pub fn to_xml_string(&self) -> String {
        let key_blocks: Vec<String> = self.entries.iter().map(|e| entry_to_xml_block(e)).collect();
        format!(
            concat!(
                "<?xml version=\"1.0\"?>\n",
                "<AndroidAttestation>\n",
                "<NumberOfKeyboxes>2</NumberOfKeyboxes>\n",
                "<Keybox DeviceID=\"sw\">\n",
                "{}\n",
                "</Keybox>\n",
                "</AndroidAttestation>\n"
            ),
            key_blocks.join("\n")
        )
    }
}

impl Default for KeyBox {
    fn default() -> Self {
        Self::new()
    }
}

fn algorithm_label(algorithm: SigningAlgorithm) -> &'static str {
    match algorithm {
        SigningAlgorithm::Ec => "ecdsa",
        SigningAlgorithm::Rsa => "rsa",
    }
}

fn private_key_label(algorithm: SigningAlgorithm) -> &'static str {
    match algorithm {
        SigningAlgorithm::Ec => "EC PRIVATE KEY",
        SigningAlgorithm::Rsa => "RSA PRIVATE KEY",
    }
}

fn entry_to_xml_block(entry: &KeyEntry) -> String {
    let name = algorithm_label(entry.algorithm);
    let private_label = private_key_label(entry.algorithm);
    let certificates = entry
        .info
        .chain
        .iter()
        .map(|certificate| {
            format!(
                "<Certificate format=\"pem\">\n{}\n</Certificate>",
                encode_pem_block("CERTIFICATE", &certificate.encoded_certificate)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        concat!(
            "<Key algorithm=\"{name}\">\n",
            "<PrivateKey format=\"pem\">\n",
            "{private_key}\n",
            "</PrivateKey>\n",
            "<CertificateChain>\n",
            "<NumberOfCertificates>{cert_count}</NumberOfCertificates>\n",
            "{certificates}\n",
            "</CertificateChain>\n",
            "</Key>"
        ),
        name = name,
        private_key = encode_pem_block(private_label, &entry.info.key_der),
        cert_count = entry.info.chain.len(),
        certificates = certificates,
    )
}

fn infer_algorithm_and_import(key_der: &[u8]) -> Result<(KeyMaterial, SigningAlgorithm)> {
    if let Ok(key) = rsa::import_pkcs1_key(key_der).map(|(k, _, _)| k) {
        return Ok((key, SigningAlgorithm::Rsa));
    }
    if let Ok(key) = ec::import_sec1_private_key(key_der) {
        return Ok((key, SigningAlgorithm::Ec));
    }
    bail!("failed to import private key as RSA (PKCS#1) or EC (SEC1)")
}

fn build_info_from_entry(entry: ParsedKeyEntry, algorithm: SigningAlgorithm) -> Result<CertSignAlgoInfo> {
    if entry.chain.is_empty() {
        bail!("{} certificate chain is empty", algorithm_label(algorithm));
    }
    let (key, _) = infer_algorithm_and_import(&entry.key_der)?;
    let chain: Vec<keymint::Certificate> = entry
        .chain
        .into_iter()
        .map(|encoded_certificate| keymint::Certificate {
            encoded_certificate,
        })
        .collect();
    validate_chain_matches_key(&key, &chain, algorithm)?;
    Ok(CertSignAlgoInfo {
        key,
        key_der: entry.key_der,
        chain,
    })
}

fn append_labeled_bytes(buffer: &mut Vec<u8>, label: &[u8], data: &[u8]) {
    buffer.extend_from_slice(&(label.len() as u32).to_be_bytes());
    buffer.extend_from_slice(label);
    buffer.extend_from_slice(&(data.len() as u32).to_be_bytes());
    buffer.extend_from_slice(data);
}

fn append_labeled_chain(buffer: &mut Vec<u8>, label: &[u8], chain: &[keymint::Certificate]) {
    buffer.extend_from_slice(&(label.len() as u32).to_be_bytes());
    buffer.extend_from_slice(label);
    buffer.extend_from_slice(&(chain.len() as u32).to_be_bytes());
    for certificate in chain {
        buffer.extend_from_slice(&(certificate.encoded_certificate.len() as u32).to_be_bytes());
        buffer.extend_from_slice(&certificate.encoded_certificate);
    }
}

fn validate_chain_matches_key(
    key: &KeyMaterial,
    chain: &[keymint::Certificate],
    algorithm: SigningAlgorithm,
) -> Result<()> {
    let first_cert = chain
        .first()
        .context("certificate chain must contain a leaf certificate")?;
    let certificate = <Certificate as x509_der::Decode>::from_der(&first_cert.encoded_certificate)
        .with_context(|| {
            format!(
                "failed to parse {} leaf certificate from keybox chain",
                algorithm_label(algorithm)
            )
        })?;
    let mut spki_buf = Vec::new();
    let derived_spki = key
        .subject_public_key_info(
            &mut spki_buf,
            &BoringEc::default(),
            &BoringRsa::default(),
            &BoringMlDsa,
        )
        .map_err(|e| {
            anyhow!(
                "failed to derive {} public key info from private key: {e:?}",
                algorithm_label(algorithm)
            )
        })?
        .context("symmetric key cannot back an attestation certificate")?
        .to_der()
        .with_context(|| {
            format!(
                "failed to encode {} public key info from private key",
                algorithm_label(algorithm)
            )
        })?;
    let certificate_spki =
        x509_der::Encode::to_der(&certificate.tbs_certificate.subject_public_key_info)
            .with_context(|| {
                format!(
                    "failed to encode {} public key info from certificate chain",
                    algorithm_label(algorithm)
                )
            })?;
    if derived_spki != certificate_spki {
        bail!(
            "{} certificate chain does not match the supplied private key",
            algorithm_label(algorithm)
        );
    }
    Ok(())
}

fn decode_pem(pem: &str) -> Result<Vec<u8>> {
    let base64_body = pem
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| !line.starts_with("-----BEGIN ") && !line.starts_with("-----END "))
        .collect::<String>();

    if base64_body.is_empty() {
        bail!("empty PEM payload");
    }

    STANDARD
        .decode(base64_body.as_bytes())
        .context("failed to decode PEM payload")
}

fn encode_pem_block(label: &str, der: &[u8]) -> String {
    let mut pem = String::new();
    pem.push_str(&format!("-----BEGIN {label}-----\n"));
    let base64 = STANDARD.encode(der);
    for chunk in base64.as_bytes().chunks(64) {
        pem.push_str(std::str::from_utf8(chunk).expect("base64 is valid UTF-8"));
        pem.push('\n');
    }
    pem.push_str(&format!("-----END {label}-----"));
    pem
}

fn temp_keybox_path(path: &str) -> PathBuf {
    let target = Path::new(path);
    let file_name = target
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("keybox.xml");
    let temp_name = format!(".{file_name}.tmp-{}", std::process::id());
    target
        .parent()
        .map(|parent| parent.join(&temp_name))
        .unwrap_or_else(|| PathBuf::from(temp_name))
}

fn write_keybox_xml(path: &str, xml: &str) -> Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create keybox directory {}", parent.display()))?;
    }
    let temp_path = temp_keybox_path(path);
    {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&temp_path)
            .with_context(|| {
                format!(
                    "failed to open temporary keybox.xml {}",
                    temp_path.display()
                )
            })?;
        file.write_all(xml.as_bytes()).with_context(|| {
            format!(
                "failed to write temporary keybox.xml {}",
                temp_path.display()
            )
        })?;
        file.sync_all().with_context(|| {
            format!(
                "failed to sync temporary keybox.xml {}",
                temp_path.display()
            )
        })?;
    }

    #[cfg(windows)]
    if Path::new(path).exists() {
        fs::remove_file(path)
            .with_context(|| format!("failed to replace keybox.xml at {path} on Windows"))?;
    }

    fs::rename(&temp_path, path)
        .with_context(|| format!("failed to atomically replace keybox.xml at {path}"))
}

fn write_bundled_keybox(path: &str) -> Result<()> {
    write_keybox_xml(path, BUNDLED_KEYBOX_XML)
}

pub fn ensure_keybox_file(path: &str) -> Result<()> {
    if Path::new(path).exists() {
        return Ok(());
    }
    info!("keybox.xml missing at {}; seeding bundled template", path);
    write_bundled_keybox(path)
}

fn is_bundled_keybox_xml(xml: &str) -> bool {
    xml.trim() == BUNDLED_KEYBOX_XML.trim()
}

fn retire_stale_keybox_bound_entries(current_identity: [u8; 32]) {
    if !db_retirement_allowed() {
        warn!("skipping stale keybox-bound DB retirement while active keybox came from fallback");
        return;
    }

    match crate::global::DB.with(|db| {
        db.borrow_mut()
            .retire_stale_keybox_bound_entries(current_identity)
    }) {
        Ok(0) => debug!("no stale keybox-bound key entries needed retirement"),
        Ok(retired) => info!("retired {retired} stale keybox-bound key entries"),
        Err(error) => error!("failed to retire stale keybox-bound key entries: {error:#}"),
    }
}

fn install_keybox(
    new_keybox: KeyBox,
    retire_db_entries: bool,
    db_retirement_allowed: bool,
) -> bool {
    let new_identity = new_keybox.identity_digest();
    let changed = {
        let mut keybox = KEYBOX.write().unwrap();
        let changed = keybox.identity_digest() != new_identity;
        *keybox = new_keybox;
        changed
    };
    KEYBOX_DB_RETIRE_ALLOWED.store(db_retirement_allowed, Ordering::Release);
    KEYBOX_RUNTIME_LOADED.store(true, Ordering::Release);

    if changed {
        crate::keymaster::keymint_device::clear_initialized_attestation_caches();
    }

    if retire_db_entries {
        retire_stale_keybox_bound_entries(new_identity);
    }

    changed
}

pub fn db_retirement_allowed() -> bool {
    KEYBOX_DB_RETIRE_ALLOWED.load(Ordering::Acquire)
}

fn is_fallback_continuation(keybox: &KeyBox, contents: &str) -> bool {
    let current_identity = KEYBOX
        .read()
        .map(|current| current.identity_digest())
        .unwrap_or([0u8; 32]);
    is_fallback_continuation_with_state(
        keybox,
        contents,
        KEYBOX_RUNTIME_LOADED.load(Ordering::Acquire),
        db_retirement_allowed(),
        current_identity,
    )
}

fn is_fallback_continuation_with_state(
    keybox: &KeyBox,
    contents: &str,
    runtime_loaded: bool,
    retirement_allowed: bool,
    current_identity: [u8; 32],
) -> bool {
    runtime_loaded
        && !retirement_allowed
        && is_bundled_keybox_xml(contents)
        && current_identity == keybox.identity_digest()
}

fn load_keybox_with_fallback(path: &str) -> Result<(KeyBox, bool)> {
    match fs::read_to_string(path) {
        Ok(contents) => match KeyBox::from_xml_str(&contents) {
            Ok(keybox) => {
                let fallback_origin = is_fallback_continuation(&keybox, &contents);
                Ok((keybox, fallback_origin))
            }
            Err(error) => {
                warn!(
                    "invalid keybox.xml at {}: {:#}; rewriting bundled template",
                    path, error
                );
                write_bundled_keybox(path)?;
                Ok((KeyBox::new(), true))
            }
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            info!("keybox.xml missing at {}; writing bundled template", path);
            write_bundled_keybox(path)?;
            Ok((KeyBox::new(), true))
        }
        Err(error) => Err(error).with_context(|| format!("failed to read keybox.xml from {path}")),
    }
}

pub fn reload_from_disk() -> Result<bool> {
    reload_from_disk_inner(true)
}

fn reload_from_disk_inner(retire_db_entries: bool) -> Result<bool> {
    let _io_guard = KEYBOX_IO_LOCK.lock().unwrap();
    let (keybox, used_fallback) = load_keybox_with_fallback(KEYBOX_PATH)?;
    let changed = install_keybox(keybox, retire_db_entries, !used_fallback);
    if changed {
        info!(
            "active keybox identity updated from {} (fallback={})",
            KEYBOX_PATH, used_fallback
        );
    } else {
        debug!(
            "keybox reload completed without identity change (fallback={})",
            used_fallback
        );
    }
    Ok(changed)
}

pub fn initialize() -> Result<()> {
    ensure_keybox_file(KEYBOX_PATH)?;
    reload_from_disk_inner(false)?;
    KEYBOX_WATCHER.get_or_init(|| {
        if let Err(error) = kmr_common::runtime::file_watch::spawn_path_watcher(
            "omk-keybox-watch",
            PathBuf::from(KEYBOX_PATH),
            |_trigger| {
                if let Err(reload_error) = reload_from_disk() {
                    error!("failed to reload keybox.xml after change: {reload_error:#}");
                }
            },
        ) {
            error!("failed to watch keybox.xml: {error:#}");
        }
    });
    Ok(())
}

pub fn update_rsa_keybox(key_der: Vec<u8>, chain: Vec<keymint::Certificate>) -> Result<bool> {
    update_keybox_file(SigningAlgorithm::Rsa, key_der, chain)
}

pub fn update_ec_keybox(key_der: Vec<u8>, chain: Vec<keymint::Certificate>) -> Result<bool> {
    update_keybox_file(SigningAlgorithm::Ec, key_der, chain)
}

fn update_keybox_file(
    algorithm: SigningAlgorithm,
    key_der: Vec<u8>,
    chain: Vec<keymint::Certificate>,
) -> Result<bool> {
    let _io_guard = KEYBOX_IO_LOCK.lock().unwrap();
    let mut keybox = KEYBOX.read().unwrap().clone();
    keybox.update_keybox(algorithm, key_der, chain)?;
    write_keybox_xml(KEYBOX_PATH, &keybox.to_xml_string())?;
    Ok(install_keybox(keybox, true, true))
}

pub fn current_identity_digest() -> [u8; 32] {
    KEYBOX.read().unwrap().identity_digest()
}

pub struct KeyboxManager;

impl RetrieveCertSigningInfo for KeyboxManager {
    fn signing_info(&self, key_type: SigningKeyType) -> Result<SigningInfoSnapshot, Error> {
        let keybox = KEYBOX
            .read()
            .map_err(|_| kmr_common::km_err!(UnknownError, "failed to lock KEYBOX"))?;
        keybox.signing_info(key_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kmr_ta::device::SigningKey;

    fn write_temp_keybox(name: &str, contents: &str) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("omk-keybox-{name}-{}.xml", std::process::id()));
        fs::write(&path, contents).unwrap();
        path
    }

    #[test]
    fn parses_bundled_template() {
        let keybox = KeyBox::from_xml_str(BUNDLED_KEYBOX_XML).unwrap();
        assert_eq!(keybox.entries.len(), 2);
        let ec_entry = keybox
            .entries
            .iter()
            .find(|e| e.algorithm == SigningAlgorithm::Ec)
            .unwrap();
        let rsa_entry = keybox
            .entries
            .iter()
            .find(|e| e.algorithm == SigningAlgorithm::Rsa)
            .unwrap();
        assert_eq!(ec_entry.info.chain.len(), 2);
        assert_eq!(rsa_entry.info.chain.len(), 2);
        assert_ne!(keybox.identity_digest(), [0u8; 32]);
    }

    #[test]
    fn rejects_invalid_xml() {
        assert!(KeyBox::from_xml_str("<AndroidAttestation/>").is_err());
    }

    #[test]
    fn identity_changes_when_chain_changes() {
        let original = KeyBox::from_xml_str(BUNDLED_KEYBOX_XML).unwrap();
        let mut changed = original.clone();
        let ec_entry = changed
            .entries
            .iter_mut()
            .find(|e| e.algorithm == SigningAlgorithm::Ec)
            .unwrap();
        ec_entry.info.chain.push(ec_entry.info.chain[0].clone());
        changed.refresh_identity_digest().unwrap();

        let modified = KeyBox::from_xml_str(&changed.to_xml_string()).unwrap();
        assert_ne!(original.identity_digest(), modified.identity_digest());
    }

    #[test]
    fn rejects_mismatched_private_key_and_certificate_chain() {
        let keybox = KeyBox::from_xml_str(BUNDLED_KEYBOX_XML).unwrap();
        let rsa_cert = encode_pem_block(
            "CERTIFICATE",
            &keybox
                .entries
                .iter()
                .find(|e| e.algorithm == SigningAlgorithm::Rsa)
                .unwrap()
                .info
                .chain[0]
                .encoded_certificate,
        );
        let ec_cert = encode_pem_block(
            "CERTIFICATE",
            &keybox
                .entries
                .iter()
                .find(|e| e.algorithm == SigningAlgorithm::Ec)
                .unwrap()
                .info
                .chain[0]
                .encoded_certificate,
        );
        let modified_xml = BUNDLED_KEYBOX_XML.replacen(&rsa_cert, &ec_cert, 1);
        assert!(KeyBox::from_xml_str(&modified_xml).is_err());
    }

    #[test]
    fn signing_snapshot_keeps_key_chain_and_digest_in_sync() {
        let keybox = KeyBox::from_xml_str(BUNDLED_KEYBOX_XML).unwrap();

        let rsa_snapshot = keybox
            .signing_info(SigningKeyType {
                which: SigningKey::Batch,
                algo_hint: SigningAlgorithm::Rsa,
            })
            .unwrap();
        assert_eq!(rsa_snapshot.identity_digest, keybox.identity_digest());
        validate_chain_matches_key(
            &rsa_snapshot.signing_key,
            &rsa_snapshot.cert_chain,
            SigningAlgorithm::Rsa,
        )
        .unwrap();

        let ec_snapshot = keybox
            .signing_info(SigningKeyType {
                which: SigningKey::Batch,
                algo_hint: SigningAlgorithm::Ec,
            })
            .unwrap();
        assert_eq!(ec_snapshot.identity_digest, keybox.identity_digest());
        validate_chain_matches_key(
            &ec_snapshot.signing_key,
            &ec_snapshot.cert_chain,
            SigningAlgorithm::Ec,
        )
        .unwrap();
    }

    #[test]
    fn invalid_file_falls_back_to_bundled_template() {
        let path = write_temp_keybox("invalid", "<not-xml>");
        let (keybox, used_fallback) = load_keybox_with_fallback(path.to_str().unwrap()).unwrap();
        assert!(used_fallback);
        assert_eq!(keybox.identity_digest(), KeyBox::new().identity_digest());
        let written = fs::read_to_string(path).unwrap();
        assert!(written.contains("<AndroidAttestation>"));
    }

    #[test]
    fn explicit_bundled_template_is_retirement_eligible_before_runtime_fallback() {
        let keybox = KeyBox::from_xml_str(BUNDLED_KEYBOX_XML).unwrap();

        assert!(!is_fallback_continuation_with_state(
            &keybox,
            BUNDLED_KEYBOX_XML,
            false,
            false,
            keybox.identity_digest(),
        ));
    }

    #[test]
    fn non_bundled_keybox_is_retirement_eligible() {
        let modified_xml = format!("{BUNDLED_KEYBOX_XML}\n<!-- explicit local keybox -->\n");
        let path = write_temp_keybox("modified", &modified_xml);

        let (_, used_fallback) = load_keybox_with_fallback(path.to_str().unwrap()).unwrap();

        assert!(!used_fallback);
    }

    #[test]
    fn accepts_multiple_keys_per_algorithm() {
        let mut keybox = KeyBox::from_xml_str(BUNDLED_KEYBOX_XML).unwrap();
        assert_eq!(keybox.entries.len(), 2);

        let ec_entry = keybox
            .entries
            .iter()
            .find(|e| e.algorithm == SigningAlgorithm::Ec)
            .unwrap()
            .clone();
        keybox.entries.push(ec_entry);
        keybox.refresh_identity_digest().unwrap();

        let xml = keybox.to_xml_string();
        let parsed = KeyBox::from_xml_str(&xml).unwrap();
        assert_eq!(parsed.entries.len(), 3);

        let ec_count = parsed
            .entries
            .iter()
            .filter(|e| e.algorithm == SigningAlgorithm::Ec)
            .count();
        let rsa_count = parsed
            .entries
            .iter()
            .filter(|e| e.algorithm == SigningAlgorithm::Rsa)
            .count();
        assert_eq!(ec_count, 2);
        assert_eq!(rsa_count, 1);

        let snapshot = parsed
            .signing_info(SigningKeyType {
                which: SigningKey::Batch,
                algo_hint: SigningAlgorithm::Ec,
            })
            .unwrap();
        assert_eq!(snapshot.identity_digest, parsed.identity_digest());
    }

    #[test]
    fn single_algorithm_keybox_missing_other_fails() {
        let keybox = KeyBox::from_xml_str(BUNDLED_KEYBOX_XML).unwrap();
        let ec_entry = keybox
            .entries
            .iter()
            .find(|e| e.algorithm == SigningAlgorithm::Ec)
            .unwrap()
            .clone();

        let ec_only = KeyBox {
            entries: vec![ec_entry],
            identity_digest: [0u8; 32],
        };
        let mut ec_only = ec_only;
        ec_only.refresh_identity_digest().unwrap();

        let xml = ec_only.to_xml_string();
        let parsed = KeyBox::from_xml_str(&xml).unwrap();
        assert_eq!(parsed.entries.len(), 1);

        assert!(parsed
            .signing_info(SigningKeyType {
                which: SigningKey::Batch,
                algo_hint: SigningAlgorithm::Rsa,
            })
            .is_err());
        assert!(parsed
            .signing_info(SigningKeyType {
                which: SigningKey::Batch,
                algo_hint: SigningAlgorithm::Ec,
            })
            .is_ok());
    }

    #[test]
    fn algorithm_inferred_from_key_material_not_xml_tag() {
        let keybox = KeyBox::from_xml_str(BUNDLED_KEYBOX_XML).unwrap();

        let altered_xml = keybox.to_xml_string().replace(
            "<Key algorithm=\"ecdsa\">",
            "<Key algorithm=\"rsa\">",
        );
        let parsed = KeyBox::from_xml_str(&altered_xml).unwrap();
        assert_eq!(parsed.entries.len(), 2);
        let inferred = parsed
            .entries
            .iter()
            .find(|e| e.algorithm == SigningAlgorithm::Ec);
        assert!(
            inferred.is_some(),
            "EC key should be inferred from key material, not XML tag"
        );
    }
