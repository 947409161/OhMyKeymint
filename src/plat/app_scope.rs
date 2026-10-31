//! Automatic routing scope discovery, run by the privileged keymint helper.
//!
//! The helper is root, so it can query PackageManager. The injector payload
//! runs inside keystore2 as the keystore uid and only reads the published
//! snapshot.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Read};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use kmr_common::app_scope::{
    encode_scope_file, global_packages, is_valid_package_name, read_scope_file, ScopeFile,
    CURRENT_SCOPE_VERSION, MAX_BYTES, MAX_USERS, SCOPE_PATH,
};
use kmr_common::consts::{AID_USER_OFFSET, KEYSTORE_GID, KEYSTORE_UID};
use kmr_common::runtime::{file_watch, fs::atomic_replace_preserving_metadata};

const PACKAGE_LIST_PATH: &str = "/data/system/packages.list";
const RETRY_INTERVAL: Duration = Duration::from_secs(10);
const COMMAND_TIMEOUT: Duration = Duration::from_secs(12);

#[derive(Default)]
struct Discovery {
    users: BTreeMap<u32, Option<BTreeSet<String>>>,
}

/// Start one discovery thread inside the long-lived root helper.
pub(crate) fn start() {
    if let Err(error) = std::thread::Builder::new().name("omk-app-scope".into()).spawn(run) {
        log::warn!("cannot start app scope discovery: {error}");
    }
}

fn run() {
    if unsafe { libc::geteuid() } != 0 {
        log::warn!("app scope discovery requires root; keeping the last scope");
        return;
    }
    let (sender, receiver) = mpsc::sync_channel(1);
    if let Err(error) = file_watch::spawn_path_watcher(
        "omk-app-scope-events",
        PathBuf::from(PACKAGE_LIST_PATH),
        move |_| {
            let _ = sender.try_send(());
        },
    ) {
        log::warn!("app scope event watcher unavailable: {error:#}");
    }

    let mut first_success = false;
    let mut last_error = None;
    loop {
        match poll() {
            Ok(()) => {
                first_success = true;
                last_error = None;
            }
            Err(error) => {
                // PackageManager can be unavailable during boot. Avoid filling
                // the daemon log with the same retry failure repeatedly.
                let message = format!("{error:#}");
                if last_error.as_ref() != Some(&message) {
                    log::warn!("app scope discovery: {message}");
                    last_error = Some(message);
                }
            }
        }
        if first_success && last_error.is_none() {
            // The startup snapshot is the baseline. Afterwards only an
            // installed-package change triggers another scan.
            if receiver.recv().is_err() {
                // The event thread is gone; retry slowly instead of spinning.
                std::thread::sleep(RETRY_INTERVAL);
            }
        } else {
            // PackageManager can be unavailable during boot or transiently
            // busy; retry until a scan succeeds without polling periodically.
            let _ = receiver.recv_timeout(RETRY_INTERVAL);
        }
        std::thread::sleep(Duration::from_millis(500));
        while receiver.try_recv().is_ok() {}
    }
}

fn parse_users(output: &str) -> Result<BTreeSet<u32>> {
    let mut users = BTreeSet::new();
    for tail in output.split("UserInfo{").skip(1) {
        let id = tail.split(':').next().unwrap_or("");
        if id.is_empty() || !id.bytes().all(|byte| byte.is_ascii_digit()) {
            bail!("invalid Android user list");
        }
        let user: u32 = id.parse()?;
        if user > u32::MAX / AID_USER_OFFSET {
            bail!("Android user is out of range");
        }
        users.insert(user);
    }
    if users.is_empty() || users.len() > MAX_USERS {
        bail!("Android user list is unavailable");
    }
    Ok(users)
}

fn parse_packages(output: &str) -> Result<BTreeSet<String>> {
    output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let name = line
                .trim()
                .strip_prefix("package:")
                .ok_or_else(|| anyhow!("unexpected PackageManager output"))?;
            if !is_valid_package_name(name) {
                bail!("invalid installed package name");
            }
            Ok(name.to_owned())
        })
        .collect()
}

fn discover() -> Result<Discovery> {
    let output = run_command(&["user", "list"])?;
    let users = parse_users(&output)?;
    let mut discovery = Discovery::default();
    for user in users {
        let output =
            run_command(&["package", "list", "packages", "-3", "--user", &user.to_string()]);
        let packages = output.and_then(|output| parse_packages(&output)).ok();
        discovery.users.insert(user, packages);
    }
    if discovery.users.values().all(Option::is_none) {
        bail!("no Android user package snapshot is available");
    }
    Ok(discovery)
}

/// Merge a discovery round into the previous snapshot. Users that disappeared
/// are dropped; a user whose query failed keeps its previous packages so a
/// transient PackageManager failure cannot remove routing for that profile.
fn merge(previous: Option<&ScopeFile>, discovery: Discovery) -> Option<ScopeFile> {
    let mut users = BTreeMap::new();
    let mut any_success = false;
    for (user, packages) in discovery.users {
        match packages {
            Some(packages) => {
                any_success = true;
                users.insert(user, packages);
            }
            None => {
                if let Some(packages) = previous.and_then(|scope| scope.users.get(&user)) {
                    users.insert(user, packages.clone());
                }
            }
        }
    }
    if !any_success {
        return None;
    }
    Some(ScopeFile { version: CURRENT_SCOPE_VERSION, global: global_packages(), users })
}

fn poll() -> Result<()> {
    let path = Path::new(SCOPE_PATH);
    let previous = read_scope_file(path).ok().flatten();
    let discovery = discover()?;
    let Some(next) = merge(previous.as_ref(), discovery) else {
        bail!("no Android user package snapshot is available");
    };
    if previous.as_ref() == Some(&next) {
        return Ok(());
    }
    let encoded = encode_scope_file(&next)?;
    atomic_replace_preserving_metadata(path, &encoded, 0o600, KEYSTORE_UID, KEYSTORE_GID)
        .context("cannot save app scope")
}

/// Bounded argv-only calls into Android's installed package manager, never a
/// shell program or a per-app Binder transaction. No probe files are created.
fn run_command(args: &[&str]) -> Result<String> {
    let mut child = Command::new("/system/bin/cmd")
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let result = (|| {
        let mut stdout = child.stdout.take().context("missing package manager stdout")?;
        let fd = stdout.as_raw_fd();
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
            return Err(io::Error::last_os_error()).context("cannot read package manager output");
        }
        let started = Instant::now();
        let mut output = Vec::new();
        let mut buffer = [0u8; 8192];
        loop {
            loop {
                match stdout.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(size) => {
                        output.extend_from_slice(&buffer[..size]);
                        if output.len() > MAX_BYTES {
                            bail!("package manager output is too large");
                        }
                    }
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                    Err(error) => return Err(error.into()),
                }
            }
            if let Some(status) = child.try_wait()? {
                if !status.success() {
                    bail!("package manager query failed: {status}");
                }
                // Drain the final bytes emitted between the last read and exit.
                stdout.take((MAX_BYTES - output.len() + 1) as u64).read_to_end(&mut output)?;
                if output.len() > MAX_BYTES {
                    bail!("package manager output is too large");
                }
                return String::from_utf8(output).context("package manager output is not UTF-8");
            }
            if started.elapsed() >= COMMAND_TIMEOUT {
                bail!("package manager query timed out");
            }
            std::thread::sleep(Duration::from_millis(40));
        }
    })();
    if result.is_err() {
        let _ = child.kill();
    }
    let _ = child.wait();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packages(names: &[&str]) -> Option<BTreeSet<String>> {
        Some(names.iter().map(|name| name.to_string()).collect())
    }

    #[test]
    fn merge_keeps_failed_profiles_and_drops_removed_users() {
        let previous = ScopeFile {
            version: CURRENT_SCOPE_VERSION,
            global: BTreeSet::new(),
            users: BTreeMap::from([
                (0, packages(&["com.old"]).unwrap()),
                (10, packages(&["com.profile"]).unwrap()),
            ]),
        };
        let merged = merge(
            Some(&previous),
            Discovery {
                users: BTreeMap::from([
                    (0, packages(&["com.old", "com.new"])),
                    (10, None),
                    (11, packages(&["com.late"])),
                ]),
            },
        )
        .unwrap();
        assert_eq!(merged.users[&0], packages(&["com.old", "com.new"]).unwrap());
        assert_eq!(merged.users[&10], previous.users[&10]);
        assert_eq!(merged.users[&11], packages(&["com.late"]).unwrap());
        assert!(merged.global.contains("com.google.android.gms"));

        let removed = merge(
            Some(&previous),
            Discovery { users: BTreeMap::from([(0, packages(&["com.old"]))]) },
        )
        .unwrap();
        assert!(!removed.users.contains_key(&10));
    }

    #[test]
    fn merge_without_any_success_keeps_the_previous_file() {
        assert!(merge(None, Discovery { users: BTreeMap::from([(0, None)]) },).is_none());
    }

    #[test]
    fn package_manager_outputs_are_validated() {
        assert_eq!(
            parse_users("Users:\n UserInfo{0:Owner:c13} running\n UserInfo{10:Work:30}").unwrap(),
            BTreeSet::from([0, 10])
        );
        assert!(parse_users("Error: service unavailable").is_err());
        assert!(parse_packages("Error: permission denied").is_err());
        assert!(parse_packages("package:com.app@0").is_err());
        assert_eq!(parse_packages("package:com.app\npackage:com.app\n").unwrap().len(), 1);
        assert!(parse_packages("").unwrap().is_empty());
    }
}
