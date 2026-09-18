//! Soter spoof payload toggle shared by the WebUI command and module boot script.
//!
//! The payload is a vendored Zygisk module (see `third_party/d-soter`) that has
//! no configuration of its own and cannot be switched off at runtime: once the
//! loader maps it, it hooks the Soter service unconditionally.  Switching it
//! off therefore means stopping the loader from finding it, which only the root
//! boot script can do, and which only takes effect on the next boot.
//!
//! This module records the requested state and reports whether the payload on
//! disk still disagrees with it.  Like `adb_disabler`, the value is a single
//! line so the boot script can read it without a second parser.

use std::{fs, os::unix::fs::PermissionsExt, path::Path};

use anyhow::{anyhow, bail, Context, Result};
use serde::Serialize;

const STATE_PATH: &str = "/data/misc/keystore/omk/data/soter_spoof.conf";

/// The Zygisk payload directory inside the installed module.  The loader
/// addresses payloads as `zygisk/<abi>.so`, so `PAYLOAD_NAME` is the name it
/// looks for.  `template/post-fs-data.sh` hides the payload by appending
/// `.disabled`, which does not end in `.so`; that name is deliberately not
/// repeated here because this side only needs to know whether the payload is
/// currently loadable.
const MODULE_ZYGISK_DIR: &str = "/data/adb/modules/oh_my_keymint/zygisk";
const PAYLOAD_NAME: &str = "arm64-v8a.so";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Settings {
    pub enabled: bool,
}

impl Settings {
    pub fn from_tokens(tokens: &[String]) -> Result<Self> {
        if tokens.len() != 1 {
            bail!("Soter spoof requires exactly one value: 0 or 1");
        }
        match tokens[0].as_str() {
            "0" => Ok(Self { enabled: false }),
            "1" => Ok(Self { enabled: true }),
            _ => Err(anyhow!("Soter spoof values must be 0 or 1")),
        }
    }
}

pub fn state_path() -> &'static str {
    STATE_PATH
}

/// Reads the stored setting.  A missing or malformed file means the payload
/// stays enabled, which is the state a fresh install ships in; a device whose
/// state file was truncated by an unclean shutdown therefore behaves like one
/// that never touched the switch.
pub fn read() -> Settings {
    let Ok(contents) = fs::read_to_string(STATE_PATH) else {
        return Settings { enabled: true };
    };
    let values = contents.lines().map(str::to_string).collect::<Vec<_>>();
    Settings::from_tokens(&values).unwrap_or(Settings { enabled: true })
}

pub fn persist(settings: Settings) -> Result<()> {
    let path = Path::new(STATE_PATH);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("failed to create OMK data directory")?;
    }
    let contents = format!("{}\n", u8::from(settings.enabled));
    let temporary = path.with_extension("conf.tmp");
    fs::write(&temporary, contents).context("failed to write Soter spoof setting")?;
    fs::set_permissions(&temporary, fs::Permissions::from_mode(0o600)).ok();
    fs::rename(&temporary, path).context("failed to install Soter spoof setting")?;
    Ok(())
}

/// Whether the packaged payload currently sits under its loadable name.
///
/// `None` means the module directory could not be inspected, which is reported
/// as "no reboot pending" rather than as a spurious one.  The daemon runs as
/// keystore and may only read this directory; the rename itself belongs to the
/// root boot script.
fn payload_present() -> Option<bool> {
    let directory = Path::new(MODULE_ZYGISK_DIR);
    if !directory.is_dir() {
        return None;
    }
    Some(directory.join(PAYLOAD_NAME).is_file())
}

#[derive(Serialize)]
struct StateJson {
    enabled: bool,
    /// True while the payload on disk disagrees with the stored setting, i.e.
    /// until the next boot lets `post-fs-data.sh` act on it.  Derived on every
    /// read rather than stored, so it clears itself once the rename happens.
    reboot_required: bool,
}

fn state_json(settings: Settings) -> Result<String> {
    let reboot_required = payload_present().is_some_and(|present| present != settings.enabled);
    serde_json::to_string(&StateJson {
        enabled: settings.enabled,
        reboot_required,
    })
    .context("failed to serialize the Soter spoof state")
}

pub fn state() -> Result<String> {
    state_json(read())
}

pub fn apply(settings: Settings) -> Result<String> {
    persist(settings)?;
    state_json(settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_the_two_documented_values() {
        let parse = |value: &str| Settings::from_tokens(&[value.to_string()]);
        assert_eq!(parse("0").unwrap(), Settings { enabled: false });
        assert_eq!(parse("1").unwrap(), Settings { enabled: true });
        assert!(parse("").is_err());
        assert!(parse("true").is_err());
        assert!(parse("2").is_err());
    }

    #[test]
    fn rejects_a_missing_or_extra_value() {
        assert!(Settings::from_tokens(&[]).is_err());
        assert!(Settings::from_tokens(&["1".to_string(), "0".to_string()]).is_err());
    }

    #[test]
    fn serializes_both_boolean_fields() {
        let json = state_json(Settings { enabled: false }).unwrap();
        assert!(json.contains("\"enabled\":false"));
        assert!(json.contains("\"reboot_required\":"));
    }
}
