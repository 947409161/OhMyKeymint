use std::{
    ffi::CStr,
    io::{Read, Write},
    time::Duration,
};

use jni::{JNIEnv, objects::JString};
use zygisk_api::{
    ZygiskModule,
    api::{
        V4, ZygiskApi,
        v4::{AppSpecializeArgs, ServerSpecializeArgs, ZygiskOption},
    },
};

use libc::{c_char, c_void};

mod attk;
mod ids;
mod soter;

const SOTER_PROCESS: &str = soter::PACKAGE;
const IPC_TIMEOUT: Duration = Duration::from_secs(2);
const MAX_PROFILE_BYTES: usize = 4 * 1024;

#[derive(Default)]
struct SoterSpoofModule;

impl ZygiskModule for SoterSpoofModule {
    type Api = V4;

    fn pre_app_specialize<'a>(
        &self,
        mut api: ZygiskApi<'a, V4>,
        mut env: JNIEnv<'a>,
        args: &'a mut AppSpecializeArgs<'_>,
    ) {
        let process = read_java_string(&mut env, args.nice_name);
        let data_dir = read_java_string(&mut env, args.app_data_dir);
        if is_soter_target(process.as_deref(), data_dir.as_deref()) {
            soter::log("Soter Beta target selected: com.tencent.soter.soterserver");
            match api.with_companion(read_soter_from_companion) {
                Ok(Ok(true)) => {
                    // Native Binder callbacks may outlive a partially installed
                    // hook. Keep the payload mapped once installation is attempted.
                    match soter::install(&mut api) {
                        Ok(()) => soter::log("Soter Beta hook registered; awaiting specialization"),
                        Err(error) => soter::log(&format!("Soter Beta hook unavailable: {error}")),
                    }
                }
                result => {
                    if matches!(result, Ok(Ok(false))) {
                        soter::log("Soter Beta is disabled");
                    } else {
                        soter::log(&format!("Soter Beta companion unavailable: {result:?}"));
                    }
                    api.set_option(ZygiskOption::DlCloseModuleLibrary);
                }
            }
            return;
        }
        api.set_option(ZygiskOption::DlCloseModuleLibrary);
    }

    fn post_app_specialize<'a>(
        &self,
        _api: ZygiskApi<'a, V4>,
        _env: JNIEnv<'a>,
        _args: &'a AppSpecializeArgs<'a>,
    ) {
        soter::activate();
    }

    fn pre_server_specialize<'a>(
        &self,
        mut api: ZygiskApi<'a, V4>,
        _env: JNIEnv<'a>,
        _args: &'a mut ServerSpecializeArgs<'_>,
    ) {
        api.set_option(ZygiskOption::DlCloseModuleLibrary);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct HookTarget {
    device: libc::dev_t,
    inode: libc::ino_t,
}

fn resolve_symbol_address(symbol: &CStr) -> Option<*mut c_void> {
    let address = unsafe { libc::dlsym(libc::RTLD_DEFAULT, symbol.as_ptr()) };
    (!address.is_null()).then_some(address)
}

fn parse_hook_targets(contents: &str) -> Vec<HookTarget> {
    let mut targets = Vec::new();
    for line in contents.lines() {
        let mut fields = line.split_whitespace();
        let _range = fields.next();
        let Some(perms) = fields.next() else {
            continue;
        };
        // V4 delegates the final ELF scan to LSPlt, which considers private,
        // readable file mappings and ignores bracketed pseudo mappings.
        let permission_bytes = perms.as_bytes();
        if permission_bytes.first() != Some(&b'r') || permission_bytes.get(3) != Some(&b'p') {
            continue;
        }
        let _offset = fields.next();
        let Some(device_text) = fields.next() else {
            continue;
        };
        let Some(inode_text) = fields.next() else {
            continue;
        };
        let Some(path) = fields.next() else {
            continue;
        };
        if path.starts_with('[') {
            continue;
        }
        let Some(device) = parse_map_device(device_text) else {
            continue;
        };
        let Ok(inode) = inode_text.parse::<libc::ino_t>() else {
            continue;
        };
        if device == 0 || inode == 0 {
            continue;
        }
        let target = HookTarget { device, inode };
        if !targets.contains(&target) {
            targets.push(target);
        }
    }
    targets
}

fn parse_map_device(value: &str) -> Option<libc::dev_t> {
    let (major, minor) = value.split_once(':')?;
    let major = u32::from_str_radix(major, 16).ok()?;
    let minor = u32::from_str_radix(minor, 16).ok()?;
    Some(libc::makedev(major, minor))
}

fn is_soter_target(process: Option<&str>, data_dir: Option<&str>) -> bool {
    process == Some(SOTER_PROCESS)
        // Some loaders have not populated app_data_dir at pre-specialization.
        // The zygote-supplied name must still match the service process exactly.
        && match data_dir {
            None | Some("") => true,
            Some(path) => is_package_data_dir(path, SOTER_PROCESS),
        }
}

fn is_package_data_dir(path: &str, package: &str) -> bool {
    let allowed_root = path.starts_with("/data/data/")
        || path.starts_with("/data/user/")
        || path.starts_with("/data/user_de/")
        || path.starts_with("/mnt/expand/");
    allowed_root
        && path
            .strip_suffix(package)
            .is_some_and(|prefix| prefix.ends_with('/'))
}

fn read_java_string(env: &mut JNIEnv<'_>, value: &JString<'_>) -> Option<String> {
    env.get_string(value).ok().map(Into::into)
}

fn read_soter_from_companion(stream: &mut std::os::unix::net::UnixStream) -> std::io::Result<bool> {
    stream.set_read_timeout(Some(IPC_TIMEOUT))?;
    read_soter_frame(stream)
}

fn read_soter_frame(stream: &mut impl Read) -> std::io::Result<bool> {
    let mut length = [0u8; 4];
    stream.read_exact(&mut length)?;
    let length = u32::from_be_bytes(length) as usize;
    if length > MAX_PROFILE_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "companion frame exceeds the 4 KiB limit",
        ));
    }
    // The frame stays byte-for-byte compatible with the companion writer. The
    // payload is ignored here; only the appended independent flag is read.
    let mut contents = [0u8; MAX_PROFILE_BYTES];
    stream.read_exact(&mut contents[..length])?;
    let mut enabled = [0u8; 1];
    match stream.read_exact(&mut enabled) {
        Ok(()) if enabled[0] <= 1 => Ok(enabled[0] == 1),
        Ok(()) => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "invalid Soter Beta companion flag",
        )),
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => Ok(false),
        Err(error) => Err(error),
    }
}

fn companion(stream: &mut std::os::unix::net::UnixStream) {
    let soter_enabled = soter_common::read().unwrap_or_else(|error| {
        soter::log(&format!(
            "Soter Beta companion rejected configuration: {error}"
        ));
        false
    });
    // The payload is intentionally empty; the client parser accepts any length
    // up to MAX_PROFILE_BYTES and reads the enable flag that follows it.
    let payload: &[u8] = &[];
    let mut frame = Vec::with_capacity(4 + payload.len() + 1);
    frame.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    frame.extend_from_slice(payload);
    frame.push(u8::from(soter_enabled));
    if let Err(error) = stream
        .set_write_timeout(Some(IPC_TIMEOUT))
        .and_then(|()| stream.write_all(&frame))
    {
        soter::log(&format!("Soter Beta companion write failed: {error}"));
    }
}

#[link(name = "log")]
unsafe extern "C" {
    fn __android_log_write(
        priority: i32,
        tag: *const libc::c_char,
        text: *const libc::c_char,
    ) -> i32;
}

zygisk_api::register_module!(SoterSpoofModule);
zygisk_api::register_companion!(companion);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soter_accepts_own_or_unavailable_data_directory() {
        for path in [
            None,
            Some(""),
            Some("/data/user/0/com.tencent.soter.soterserver"),
            Some("/data/user_de/10/com.tencent.soter.soterserver"),
            Some("/data/data/com.tencent.soter.soterserver"),
            Some("/mnt/expand/volume/user/0/com.tencent.soter.soterserver"),
        ] {
            assert!(is_soter_target(Some(SOTER_PROCESS), path), "{path:?}");
        }
    }

    #[test]
    fn soter_requires_exact_service_process_even_without_data_directory() {
        for process in [
            None,
            Some(""),
            Some("com.tencent.soter.soterserver:remote"),
            Some("com.tencent.soter.soterserver.evil"),
            Some("com.example.app"),
        ] {
            for path in [
                None,
                Some(""),
                Some("/data/data/com.tencent.soter.soterserver"),
            ] {
                assert!(!is_soter_target(process, path), "{process:?}, {path:?}");
            }
        }
    }

    #[test]
    fn soter_rejects_mismatched_nonempty_data_directory() {
        for path in [
            "/data/local/tmp/com.tencent.soter.soterserver",
            "/data/data/other.package",
            "/data/data/com.tencent.soter.soterserver.evil",
        ] {
            assert!(!is_soter_target(Some(SOTER_PROCESS), Some(path)), "{path}");
        }
    }

    #[test]
    fn soter_companion_frame_is_independent_and_bounded() {
        for payload in [b"".as_slice(), b"not a profile".as_slice()] {
            let mut frame = (payload.len() as u32).to_be_bytes().to_vec();
            frame.extend_from_slice(payload);
            assert!(!read_soter_frame(&mut frame.as_slice()).unwrap());
            frame.push(1);
            assert!(read_soter_frame(&mut frame.as_slice()).unwrap());
            *frame.last_mut().unwrap() = 0;
            assert!(!read_soter_frame(&mut frame.as_slice()).unwrap());
            *frame.last_mut().unwrap() = 2;
            assert!(read_soter_frame(&mut frame.as_slice()).is_err());
        }
        let oversized = (MAX_PROFILE_BYTES as u32 + 1).to_be_bytes();
        assert!(read_soter_frame(&mut oversized.as_slice()).is_err());
        assert!(read_soter_frame(&mut [0, 0, 0, 2, 0].as_slice()).is_err());
    }
}
