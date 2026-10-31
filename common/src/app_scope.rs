//! Automatic routing scope shared by the keymint helper and the injector.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read;
use std::path::Path;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

pub const SCOPE_PATH: &str = "/data/misc/keystore/omk/data/app_scope.json";
pub const CURRENT_SCOPE_VERSION: u32 = 1;
pub const GLOBAL_PACKAGES: [&str; 3] =
    ["com.android.vending", "com.google.android.gms", "com.google.android.gsf"];
pub const MAX_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_USERS: usize = 128;

pub fn is_valid_package_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 255
        && value.split('.').all(|segment| {
            !segment.is_empty()
                && segment.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        })
}

pub fn global_packages() -> BTreeSet<String> {
    GLOBAL_PACKAGES.iter().map(|name| name.to_string()).collect()
}

/// On-disk snapshot shared between the keymint helper and the injector.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ScopeFile {
    pub version: u32,
    #[serde(default)]
    pub global: BTreeSet<String>,
    #[serde(default)]
    pub users: BTreeMap<u32, BTreeSet<String>>,
}

impl ScopeFile {
    pub fn validate(&self) -> Result<()> {
        if self.version != CURRENT_SCOPE_VERSION {
            bail!("unsupported app scope version {}", self.version);
        }
        if self.users.len() > MAX_USERS {
            bail!("app scope has too many Android users");
        }
        for (user, packages) in &self.users {
            if *user > u32::MAX / crate::consts::AID_USER_OFFSET {
                bail!("invalid Android user in app scope");
            }
            for package in packages {
                if !is_valid_package_name(package) {
                    bail!("invalid package in app scope");
                }
            }
        }
        for package in &self.global {
            if !is_valid_package_name(package) {
                bail!("invalid package in app scope");
            }
        }
        Ok(())
    }
}

/// Routing scope used at decision time. An unreadable snapshot produces an
/// empty scope, which routes every resolved caller to System.
#[derive(Debug, Clone, Default)]
pub struct AppScope {
    global: BTreeSet<String>,
    users: BTreeMap<u32, BTreeSet<String>>,
}

impl AppScope {
    pub fn from_file(file: ScopeFile) -> Self {
        Self { global: file.global, users: file.users }
    }

    pub fn from_parts(global: BTreeSet<String>, users: BTreeMap<u32, BTreeSet<String>>) -> Self {
        Self { global, users }
    }

    pub fn contains(&self, user: u32, package: &str) -> bool {
        self.global.contains(package)
            || self.users.get(&user).is_some_and(|packages| packages.contains(package))
    }
}

pub fn read_scope_file(path: &Path) -> Result<Option<ScopeFile>> {
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).context("cannot read app scope"),
    };
    let mut bytes = Vec::new();
    file.take(MAX_BYTES as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > MAX_BYTES {
        bail!("app scope is too large");
    }
    let scope: ScopeFile = serde_json::from_slice(&bytes).context("invalid app scope")?;
    scope.validate()?;
    Ok(Some(scope))
}

pub fn encode_scope_file(scope: &ScopeFile) -> Result<Vec<u8>> {
    let encoded = serde_json::to_vec(scope)?;
    if encoded.len() > MAX_BYTES {
        bail!("app scope is too large");
    }
    Ok(encoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packages(names: &[&str]) -> Option<BTreeSet<String>> {
        Some(names.iter().map(|name| name.to_string()).collect())
    }

    fn scope(users: BTreeMap<u32, BTreeSet<String>>) -> AppScope {
        AppScope::from_parts(global_packages(), users)
    }

    #[test]
    fn scope_matches_global_and_per_user_packages() {
        let scope = scope(BTreeMap::from([(0, packages(&["com.app"]).unwrap())]));
        assert!(scope.contains(0, "com.app"));
        assert!(!scope.contains(10, "com.app"));
        assert!(scope.contains(10, "com.google.android.gms"));
        assert!(!scope.contains(0, "com.other"));
    }

    #[test]
    fn missing_scope_routes_nothing() {
        let scope = AppScope::default();
        assert!(!scope.contains(0, "com.app"));
        assert!(!scope.contains(0, "com.google.android.gms"));
    }

    #[test]
    fn scope_file_is_validated() {
        let path = std::env::temp_dir().join(format!("omk-app-scope-test-{}", std::process::id()));
        let write = |contents: &str| {
            std::fs::write(&path, contents).unwrap();
            read_scope_file(&path)
        };

        let valid = concat!(
            r#"{"version":1,"global":["com.google.android.gms"],"#,
            r#""users":{"0":["com.app"]}}"#
        );
        assert!(write(valid).unwrap().is_some());
        assert!(write(r#"{"version":2,"users":{}}"#).is_err());
        assert!(write(r#"{"version":1,"extra":true}"#).is_err());
        assert!(write(r#"{"version":1,"users":{"0":["bad package"]}}"#).is_err());
        assert!(write(r#"{"version":1,"users":{"42950":["com.app"]}}"#).is_err());
        let long_name = format!(r#"{{"version":1,"users":{{"0":["{}"]}}}}"#, "a".repeat(256));
        assert!(write(&long_name).is_err());
        std::fs::remove_file(&path).unwrap();
        assert!(read_scope_file(&path).unwrap().is_none());
    }

    #[test]
    fn scope_file_round_trips() {
        let file = ScopeFile {
            version: CURRENT_SCOPE_VERSION,
            global: global_packages(),
            users: BTreeMap::from([(0, packages(&["com.app"]).unwrap())]),
        };
        let encoded = encode_scope_file(&file).unwrap();
        let decoded: ScopeFile = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, file);
        decoded.validate().unwrap();
    }
}
