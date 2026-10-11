use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use super::{BAD_VALUE, DESCRIPTOR, MAX_REQUEST_BYTES, UNKNOWN_TRANSACTION};
use crate::{attk, ids};

/// `finishSign` keeps the reference reply shape: D-Soter answers it with a fixed zero buffer and
/// keeps no session state between `initSigh` and `finishSign`.
const SIGNATURE: [u8; 256] = [0; 256];

struct Record {
    key: attk::KeyPair,
    blob: Arc<[u8]>,
}

struct User {
    ask: Record,
    auth: HashMap<Vec<u16>, Record>,
}

/// All keys live only in this SoterServer process, isolated by UID and exact UTF-16 alias.
struct KeyStore {
    users: HashMap<u32, User>,
    generate: fn() -> Option<attk::KeyPair>,
    attk_private: Vec<u8>,
}

impl KeyStore {
    fn new(generate: fn() -> Option<attk::KeyPair>, attk_private: Vec<u8>) -> Self {
        Self {
            users: HashMap::new(),
            generate,
            attk_private,
        }
    }

    fn make_record(&self, uid: u32, signer: &[u8]) -> Option<Record> {
        use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
        let key = (self.generate)()?;
        if key.public_der.is_empty() {
            return None;
        }
        let json = format!(
            "{{\"pub_key\":\"{}\",\"counter\":{},\"cpu_id\":\"{}\",\"uid\":{}}}",
            BASE64.encode(&key.public_der),
            attk::KEY_COUNTER,
            ids::cpu_id(),
            uid,
        );
        let signature = attk::sign_export_json(signer, json.as_bytes())?;
        let mut blob = Vec::with_capacity(4 + json.len() + signature.len());
        blob.extend_from_slice(&(json.len() as u32).to_le_bytes());
        blob.extend_from_slice(json.as_bytes());
        blob.extend_from_slice(&signature);
        Some(Record {
            key,
            blob: Arc::from(blob),
        })
    }

    fn generate_ask(&mut self, uid: u32) -> bool {
        let Some(ask) = self.make_record(uid, &self.attk_private) else {
            return false;
        };
        // Publish only after generation and signing succeed; replacing ASK invalidates its children.
        self.users.insert(
            uid,
            User {
                ask,
                auth: HashMap::new(),
            },
        );
        true
    }

    fn generate_auth(&mut self, uid: u32, alias: &[u16]) -> bool {
        if alias.is_empty() {
            return false;
        }
        let Some(user) = self.users.get(&uid) else {
            return false;
        };
        let Some(auth) = self.make_record(uid, &user.ask.key.private_der) else {
            return false;
        };
        self.users
            .get_mut(&uid)
            .expect("existing ASK")
            .auth
            .insert(alias.to_vec(), auth);
        true
    }

    fn record(&self, code: u32, uid: u32, alias: &[u16]) -> Option<&Record> {
        let user = self.users.get(&uid)?;
        if matches!(code, 2 | 3) {
            Some(&user.ask)
        } else {
            user.auth.get(alias)
        }
    }
}

/// Parse arguments at the cursor left by Android's enforced AIDL interface header.
/// The caller-supplied UID is consumed but never used as the key-store identity.
pub(super) fn request_alias(code: u32, bytes: &[u8], position: usize) -> Result<Vec<u16>, i32> {
    if position > bytes.len() || !position.is_multiple_of(4) {
        return Err(BAD_VALUE);
    }
    if !(1..=9).contains(&code) {
        return Ok(Vec::new());
    }
    let position = position
        .checked_add(4)
        .filter(|end| *end <= bytes.len())
        .ok_or(BAD_VALUE)?;
    if !matches!(code, 4 | 5 | 6 | 8 | 9) {
        return Ok(Vec::new());
    }
    let length = i32::from_le_bytes(
        bytes
            .get(position..position + 4)
            .ok_or(BAD_VALUE)?
            .try_into()
            .unwrap(),
    );
    let length = usize::try_from(length)
        .ok()
        .filter(|length| *length > 0)
        .ok_or(BAD_VALUE)?;
    let start = position + 4;
    let end = length
        .checked_add(1)
        .and_then(|length| length.checked_mul(2))
        .and_then(|length| start.checked_add(length))
        .ok_or(BAD_VALUE)?;
    let padded_end = end.checked_add(3).map(|end| end & !3).ok_or(BAD_VALUE)?;
    let name = bytes
        .get(start..end)
        .filter(|_| padded_end <= bytes.len())
        .ok_or(BAD_VALUE)?;
    if name[name.len() - 2..] != [0, 0] {
        return Err(BAD_VALUE);
    }
    Ok(name[..name.len() - 2]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u16::from_le_bytes(*pair))
        .collect())
}

pub(super) fn valid_request(code: u32, bytes: &[u8]) -> bool {
    if !(1..=13).contains(&code) || bytes.len() > MAX_REQUEST_BYTES {
        return false;
    }
    // D-soter identifies the interface by its UTF-16 descriptor anywhere in the
    // driver-validated payload. Android validates the interface header before
    // the native callback parses arguments; trailing OEM fields are preserved.
    let expected = DESCRIPTOR.to_bytes();
    bytes.windows(expected.len() * 2).any(|token| {
        token
            .as_chunks::<2>()
            .0
            .iter()
            .zip(expected)
            .all(|(unit, byte)| *unit == [*byte, 0])
    })
}

pub(super) trait Writer {
    fn int32(&mut self, value: i32) -> Result<(), i32>;
    fn int64(&mut self, value: i64) -> Result<(), i32>;
    fn bytes(&mut self, value: &[u8]) -> Result<(), i32>;
    fn string(&mut self, value: &str) -> Result<(), i32>;
}

pub(super) fn write_reply(
    code: u32,
    uid: u32,
    alias: &[u16],
    output: &mut impl Writer,
) -> Result<(), i32> {
    static KEYS: OnceLock<Mutex<KeyStore>> = OnceLock::new();
    let keys = KEYS.get_or_init(|| {
        Mutex::new(KeyStore::new(
            attk::generate_keypair,
            attk::attk_private_der().unwrap_or_default(),
        ))
    });
    let mut keys = keys.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    write_reply_with_store(code, uid, alias, output, &mut keys)
}

fn write_reply_with_store(
    code: u32,
    uid: u32,
    alias: &[u16],
    output: &mut impl Writer,
    keys: &mut KeyStore,
) -> Result<(), i32> {
    if !(1..=13).contains(&code) {
        return Err(UNKNOWN_TRANSACTION);
    }
    output.int32(0)?; // Java Parcel.writeNoException()
    match code {
        1 | 4 => {
            let ok = if code == 1 {
                keys.generate_ask(uid)
            } else {
                keys.generate_auth(uid, alias)
            };
            if !ok {
                crate::soter::log("Soter key generation or export signing failed");
            }
            output.int32(if ok { 0 } else { -1 })
        }
        5 => {
            if let Some(user) = keys.users.get_mut(&uid) {
                user.auth.remove(alias);
            }
            output.int32(0)
        }
        7 => {
            keys.users.remove(&uid);
            output.int32(0)
        }
        3 | 8 => output.int32(i32::from(keys.record(code, uid, alias).is_some())),
        12 => output.int32(1),
        2 | 6 | 10 | 11 => {
            let blob;
            let bytes: &[u8] = match code {
                2 | 6 => {
                    blob = keys
                        .record(code, uid, alias)
                        .map(|record| Arc::clone(&record.blob));
                    blob.as_deref().unwrap_or_default()
                }
                10 => &SIGNATURE,
                11 => ids::device_blob(),
                _ => return Err(BAD_VALUE),
            };
            output.int32(1)?;
            output.int32(if matches!(code, 2 | 6) && bytes.is_empty() {
                -1
            } else {
                0
            })?;
            output.bytes(bytes)?;
            output.int32(bytes.len() as i32)
        }
        9 => {
            output.int32(1)?;
            output.int64(1)?;
            output.int32(0)
        }
        13 => {
            output.int32(1)?;
            output.int32(0)?; // Parcel.writeValue() VAL_STRING
            output.string("optical")
        }
        _ => Err(UNKNOWN_TRANSACTION),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::soter::tests::Buffer;
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

    fn root() -> Option<attk::KeyPair> {
        Some(attk::KeyPair {
            public_der: BASE64.decode(attk::ATTK_PUBLIC_B64).ok()?,
            private_der: attk::attk_private_der()?,
        })
    }

    // Independent RSA-2048 fixtures generated by OpenSSL; used only by tests.
    fn second() -> Option<attk::KeyPair> {
        Some(attk::KeyPair {
            public_der: BASE64.decode(concat!(
            "MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA3CTbIbg8xdGAg5v9TStZ79FX2jGAGRh69cYW",
            "r+lmx8MQfg1oMxIVCyq2ieiRbgMkbwJAqTPhZxU5Gs+pwMweetwa79r//hHYfaQ5xb5z5HCp2z+izLEM",
            "7gt+Y4iyWzIsGXMpOWwbSTLMt94RsqQPuyz5o9PZNnc4Gg9rwwT9bsusAXyTwtSZ6tuDX5XJp1tNKDol",
            "jnaTLwkq/LGc1p5mbR8LlHdNIwl7/qMYCt46vMQuMHAt7Csp07tgQoTCO7k8zj9XvhabtkusfJ0X2wdV",
            "eC4Wa56GO2xmhzTMdNb9dD3+G8OPeJ4JrbT03cgUSGfbG4pTKTKPL4VsBPk8ae9tnwIDAQAB",
        )).ok()?,
            private_der: BASE64.decode(concat!(
            "MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQDcJNshuDzF0YCDm/1NK1nv0VfaMYAZ",
            "GHr1xhav6WbHwxB+DWgzEhULKraJ6JFuAyRvAkCpM+FnFTkaz6nAzB563Brv2v/+Edh9pDnFvnPkcKnb",
            "P6LMsQzuC35jiLJbMiwZcyk5bBtJMsy33hGypA+7LPmj09k2dzgaD2vDBP1uy6wBfJPC1Jnq24Nflcmn",
            "W00oOiWOdpMvCSr8sZzWnmZtHwuUd00jCXv+oxgK3jq8xC4wcC3sKynTu2BChMI7uTzOP1e+Fpu2S6x8",
            "nRfbB1V4LhZrnoY7bGaHNMx01v10Pf4bw494ngmttPTdyBRIZ9sbilMpMo8vhWwE+Txp722fAgMBAAEC",
            "ggEACDn0Wlk5JCdFByYAHC8fiD+pacTpxYZNqfuCA4bUSWi3qA42M5pcYPNyqMqhDJxfcyqBYhFHPgPk",
            "A2V5MXQvO2YYN5hWcpcWIPPPGN2aFRJEtqW1HrMLCaYyTePgzlmcnkLYaoXoqx6T2dcZkDRjRhpTJdUX",
            "y9o+pXKoEzyPf+FYww0myin3BD+gUx61ugflzJSyVji5r6+wKe0VX8OOfc1SNsmGHH8OI2x9uEDviVy1",
            "BCT96XV9cgptMqgLAirlO745iVItn3SA9CwyGjWt9EfFlZUH+t+T2yRuGxdg2mY8shxqGps5FDLI4YWX",
            "lZTURZrF4U77yZvNhU4LXwOR2QKBgQD58anIqzoWyv0z0msf09xShUzFqpkywN3tuC35K1zzKYGG6elf",
            "9bHdiyuQe4/tMKAZx3ADyFwcTwgUjusnq9/UdOuzWfLmghRMTXeHEs9HZiqYuJ0fRtdWxO+Ob2ENfXsN",
            "Lwpa725R9TJIOieQt/YICbT7QnFT8zuP4xI9sea6CwKBgQDhelndqn9QnO0y76HzcbFhp8nXlpvYLngw",
            "3lwy/7miyfWIawasDSD8lbARC/HQSVG4iqjFAoaEa7sBlFHTMk2AUxyGOpC9tZ1Ai27rKQnwYyjSwG0D",
            "CEfXIa2/sQ1pI+j7IetA+RtDH306+vd5TUqj4VNb3Xc2XXfXNUlKFKfrPQKBgQCWThiO/0FAgbOAnXIk",
            "QrCEw3qmh+/20HrlQNQMumH8LKKi8jmQplze4JlDGrszpQEyX3fiHnQd5fzy9/s78ST6S5GOWnIy4li4",
            "hgFyLgKOakcFbke2jlBi8q0o15vik+oSxI4Sn1eVW8FwI+C1K5t9u/ClBJDILkZMQf4vMbORfwKBgENy",
            "afdYzQS8LGgULom39KuVmVcYAj5EiEd1/HZ08H61kvDfsvs8YiWsUSKIRCleBUgCkkF8Y+iwoRvz1hYq",
            "8ev08I9mbW2SKhvomaFWgVabEtcYIIPLslG0DVlOCQjuaP7WQp0I5xLJFJjUAbsTV8R13o2YTdTBWNvT",
            "YLz8OM6xAoGBAMIsNfDoHxs91BiI80zX40CnT5Wz/1t3xva1GQkUMiMZ8e40xrfTX1BINW6Nuf1FpF70",
            "LDTXmaDmdZglZa+s/7NrA15PnL4DzMIfK1bO/2qp7Rx0P2q3n85VH3Q/05woP6jr8vcABKtKUcCiVn3l",
            "R0DdOCL5NVT8G/X7BazHWKZo",
        )).ok()?,
        })
    }

    // Independent RSA-2048 fixtures generated by OpenSSL; used only by tests.
    fn third() -> Option<attk::KeyPair> {
        Some(attk::KeyPair {
            public_der: BASE64.decode(concat!(
            "MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAuhhABKz5U4z7il2J9eQg29HLetrIRDCFHvUB",
            "yYuSTRaXTtp6pa4GxMjziUtoJMriJAlLxgC2xt6Z6+kzYX3KL9tyUBYlkULv99s0kotIicESvLRaPEm7",
            "0qDjROjAhDxpV0wZbBosKa6PhwuKZrA1H/zpv8MFlbEvGE7ZTW7zQ4Ves7fCIBE+OdqKQ6kpFnGY64ft",
            "B9MnQKSsas6KFt+3/fHFotVTfKShjLHx+ksegz6+6KMHfb5FuS18gY9OazAUiAzVZDHABf6hMJE/G5k8",
            "+4+hD1Rma7vfCNIjb3u8gCLLCT7kkv532dHS9PQGCxmnnuxkNuOzYGVuX+Hwp3k3iwIDAQAB",
        )).ok()?,
            private_der: BASE64.decode(concat!(
            "MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQC6GEAErPlTjPuKXYn15CDb0ct62shE",
            "MIUe9QHJi5JNFpdO2nqlrgbEyPOJS2gkyuIkCUvGALbG3pnr6TNhfcov23JQFiWRQu/32zSSi0iJwRK8",
            "tFo8SbvSoONE6MCEPGlXTBlsGiwpro+HC4pmsDUf/Om/wwWVsS8YTtlNbvNDhV6zt8IgET452opDqSkW",
            "cZjrh+0H0ydApKxqzooW37f98cWi1VN8pKGMsfH6Sx6DPr7oowd9vkW5LXyBj05rMBSIDNVkMcAF/qEw",
            "kT8bmTz7j6EPVGZru98I0iNve7yAIssJPuSS/nfZ0dL09AYLGaee7GQ247NgZW5f4fCneTeLAgMBAAEC",
            "ggEATA70P1T5FEJnXn5rkAG8wkslOKGpv2D7T5NAt0Chu3sTvWlDPmr5nj0peEBHIN6A/cyE2Tjaez5o",
            "t4hY9Swl0liFCKF8mjm6/VjszEY8D/1G7LRjuwdBykzyhmwAnytXvJPAQjQSP7j2NXIsj1rOY7QJT1Rk",
            "eHFx96PmYaUKrNnnNWo1/ihUVIPlJcEK/ZLAzLIHgdx5jtTW8Lu5CNAB9D9LX4NYuylYX50dlPWFuJk4",
            "xPsyARJs986WofEFYapuYhYEnJ6lRx5EEF7kseeHuRoYYYL7pD/PPk0Vo6ctJO5HrxeZFYQSsuecmnoc",
            "QbUkwRnG76/1axJBxKsFUeySDQKBgQDtcpC66Yp+OuWfyM2Txi4kud0MEWcnhoxVhnLzXyEe8r1jArVk",
            "mO29GbSqovqcnfp8Lliy7aT6w+5RoTq7WAeXATmBAQX50cUx4UujBTP0itdhnH7j2qVnX7B7G4n3JBCt",
            "iSIeSfnCReZ8qvPnMOhAYhNg2w1G4LCzKLZmalmNTQKBgQDIooYsTnqbf2a/dyIhAh/jH2GK9hZoh9L2",
            "4KmKi8oixydLlGLaEFeOj9kbVooR6QXzz2rXCcA+/DCOBkp6kpf7WYxYSRYbyvkUB8JN+0KPR1Bgyd/p",
            "a98mMnCMu7/ME9YqUDjo7gQZr+aBdryV65aivpu9fpH6OHpLR6O3CO1MNwKBgGWKiq4D1T/p++x6uL8r",
            "DuyRtx/ECXLzrOueFWZSFh3f/HFs5Rlh9LjbV1FpZPa48DdOsovB6L18MFwkMLORD53Z3jd2spGEzWlp",
            "SSkrvm4jObv6aqESDY3Jc/URb8yKVb+AV2DQjcgwkEJb8PW7CncN+e9/fYwD5kyXankdnbI5AoGBALcI",
            "JVUIJyvyNr7WpOjt8j8VorvNsjWcKyYTYDfHPRkrC7sUXHXI0Md6jTo2YXDjXw3EHBieiaYsFPOL46gc",
            "Uc/4KmNkEWo7aa0PQwA3BsqE6VvS7pUYVpOXEjwhD2DSe99QHgrJEC6K3MMgmfw+1sYTEyr066EUouRI",
            "L59L4odTAoGBAIJaYyDEroDBZCJacrtKYQyY13d7FlLGDuRJe3ej0R5IsnKWYvd6kv+HoEYQA75KlDFt",
            "eZrZP/cj0xsKGda/bDm80fd5PmJ41OTZ5v/wplRymZKgju+gEGTRBDV4SwqXSw2T8m7BEtlq7j4IrowI",
            "YohnR/lkV12xzcdCqROqhqop",
        )).ok()?,
        })
    }

    fn reply(keys: &mut KeyStore, code: u32, uid: u32, alias: &[u16]) -> Vec<u8> {
        let mut output = Buffer::default();
        write_reply_with_store(code, uid, alias, &mut output, keys).unwrap();
        output.0
    }

    fn scalar(bytes: &[u8]) -> i32 {
        i32::from_le_bytes(bytes[4..8].try_into().unwrap())
    }

    #[test]
    fn isolates_keys_and_signs_exports_with_their_parent() {
        const UID: u32 = 10123;
        let a = [0x61];
        let b = [0x61, 0, 0x62, 0xd800];
        let mut keys = KeyStore::new(second, attk::attk_private_der().unwrap());
        assert_eq!(scalar(&reply(&mut keys, 3, UID, &[])), 0);
        assert_eq!(scalar(&reply(&mut keys, 4, UID, &a)), -1);
        let missing = reply(&mut keys, 2, UID, &[]);
        assert_eq!(
            missing,
            [
                0, 0, 0, 0, 1, 0, 0, 0, 255, 255, 255, 255, 0, 0, 0, 0, 0, 0, 0, 0
            ]
        );
        assert_eq!(scalar(&reply(&mut keys, 1, UID, &[])), 0);
        let ask = Arc::clone(&keys.record(2, UID, &[]).unwrap().blob);
        assert_eq!(scalar(&reply(&mut keys, 3, UID, &[])), 1);
        keys.generate = third;
        assert_eq!(scalar(&reply(&mut keys, 4, UID, &a)), 0);
        keys.generate = root;
        assert_eq!(scalar(&reply(&mut keys, 4, UID, &b)), 0);
        let auth_a = Arc::clone(&keys.record(6, UID, &a).unwrap().blob);
        let auth_b = Arc::clone(&keys.record(6, UID, &b).unwrap().blob);
        assert_ne!(
            keys.record(2, UID, &[]).unwrap().key.public_der,
            keys.record(6, UID, &a).unwrap().key.public_der
        );
        assert_ne!(
            keys.record(6, UID, &a).unwrap().key.public_der,
            keys.record(6, UID, &b).unwrap().key.public_der
        );
        assert!(attk::tests::verifies_export(&ask, &keys.attk_private));
        for blob in [&auth_a, &auth_b] {
            assert!(attk::tests::verifies_export(
                blob,
                &keys.users[&UID].ask.key.private_der
            ));
            assert!(!attk::tests::verifies_export(blob, &keys.attk_private));
        }
        let exported = reply(&mut keys, 6, UID, &a);
        assert_eq!(&exported[16..16 + auth_a.len()], &*auth_a);
        assert_eq!(reply(&mut keys, 6, UID, &a), exported);
        assert_eq!(scalar(&reply(&mut keys, 8, UID + 1, &a)), 0);
        assert_eq!(scalar(&reply(&mut keys, 1, UID + 1, &[])), 0);
        keys.generate = root;
        assert_eq!(scalar(&reply(&mut keys, 4, UID + 1, &a)), 0);
        assert_ne!(
            keys.record(6, UID, &a).unwrap().key.public_der,
            keys.record(6, UID + 1, &a).unwrap().key.public_der
        );

        keys.generate = || None;
        assert_eq!(scalar(&reply(&mut keys, 1, UID, &[])), -1);
        assert_eq!(scalar(&reply(&mut keys, 4, UID, &a)), -1);
        assert_eq!(&*keys.record(2, UID, &[]).unwrap().blob, &*ask);
        assert_eq!(&*keys.record(6, UID, &a).unwrap().blob, &*auth_a);
        keys.generate = root;
        assert_eq!(scalar(&reply(&mut keys, 4, UID, &[])), -1);
        assert_eq!(scalar(&reply(&mut keys, 5, UID, &a)), 0);
        assert_eq!(scalar(&reply(&mut keys, 8, UID, &a)), 0);
        assert_eq!(&*keys.record(6, UID, &b).unwrap().blob, &*auth_b);
        assert_eq!(scalar(&reply(&mut keys, 4, UID, &a)), 0);
        assert_ne!(
            keys.record(6, UID, &a).unwrap().key.public_der,
            third().unwrap().public_der
        );
        keys.generate = third;
        assert_eq!(scalar(&reply(&mut keys, 1, UID, &[])), 0);
        assert_ne!(
            keys.record(2, UID, &[]).unwrap().key.public_der,
            second().unwrap().public_der
        );
        assert_eq!(scalar(&reply(&mut keys, 8, UID, &a)), 0);
        assert_eq!(scalar(&reply(&mut keys, 8, UID, &b)), 0);
        assert_eq!(scalar(&reply(&mut keys, 7, UID, &[])), 0);
        assert_eq!(scalar(&reply(&mut keys, 3, UID, &[])), 0);
        assert_eq!(scalar(&reply(&mut keys, 3, UID + 1, &[])), 1);
        assert_eq!(scalar(&reply(&mut keys, 8, UID + 1, &a)), 1);
        let mut bad_signer = KeyStore::new(root, vec![]);
        assert!(!bad_signer.generate_ask(UID));
        assert!(bad_signer.users.is_empty());
    }
}
