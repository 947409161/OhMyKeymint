use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use super::{BAD_VALUE, DESCRIPTOR, MAX_REQUEST_BYTES, UNKNOWN_TRANSACTION};
use crate::{attk, ids};

/// `finishSign` keeps the reference reply shape: D-Soter answers it with a fixed zero buffer and
/// keeps no session state between `initSigh` and `finishSign`.
const SIGNATURE: [u8; 256] = [0; 256];

/// The layout the Soter client parses in `SoterCoreBase.retrieveJsonFromExportedData`:
/// `[uint32 LE json length][json bytes][signature]`, with the SHA256withRSA/PSS signature taken
/// over exactly the json bytes. The public key inside the json and the signing key are the same
/// pair, so the client can verify what it receives.
fn build_export_blob(uid: u32) -> Vec<u8> {
    let json = format!(
        "{{\"pub_key\":\"{}\",\"counter\":{},\"cpu_id\":\"{}\",\"uid\":{}}}",
        attk::ATTK_PUBLIC_B64,
        attk::KEY_COUNTER,
        ids::cpu_id(),
        uid,
    );
    let mut blob = Vec::with_capacity(4 + json.len() + 256);
    blob.extend_from_slice(&(json.len() as u32).to_le_bytes());
    blob.extend_from_slice(json.as_bytes());
    match attk::sign_export_json(json.as_bytes()) {
        Some(signature) => blob.extend_from_slice(&signature),
        // An unsigned blob is shorter than the client expects, but leaving the key pair out is
        // worse than shipping the export: the caller sees the failure in the log.
        None => crate::soter::log("ATTK signing failed; export blob would not verify"),
    }
    blob
}

/// One blob per requesting uid, built once. Real installs key the export on the caller, so a
/// single process-wide blob would hand every caller the same identity.
fn export_blob(uid: u32) -> Arc<[u8]> {
    static BLOBS: OnceLock<Mutex<HashMap<u32, Arc<[u8]>>>> = OnceLock::new();
    let blobs = BLOBS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut blobs = blobs.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(blob) = blobs.get(&uid) {
        return Arc::clone(blob);
    }
    let blob: Arc<[u8]> = Arc::from(build_export_blob(uid));
    blobs.insert(uid, Arc::clone(&blob));
    blob
}

pub(super) fn valid_request(code: u32, bytes: &[u8]) -> bool {
    if !(1..=13).contains(&code) || bytes.len() > MAX_REQUEST_BYTES {
        return false;
    }
    // D-soter identifies the interface by its UTF-16 descriptor anywhere in the
    // driver-validated payload. Its replies do not consume arguments: OEM AIDL
    // extensions and additional fields must not silently select the real HAL.
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

pub(super) fn write_reply(code: u32, uid: u32, output: &mut impl Writer) -> Result<(), i32> {
    if !(1..=13).contains(&code) {
        return Err(UNKNOWN_TRANSACTION);
    }
    output.int32(0)?; // Java Parcel.writeNoException()
    match code {
        1 | 4 | 5 | 7 => output.int32(0),
        3 | 8 | 12 => output.int32(1),
        2 | 6 | 10 | 11 => {
            let blob;
            let bytes: &[u8] = match code {
                2 | 6 => {
                    blob = export_blob(uid);
                    &blob
                }
                10 => &SIGNATURE,
                11 => ids::device_blob(),
                _ => return Err(BAD_VALUE),
            };
            output.int32(1)?;
            output.int32(0)?;
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
