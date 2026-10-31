//! Payload-side reader for the routing scope published by the keymint helper.
//!
//! The injector runs inside keystore2 as the keystore uid and cannot query
//! PackageManager. The privileged keymint helper owns discovery; this module
//! only loads and watches the published snapshot.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock, RwLock};

use kmr_common::app_scope::{AppScope, read_scope_file, SCOPE_PATH};
use kmr_common::runtime::file_watch;

fn scope_path() -> PathBuf {
    std::env::var_os("OMK_APP_SCOPE_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(SCOPE_PATH))
}

static SCOPE: OnceLock<RwLock<Arc<AppScope>>> = OnceLock::new();
static WATCHER_STARTED: OnceLock<()> = OnceLock::new();

pub(crate) fn get() -> Arc<AppScope> {
    if SCOPE.get().is_none() || WATCHER_STARTED.get().is_none() {
        ensure_initialized();
    }
    Arc::clone(
        &SCOPE
            .get()
            .expect("app scope should be initialized")
            .read()
            .expect("app scope lock poisoned"),
    )
}

fn ensure_initialized() {
    let path = scope_path();
    SCOPE.get_or_init(|| RwLock::new(Arc::new(load_initial(&path))));
    WATCHER_STARTED.get_or_init(|| start_watcher(path));
}

fn load_initial(path: &Path) -> AppScope {
    match read_scope_file(path) {
        Ok(Some(file)) => AppScope::from_file(file),
        Ok(None) => {
            log::warn!(
                "app scope {} is missing; apps stay on System until discovery completes",
                path.display()
            );
            AppScope::default()
        }
        Err(error) => {
            log::warn!(
                "app scope {} is invalid: {error:#}; apps stay on System until it is replaced",
                path.display()
            );
            AppScope::default()
        }
    }
}

fn start_watcher(path: PathBuf) {
    let reload_path = path.clone();
    if let Err(error) = file_watch::spawn_path_watcher("omk-app-scope-watch", path, move |_| {
        reload_scope(&reload_path);
    }) {
        log::error!("failed to start app scope watcher thread: {error}");
    }
}

fn reload_scope(path: &Path) {
    let next = match read_scope_file(path) {
        Ok(Some(file)) => AppScope::from_file(file),
        Ok(None) => {
            log::warn!(
                "app scope {} disappeared; keeping the last valid scope",
                path.display()
            );
            return;
        }
        Err(error) => {
            log::warn!(
                "ignoring invalid app scope {}: {error:#}; keeping the last valid scope",
                path.display()
            );
            return;
        }
    };
    if let Some(lock) = SCOPE.get() {
        match lock.write() {
            Ok(mut guard) => *guard = Arc::new(next),
            Err(error) => log::error!("failed to apply app scope reload: {error}"),
        }
    }
}
