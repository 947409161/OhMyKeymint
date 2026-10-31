use std::ffi::c_void;

use anyhow::{bail, Context};
use kmr_common::consts::{KEYSTORE_GID, KEYSTORE_UID};
use log::{error, info, warn, LevelFilter};
use nix::unistd::Pid;

use crate::android::hardware::security::keymint::SecurityLevel::SecurityLevel;

pub mod app_scope;
pub mod config;
pub mod filter;
pub mod forward;
pub mod hook;
pub mod identify;
pub mod inject;
pub mod ipc;
pub mod legacy;
pub mod logging;
pub mod parcel;
pub mod sys;
pub mod tracker;
pub mod utils;

include!(concat!(env!("OUT_DIR"), "/aidl.rs"));

fn log_runtime_identity(role: &str) {
    let uid = unsafe { libc::getuid() };
    let euid = unsafe { libc::geteuid() };
    let gid = unsafe { libc::getgid() };
    let egid = unsafe { libc::getegid() };
    info!("runtime identity role={} uid={} euid={} gid={} egid={}", role, uid, euid, gid, egid);
}

const WEBUI_GET_TEE_STATUS: &str = "--webui-get-tee-status";

fn parse_webui_tee_status_args(args: &[String]) -> Option<Result<(), String>> {
    let command = args.first()?;
    if command != WEBUI_GET_TEE_STATUS {
        return None;
    }
    if args.len() != 1 {
        return Some(Err(format!("{WEBUI_GET_TEE_STATUS} does not accept arguments")));
    }
    Some(Ok(()))
}

fn is_unsupported_webui_command(args: &[String]) -> bool {
    args.first().is_some_and(|arg| arg.starts_with("--webui-"))
}

fn enter_keystore_identity() -> anyhow::Result<()> {
    let effective_uid = unsafe { libc::geteuid() };
    let effective_gid = unsafe { libc::getegid() };
    if effective_uid == KEYSTORE_UID && effective_gid == KEYSTORE_GID {
        return Ok(());
    }
    if effective_uid != 0 {
        bail!(
            "TEE status probe requires root or keystore uid/gid; current euid={effective_uid} egid={effective_gid}"
        );
    }

    if unsafe { libc::setgid(KEYSTORE_GID) } != 0 {
        return Err(std::io::Error::last_os_error())
            .context("failed to enter keystore gid for TEE status probe");
    }
    if unsafe { libc::setuid(KEYSTORE_UID) } != 0 {
        return Err(std::io::Error::last_os_error())
            .context("failed to enter keystore uid for TEE status probe");
    }
    if unsafe { libc::geteuid() } != KEYSTORE_UID || unsafe { libc::getegid() } != KEYSTORE_GID {
        bail!("TEE status probe did not enter the keystore uid/gid");
    }
    Ok(())
}

fn probe_tee_status() -> anyhow::Result<()> {
    enter_keystore_identity()?;
    ipc::ensure_process_state();
    ipc::with_omk_once(|omk| {
        let _ = omk.r#getSecurityLevel(SecurityLevel::TRUSTED_ENVIRONMENT)?;
        Ok(())
    })
    .context("failed to obtain the OMK TEE security level")
}

fn handle_webui_tee_status_command() -> Option<Result<&'static str, String>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match parse_webui_tee_status_args(&args)? {
        Ok(()) => Some(probe_tee_status().map(|()| "normal").map_err(|error| format!("{error:#}"))),
        Err(error) => Some(Err(error)),
    }
}

fn main() {
    if std::env::args().nth(1).as_deref() == Some("--app-scope-watch") {
        // Older keymint builds delegated discovery to the injector. Discovery
        // now runs in keymint's privileged helper; never fall through to
        // injection for this legacy argument.
        eprintln!("--app-scope-watch is no longer handled by the injector");
        std::process::exit(2);
    }
    if let Some(result) = handle_webui_tee_status_command() {
        match result {
            Ok(output) => println!("{output}"),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(2);
            }
        }
        return;
    }
    // Removed commands must not fall through into the daemon/injection path
    // when an outdated WebUI still invokes them.
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if is_unsupported_webui_command(&args) {
        eprintln!("unsupported WebUI command");
        std::process::exit(2);
    }

    logging::init_logger_fallback(LevelFilter::Debug);
    let config = config::get();
    if config::parse_level_filter(&config.main.log_level).is_none() {
        warn!(
            "injector logging unknown log level '{}', keeping debug fallback",
            config.main.log_level
        );
    }
    log::set_max_level(config.main.log_level_filter());
    log_runtime_identity("Launcher");
    match utils::current_exe_identity() {
        Ok(identity) => {
            info!(
                "injector binary build_id={} build_target={} git_sha={} runtime_arch={} exe={} sha256={} elf={}",
                utils::build_id(),
                utils::build_target(),
                utils::build_git_sha(),
                std::env::consts::ARCH,
                identity.path.display(),
                identity.sha256,
                identity.elf,
            );
        }
        Err(error) => {
            error!("failed to describe current injector binary: {:#}", error);
        }
    }

    let (pid, target_path) = match utils::find_process_by_name("keystore2") {
        Ok(target) => target,
        Err(error) => {
            warn!("event=target_not_ready process=keystore2 error={error:#}; daemon will retry");
            std::process::exit(1);
        }
    };
    match utils::executable_identity(&target_path) {
        Ok(identity) => info!(
            "keystore2 target pid={} exe={} sha256={} elf={}",
            pid,
            identity.path.display(),
            identity.sha256,
            identity.elf,
        ),
        Err(error) => {
            error!("failed to describe keystore2 executable {}: {:#}", target_path.display(), error)
        }
    }
    let pid = Pid::from_raw(pid);
    match inject::inject_library(pid) {
        Ok(()) => info!("injection completed"),
        Err(e) => {
            error!("injection failed: {:#}", e);
            std::process::exit(1);
        }
    }
}

#[no_mangle]
#[allow(unused)]
pub extern "C" fn entry(handle: *const c_void) -> bool {
    // This runs inside the target process, so we must initialize logging again
    // for that process. On Android this enables both logcat and stdout logging.
    logging::init_logger_fallback(LevelFilter::Debug);
    let config = config::get();
    if config::parse_level_filter(&config.main.log_level).is_none() {
        warn!(
            "injector logging unknown log level '{}', keeping debug fallback",
            config.main.log_level
        );
    }
    log::set_max_level(config.main.log_level_filter());
    // Start the scope watcher before requests arrive so the first decision
    // already sees the persisted discovery snapshot.
    let _ = app_scope::get();
    log_runtime_identity("Payload");
    log::info!(
        "Injected library entry called! Handle: {:?}, build_id={}, build_target={}, runtime_arch={}, current_exe={}",
        handle,
        utils::build_id(),
        utils::build_target(),
        std::env::consts::ARCH,
        utils::current_exe_path()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|_| "<unknown>".to_string()),
    );
    if let Err(error) = ipc::install_direct_rpc_session() {
        error!("failed to initialize OMK RPC session: {error:#}");
        return false;
    }
    if let Err(error) = hook::init_hook() {
        error!("failed to initialize binder ioctl hook: {error:#}");
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    #[test]
    fn tee_status_command_is_dispatched_without_arguments() {
        assert_eq!(parse_webui_tee_status_args(&args(&[WEBUI_GET_TEE_STATUS])), Some(Ok(())));
    }

    #[test]
    fn tee_status_command_rejects_extra_arguments() {
        assert_eq!(
            parse_webui_tee_status_args(&args(&[WEBUI_GET_TEE_STATUS, "extra"])),
            Some(Err(format!("{WEBUI_GET_TEE_STATUS} does not accept arguments")))
        );
    }

    #[test]
    fn tee_status_parser_leaves_other_commands_for_existing_dispatch() {
        assert_eq!(parse_webui_tee_status_args(&args(&["--webui-removed-command"])), None);
        assert_eq!(parse_webui_tee_status_args(&[]), None);
    }

    #[test]
    fn removed_webui_commands_are_rejected() {
        for command in ["--webui-get-scoop", "--webui-update-scoop", "--webui-set-scoop"] {
            assert!(is_unsupported_webui_command(&args(&[command])));
        }
        assert!(!is_unsupported_webui_command(&args(&["--daemon"])));
        assert!(!is_unsupported_webui_command(&[]));
    }
}
