use kmr_common::consts::{KEYSTORE_GID, KEYSTORE_UID};
use kmr_common::runtime::{
    file_watch::{self, WatchTrigger},
    fs::atomic_replace_preserving_metadata,
    retry::{retry_read_race, ReadRaceErrorKind, RetryOutcome},
};
use log::LevelFilter;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use std::time::Duration;

pub const DEFAULT_CONFIG_PATH: &str = "/data/misc/keystore/omk/injector.toml";
const CURRENT_CONFIG_VERSION: u32 = 1;
const REPLACE_SAVE_RETRY_INTERVAL: Duration = Duration::from_millis(100);
const REPLACE_SAVE_RETRY_LIMIT: usize = 10;

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct InjectorConfig {
    pub version: u32,
    pub main: MainConfig,
    pub filter: FilterConfig,
    pub intercept: InterceptConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct MainConfig {
    pub enabled: bool,
    pub log_level: String,
    /// Inclusive extra delay range in milliseconds for challenged key generation.
    ///
    /// A challenged generation has to cost measurably more than a plain one: a real security level
    /// pays an extra round trip for the challenge, and that extra cost varies rather than staying
    /// constant. Each affected generation samples the range afresh, so the delay itself is not a
    /// fixed signature. `[0, 0]` disables it.
    #[serde(deserialize_with = "deserialize_delay_range")]
    pub attestation_generation_delay_ms: [u16; 2],
    /// Optional delay before a two-way OMK createOperation call, in milliseconds.
    pub operation_start_delay_ms: u16,
}

fn deserialize_delay_range<'de, D>(deserializer: D) -> Result<[u16; 2], D::Error>
where
    D: serde::Deserializer<'de>,
{
    Vec::<u16>::deserialize(deserializer)?.try_into().map_err(|_| {
        serde::de::Error::custom(
            "main.attestation_generation_delay_ms must contain exactly two integers",
        )
    })
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct FilterConfig {
    pub enabled: bool,
    pub block_android_package: bool,
    pub allow_unknown_package: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct InterceptConfig {
    pub get_security_level: bool,
    pub get_key_entry: bool,
    pub update_subcomponent: bool,
    pub list_entries: bool,
    pub delete_key: bool,
    pub grant: bool,
    pub ungrant: bool,
    pub get_number_of_entries: bool,
    pub list_entries_batched: bool,
    pub get_supplementary_attestation_info: bool,
}

impl Default for InjectorConfig {
    fn default() -> Self {
        Self {
            version: CURRENT_CONFIG_VERSION,
            main: MainConfig::default(),
            filter: FilterConfig::default(),
            intercept: InterceptConfig::default(),
        }
    }
}

impl Default for MainConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            log_level: "debug".to_string(),
            attestation_generation_delay_ms: [20, 30],
            operation_start_delay_ms: 0,
        }
    }
}

impl Default for FilterConfig {
    fn default() -> Self {
        Self { enabled: true, block_android_package: true, allow_unknown_package: false }
    }
}

impl Default for InterceptConfig {
    fn default() -> Self {
        Self {
            get_security_level: true,
            get_key_entry: true,
            update_subcomponent: true,
            list_entries: true,
            delete_key: true,
            grant: true,
            ungrant: true,
            get_number_of_entries: true,
            list_entries_batched: true,
            get_supplementary_attestation_info: true,
        }
    }
}

#[derive(Debug)]
enum LoadError {
    Missing(io::Error),
    Io(io::Error),
    Parse(String),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(error) | Self::Io(error) => write!(f, "{error}"),
            Self::Parse(error) => write!(f, "{error}"),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum LoadContext {
    Startup,
    Reload(WatchTrigger),
}

#[derive(Deserialize)]
struct ConfigVersion {
    version: Option<toml::Spanned<i64>>,
}

#[derive(Serialize)]
struct WritableConfig<'a> {
    main: &'a MainConfig,
    filter: &'a FilterConfig,
    intercept: &'a InterceptConfig,
}

static CONFIG: OnceLock<RwLock<Arc<InjectorConfig>>> = OnceLock::new();
static WATCHER_STARTED: OnceLock<()> = OnceLock::new();
static CONFIG_FILE_WRITE_LOCK: Mutex<()> = Mutex::new(());

/// A stable directory inode serializes atomic config replacements without a
/// lock file.
pub(crate) struct DirectoryLock(fs::File);

impl DirectoryLock {
    pub(crate) fn acquire(directory: &Path) -> io::Result<Self> {
        let file = fs::File::open(directory)?;
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self(file))
    }
}

impl Drop for DirectoryLock {
    fn drop(&mut self) {
        let _ = unsafe { libc::flock(self.0.as_raw_fd(), libc::LOCK_UN) };
    }
}

fn lock_config_directory(path: &Path) -> io::Result<DirectoryLock> {
    let parent = path.parent().ok_or_else(|| io::Error::other("config has no parent"))?;
    fs::create_dir_all(parent)?;
    DirectoryLock::acquire(parent)
}

impl LoadContext {
    fn label(self) -> &'static str {
        match self {
            Self::Startup => "startup",
            Self::Reload(trigger) => trigger.label(),
        }
    }
}

pub fn get() -> Arc<InjectorConfig> {
    if CONFIG.get().is_none() || WATCHER_STARTED.get().is_none() {
        ensure_initialized();
    }
    Arc::clone(
        &CONFIG
            .get()
            .expect("injector config should be initialized")
            .read()
            .expect("injector config lock poisoned"),
    )
}

fn ensure_initialized() {
    let path = config_path();
    CONFIG.get_or_init(|| {
        RwLock::new(Arc::new(
            load_or_seed(&path, LoadContext::Startup)
                .expect("startup config loading always returns a fallback"),
        ))
    });
    WATCHER_STARTED.get_or_init(|| start_watcher(path));
}

fn config_path() -> PathBuf {
    std::env::var_os("OMK_INJECTOR_CONFIG_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_PATH))
}

fn load_from_path(path: &Path, allow_migration: bool) -> Result<InjectorConfig, LoadError> {
    let _write_guard = CONFIG_FILE_WRITE_LOCK
        .lock()
        .map_err(|_| LoadError::Io(io::Error::other("config file write lock poisoned")))?;
    let _directory_guard = lock_config_directory(path).map_err(LoadError::Io)?;
    let contents = fs::read_to_string(path).map_err(|error| {
        if error.kind() == io::ErrorKind::NotFound {
            LoadError::Missing(error)
        } else {
            LoadError::Io(error)
        }
    })?;
    let (config, migrated_contents) =
        parse_versioned_config(&contents, allow_migration).map_err(LoadError::Parse)?;
    if let Some(migrated_contents) = migrated_contents {
        let (default_uid, default_gid) = default_owner(path);
        atomic_replace_preserving_metadata(
            path,
            migrated_contents.as_bytes(),
            0o600,
            default_uid,
            default_gid,
        )
        .map_err(LoadError::Io)?;
        log::info!("migrated injector.toml to version {CURRENT_CONFIG_VERSION}");
    }
    Ok(config)
}

fn load_with_context(
    path: &Path,
    context: LoadContext,
) -> Result<RetryOutcome<InjectorConfig>, LoadError> {
    match context {
        LoadContext::Reload(trigger) if trigger.should_retry_reads() => load_with_read_race_retry(
            path,
            context,
            |path| load_from_path(path, false),
            std::thread::sleep,
        ),
        LoadContext::Startup => {
            load_from_path(path, true).map(|value| RetryOutcome { value, retries: 0 })
        }
        LoadContext::Reload(_) => {
            load_from_path(path, false).map(|value| RetryOutcome { value, retries: 0 })
        }
    }
}

fn load_or_seed(path: &Path, context: LoadContext) -> Option<InjectorConfig> {
    match load_with_context(path, context) {
        Ok(loaded) => {
            if loaded.retries > 0 {
                log::info!(
                    "{} config load from {} succeeded after {} retr{}",
                    context.label(),
                    path.display(),
                    loaded.retries,
                    if loaded.retries == 1 { "y" } else { "ies" }
                );
            }
            if matches!(context, LoadContext::Startup) {
                log::info!("loaded config from {} via {}", path.display(), context.label());
            }
            Some(loaded.value)
        }
        Err(LoadError::Missing(error)) if matches!(context, LoadContext::Startup) => {
            log::warn!(
                "config missing at {} during startup: {}; seeding defaults",
                path.display(),
                error
            );
            let mut config = InjectorConfig::default();
            if let Err(write_error) = write_config(path, &config) {
                log::error!(
                    "failed to seed config at {}: {}; disabling injector",
                    path.display(),
                    write_error
                );
                config.main.enabled = false;
            }
            Some(config)
        }
        Err(error) => {
            log::warn!(
                "load from {} via {} failed: {}; keeping current config",
                path.display(),
                context.label(),
                error
            );
            if matches!(context, LoadContext::Startup) {
                log_removed_field_hint(path);
                let mut config = current_config_snapshot();
                config.main.enabled = false;
                Some(config)
            } else {
                None
            }
        }
    }
}

/// Routing lists were removed from the schema. Point at the concrete
/// remediation instead of leaving startup disabled with only a parse error.
fn log_removed_field_hint(path: &Path) {
    let Ok(contents) = fs::read_to_string(path) else {
        return;
    };
    if ["scoop", "deny_packages"].iter().any(|marker| contents.contains(marker)) {
        log::error!(
            "{} contains routing-list settings that this build no longer supports; \
             remove scoop, scoop_details, [scoop.*], and deny_packages, or delete the file and restart OMK",
            path.display()
        );
    }
}

fn current_config_snapshot() -> InjectorConfig {
    match CONFIG.get() {
        Some(lock) => match lock.read() {
            Ok(config) => config.as_ref().clone(),
            Err(error) => {
                log::error!("current config lock poisoned while snapshotting: {}", error);
                InjectorConfig::default()
            }
        },
        None => InjectorConfig::default(),
    }
}

fn write_config(path: &Path, config: &InjectorConfig) -> io::Result<()> {
    let _write_guard = CONFIG_FILE_WRITE_LOCK
        .lock()
        .map_err(|_| io::Error::other("config file write lock poisoned"))?;
    let _directory_guard = lock_config_directory(path)?;
    let contents = render_config(config)?;
    write_config_contents(path, &contents)
}

fn write_config_contents(path: &Path, contents: &str) -> io::Result<()> {
    let (default_uid, default_gid) = default_owner(path);
    atomic_replace_preserving_metadata(path, contents.as_bytes(), 0o600, default_uid, default_gid)?;
    log::info!("wrote config to {}", path.display());
    Ok(())
}

fn default_owner(path: &Path) -> (u32, u32) {
    if path == Path::new(DEFAULT_CONFIG_PATH) {
        (KEYSTORE_UID, KEYSTORE_GID)
    } else {
        (unsafe { libc::geteuid() }, unsafe { libc::getegid() })
    }
}

fn render_config(config: &InjectorConfig) -> io::Result<String> {
    let mut contents = String::from(
        "# With `[filter].enabled = true`, OMK routes installed third-party apps\n\
         # and the three Google packages discovered by the module helper.\n\
         # The routing scope is refreshed automatically; it is not configured here.\n\n",
    );
    contents.push_str(&format!("version = {}\n\n", config.version));

    let base = toml::to_string_pretty(&WritableConfig {
        main: &config.main,
        filter: &config.filter,
        intercept: &config.intercept,
    })
    .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    contents.push_str(&base);

    Ok(contents)
}

#[cfg(test)]
fn parse_config(contents: &str) -> Result<InjectorConfig, String> {
    parse_versioned_config(contents, true).map(|(config, _)| config)
}

fn parse_versioned_config(
    contents: &str,
    allow_migration: bool,
) -> Result<(InjectorConfig, Option<String>), String> {
    let without_bom = contents.strip_prefix('\u{feff}').unwrap_or(contents);
    let bom_len = contents.len() - without_bom.len();
    let version: ConfigVersion = toml::from_str(without_bom).map_err(|error| error.to_string())?;
    let migrated = match version.version {
        None => {
            if !allow_migration {
                return Err(
                    "injector config version 0 requires an injector restart to migrate".into()
                );
            }
            Some(insert_config_version(contents, bom_len))
        }
        Some(version) => match *version.get_ref() {
            0 if allow_migration => Some(replace_config_version(contents, bom_len)?),
            0 => {
                return Err(
                    "injector config version 0 requires an injector restart to migrate".into()
                )
            }
            version if version == i64::from(CURRENT_CONFIG_VERSION) => None,
            version if version < 0 => {
                return Err(format!("config version must not be negative: {version}"))
            }
            version => {
                return Err(format!(
                "config version {version} is newer than supported version {CURRENT_CONFIG_VERSION}"
            ))
            }
        },
    };
    let candidate = migrated.as_deref().unwrap_or(contents);
    let candidate = candidate.strip_prefix('\u{feff}').unwrap_or(candidate);
    let parsed: InjectorConfig = toml::from_str(candidate).map_err(|error| error.to_string())?;
    let attestation_delay = parsed.main.attestation_generation_delay_ms;
    if attestation_delay[0] > attestation_delay[1] {
        return Err(
            "main.attestation_generation_delay_ms must be an ordered [minimum, maximum]".into()
        );
    }
    if attestation_delay[1] > 250 {
        return Err("main.attestation_generation_delay_ms must be in 0..=250".into());
    }
    if parsed.main.operation_start_delay_ms > 250 {
        return Err("main.operation_start_delay_ms must be in 0..=250".into());
    }
    Ok((parsed, migrated))
}

fn insert_config_version(contents: &str, bom_len: usize) -> String {
    let newline = if contents.contains("\r\n") { "\r\n" } else { "\n" };
    let mut migrated = String::with_capacity(contents.len() + 12);
    migrated.push_str(&contents[..bom_len]);
    migrated.push_str("version = 1");
    migrated.push_str(newline);
    migrated.push_str(&contents[bom_len..]);
    migrated
}

fn replace_config_version(contents: &str, bom_len: usize) -> Result<String, String> {
    let without_bom = &contents[bom_len..];
    let mut line_offset = 0;

    for line in without_bom.split_inclusive('\n') {
        let line_body = line.strip_suffix('\n').unwrap_or(line);
        let line_body = line_body.strip_suffix('\r').unwrap_or(line_body);
        let trimmed = line_body.trim_start();
        let Some(after_name) = version_key_suffix(trimmed) else {
            line_offset += line.len();
            continue;
        };
        let Some(after_equals) = after_name.trim_start().strip_prefix('=') else {
            line_offset += line.len();
            continue;
        };
        let value = after_equals.trim_start();
        let token_end = value
            .find(|character: char| character.is_whitespace() || character == '#')
            .unwrap_or(value.len());
        if token_end == 0 {
            line_offset += line.len();
            continue;
        }

        let value_offset = line_body.len() - value.len();
        let start = bom_len + line_offset + value_offset;
        let end = start + token_end;
        let mut migrated = contents.to_string();
        migrated.replace_range(start..end, "1");
        return Ok(migrated);
    }

    Err("config version 0 was parsed but its source value could not be located".to_string())
}

fn version_key_suffix(line: &str) -> Option<&str> {
    if let Some(after_name) = line.strip_prefix("version") {
        if after_name.chars().next().is_some_and(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-')
        }) {
            return None;
        }
        return Some(after_name);
    }

    let quote = line.chars().next().filter(|character| *character == '"' || *character == '\'')?;
    let close = if quote == '"' {
        let mut escaped = false;
        line.char_indices().skip(1).find_map(|(index, character)| {
            if escaped {
                escaped = false;
                None
            } else if character == '\\' {
                escaped = true;
                None
            } else if character == quote {
                Some(index)
            } else {
                None
            }
        })?
    } else {
        line.char_indices()
            .skip(1)
            .find_map(|(index, character)| (character == quote).then_some(index))?
    };
    let key_fragment = &line[..=close];
    let parsed = toml::from_str::<toml::Value>(&format!("key = {key_fragment}")).ok()?;
    let key = parsed.get("key").and_then(toml::Value::as_str)?;
    (key == "version").then(|| &line[close + quote.len_utf8()..])
}

fn start_watcher(path: PathBuf) {
    let reload_path = path.clone();
    if let Err(error) =
        file_watch::spawn_path_watcher("injector-config-watch", path, move |trigger| {
            reload_runtime_config(&reload_path, trigger);
        })
    {
        log::error!("failed to start config watcher thread: {}", error);
    }
}

fn reload_runtime_config(path: &Path, trigger: WatchTrigger) {
    let Some(config) = load_or_seed(path, LoadContext::Reload(trigger)) else {
        return;
    };
    if let Some(lock) = CONFIG.get() {
        match lock.write() {
            Ok(mut guard) => {
                let level = config.main.log_level_filter();
                *guard = Arc::new(config);
                log::set_max_level(level);
                log::info!("reloaded config from {} via {}", path.display(), trigger.label());
            }
            Err(error) => {
                log::error!("failed to apply config reload from {}: {}", path.display(), error);
            }
        }
    }
}

fn load_with_read_race_retry<F, S>(
    path: &Path,
    context: LoadContext,
    mut loader: F,
    sleeper: S,
) -> Result<RetryOutcome<InjectorConfig>, LoadError>
where
    F: FnMut(&Path) -> Result<InjectorConfig, LoadError>,
    S: FnMut(Duration),
{
    retry_read_race(
        || loader(path),
        |error| match error {
            LoadError::Missing(_) | LoadError::Io(_) => ReadRaceErrorKind::Retryable,
            LoadError::Parse(_) => ReadRaceErrorKind::Fatal,
        },
        REPLACE_SAVE_RETRY_LIMIT,
        REPLACE_SAVE_RETRY_INTERVAL,
        sleeper,
        |retries, error, interval| {
            log::warn!(
                "{} config load from {} hit read-side race on retry {}/{}: {}; waiting {} ms",
                context.label(),
                path.display(),
                retries,
                REPLACE_SAVE_RETRY_LIMIT,
                error,
                interval.as_millis()
            );
        },
    )
}

pub fn parse_level_filter(value: &str) -> Option<LevelFilter> {
    match value.trim().to_ascii_lowercase().as_str() {
        "off" => Some(LevelFilter::Off),
        "error" => Some(LevelFilter::Error),
        "warn" | "warning" => Some(LevelFilter::Warn),
        "info" => Some(LevelFilter::Info),
        "debug" => Some(LevelFilter::Debug),
        "trace" => Some(LevelFilter::Trace),
        _ => None,
    }
}

impl MainConfig {
    pub fn log_level_filter(&self) -> LevelFilter {
        parse_level_filter(&self.log_level).unwrap_or(LevelFilter::Debug)
    }
}

#[cfg(test)]
mod tests;
