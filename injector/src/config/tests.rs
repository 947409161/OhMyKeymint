use super::*;
use std::ops::Deref;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::time::{SystemTime, UNIX_EPOCH};

struct TempConfigPath(PathBuf);

impl Deref for TempConfigPath {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Drop for TempConfigPath {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
        if let (Some(parent), Some(file_name)) = (self.0.parent(), self.0.file_name()) {
            let temp =
                parent.join(format!(".{}.tmp-{}", file_name.to_string_lossy(), std::process::id()));
            let _ = fs::remove_file(temp);
        }
    }
}

fn temp_config_path(name: &str) -> TempConfigPath {
    let unique =
        SystemTime::now().duration_since(UNIX_EPOCH).expect("time should move forward").as_nanos();
    TempConfigPath(std::env::temp_dir().join(format!("omk-injector-{name}-{unique}.toml")))
}

#[test]
fn config_defaults_and_log_levels_match_contract() {
    let config = InjectorConfig::default();
    assert!(config.main.enabled);
    assert_eq!(config.main.attestation_generation_delay_ms, [20, 30]);
    assert_eq!(config.main.operation_start_delay_ms, 0);
    assert_eq!(config.main.log_level_filter(), LevelFilter::Debug);
    assert!(config.intercept.get_security_level);
    assert!(config.intercept.get_key_entry);
    assert!(config.intercept.update_subcomponent);
    assert!(config.intercept.list_entries);
    assert!(config.intercept.delete_key);
    assert!(config.intercept.grant);
    assert!(config.intercept.ungrant);
    assert!(config.intercept.get_number_of_entries);
    assert!(config.intercept.list_entries_batched);
    assert!(config.intercept.get_supplementary_attestation_info);

    assert_eq!(parse_level_filter("warn"), Some(LevelFilter::Warn));
    assert_eq!(parse_level_filter("WARNING"), Some(LevelFilter::Warn));
    assert_eq!(parse_level_filter("trace"), Some(LevelFilter::Trace));
    assert_eq!(parse_level_filter("unknown"), None);
}

#[test]
fn attestation_generation_delay_is_optional_bounded_and_preserved() {
    assert_eq!(parse_config("").unwrap().main.attestation_generation_delay_ms, [20, 30]);
    for delay in ["[0, 0]", "[20, 30]", "[25, 25]", "[250, 250]"] {
        let config =
            parse_config(&format!("[main]\nattestation_generation_delay_ms = {delay}\n")).unwrap();
        let rendered = render_config(&config).unwrap();
        assert_eq!(
            parse_config(&rendered).unwrap().main.attestation_generation_delay_ms,
            config.main.attestation_generation_delay_ms,
            "range {delay} must survive a render and reparse round trip"
        );
    }
    for delay in [
        "25",
        "true",
        "\"25\"",
        "[25]",
        "[25, 30, 40]",
        "[30, 20]",
        "[0, 251]",
        "[-1, 25]",
        "[1.5, 2]",
    ] {
        assert!(
            parse_config(&format!("[main]\nattestation_generation_delay_ms = {delay}\n")).is_err()
        );
    }
}

#[test]
fn operation_start_delay_is_optional_bounded_and_preserved() {
    for contents in ["", "version = 1\n[main]\nenabled = true\n"] {
        assert_eq!(parse_config(contents).unwrap().main.operation_start_delay_ms, 0);
    }
    for delay in [0, 12, 250] {
        let config =
            parse_config(&format!("[main]\noperation_start_delay_ms = {delay}\n")).unwrap();
        let rendered = render_config(&config).unwrap();
        assert_eq!(parse_config(&rendered).unwrap().main.operation_start_delay_ms, delay);
    }
    for delay in ["-1", "251", "65536", "1.5", "true", "\"12\""] {
        assert!(parse_config(&format!("[main]\noperation_start_delay_ms = {delay}\n")).is_err());
    }
}

#[test]
fn invalid_operation_start_delay_reload_is_rejected_without_rewriting() {
    let path = temp_config_path("operation-start-delay-reload");
    fs::write(&*path, "version = 1\n[main]\noperation_start_delay_ms = 12\n").unwrap();
    let loaded = load_or_seed(&path, LoadContext::Reload(WatchTrigger::CloseWrite))
        .expect("valid delay should load");
    assert_eq!(loaded.main.operation_start_delay_ms, 12);

    for delay in ["-1", "251", "1.5", "\"12\""] {
        let contents = format!("version = 1\n[main]\noperation_start_delay_ms = {delay}\n");
        fs::write(&*path, &contents).unwrap();
        assert!(load_or_seed(&path, LoadContext::Reload(WatchTrigger::CloseWrite)).is_none());
        assert_eq!(fs::read_to_string(&*path).unwrap(), contents);
    }
}

#[test]
fn removed_routing_settings_are_rejected() {
    for contents in [
        "version = 1\nscoop = [\"com.example.app\"]\n",
        "version = 1\nscoop_details = {}\n",
        "version = 1\n[scoop.com.example.app]\nmode = \"strict\"\n",
        "version = 1\n[filter]\nenabled = true\n",
        "version = 1\n[filter.sub]\nenabled = true\n",
    ] {
        let error = parse_config(contents).expect_err("removed routing settings must be rejected");
        assert!(error.contains("unknown field"), "{error}");
    }
}

#[test]
fn removed_routing_hint_recognizes_filters_and_legacy_lists() {
    for contents in [
        "[filter]\nenabled = true\n",
        "[filter.sub]\nenabled = true\n",
        "scoop = []\n",
        "deny_packages = []\n",
    ] {
        assert!(has_removed_routing_settings(contents), "{contents:?}");
    }
    for contents in [
        "# [filter] is not accepted\n[main]\nenabled = true\n",
        "[intercept]\nget_key_entry = true\n",
    ] {
        assert!(!has_removed_routing_settings(contents), "{contents:?}");
    }
}

#[test]
fn startup_with_removed_routing_settings_disables_without_rewriting() {
    for contents in ["version = 1\nscoop = [\"com.example.app\"]\n", "version = 1\n[filter]\n"] {
        let path = temp_config_path("removed-routing-settings");
        fs::write(&*path, contents).unwrap();

        let loaded = load_or_seed(&path, LoadContext::Startup).unwrap();
        assert!(!loaded.main.enabled);
        assert_eq!(fs::read_to_string(&*path).unwrap(), contents);
    }
}

#[test]
fn rendered_config_contains_no_routing_list() {
    let config = InjectorConfig::default();
    let rendered = render_config(&config).expect("config should render");
    assert!(!rendered.contains("scoop"));
    assert!(!rendered.contains("[filter]"));
    assert!(rendered.contains(&format!("version = {CURRENT_CONFIG_VERSION}")));

    let reparsed = parse_config(&rendered).expect("rendered config should parse");
    assert_eq!(reparsed.main.enabled, config.main.enabled);
    assert_eq!(
        reparsed.main.attestation_generation_delay_ms,
        config.main.attestation_generation_delay_ms
    );
    assert_eq!(reparsed.intercept.get_key_entry, config.intercept.get_key_entry);
}

#[test]
fn missing_config_is_seeded_but_invalid_startup_config_is_untouched() {
    let path = temp_config_path("missing");

    let loaded = load_or_seed(&path, LoadContext::Startup).unwrap();
    assert!(path.exists(), "missing config should be written to disk");
    assert_eq!(loaded.version, CURRENT_CONFIG_VERSION);

    let on_disk = fs::read_to_string(&*path).expect("written config should be readable");
    let reparsed = parse_config(&on_disk).expect("written config should parse");
    assert_eq!(reparsed.main.enabled, loaded.main.enabled);
    assert_eq!(reparsed.intercept.get_key_entry, loaded.intercept.get_key_entry);

    let path = temp_config_path("invalid");
    let invalid = "[main\nbroken";
    fs::write(&*path, invalid).expect("invalid config should be written");

    let loaded = load_or_seed(&path, LoadContext::Startup).unwrap();
    assert!(!loaded.main.enabled, "invalid startup config must disable injection");
    assert_eq!(fs::read_to_string(&*path).unwrap(), invalid);
    assert!(!PathBuf::from(format!("{}.bak", path.display())).exists());
}

#[test]
fn v0_config_migrates_and_preserves_mode() {
    let path = temp_config_path("v0-migration");
    let v0 = "\u{feff}version = 0\r\n\r\n[main]\r\nenabled = false\r\n";
    fs::write(&*path, v0).unwrap();
    fs::set_permissions(&*path, fs::Permissions::from_mode(0o640)).unwrap();

    let loaded = load_from_path(&path, true).expect("v0 config should migrate");
    assert_eq!(loaded.version, CURRENT_CONFIG_VERSION);
    assert!(!loaded.main.enabled);

    let migrated = fs::read_to_string(&*path).unwrap();
    assert_eq!(migrated, v0.replacen("version = 0", "version = 1", 1));
    assert_eq!(fs::metadata(&*path).unwrap().mode() & 0o777, 0o640);
}

#[test]
fn explicit_v0_migration_only_replaces_the_version_value() {
    let v0 = "# keep this comment\nversion = 0 # and this one\n[main]\nenabled = true\n";
    let (_, migrated) = parse_versioned_config(v0, true).expect("v0 config should migrate");
    assert_eq!(
        migrated.as_deref(),
        Some("# keep this comment\nversion = 1 # and this one\n[main]\nenabled = true\n")
    );
}

#[test]
fn quoted_version_key_migrates_without_changing_other_text() {
    let v0 = "\"version\" = 0 # keep this comment\n[main]\nenabled = true\n";
    let (_, migrated) = parse_versioned_config(v0, true).expect("v0 config should migrate");

    assert_eq!(
        migrated.as_deref(),
        Some("\"version\" = 1 # keep this comment\n[main]\nenabled = true\n")
    );
}

#[test]
fn reload_rejects_v0_without_rewriting() {
    let path = temp_config_path("reload-v0");
    let v0 = "[main]\nenabled = true\n";
    fs::write(&*path, v0).unwrap();

    assert!(load_or_seed(&path, LoadContext::Reload(WatchTrigger::CloseWrite)).is_none());
    assert_eq!(fs::read_to_string(&*path).unwrap(), v0);
}

#[test]
fn unsupported_versions_are_rejected_without_rewriting() {
    for contents in ["version = -1\n", "version = 2\n", "version = \"1\"\n"] {
        assert!(parse_config(contents).is_err());
    }

    let path = temp_config_path("future-version");
    let future = "version = 2\n";
    fs::write(&*path, future).unwrap();
    let loaded = load_or_seed(&path, LoadContext::Startup).unwrap();
    assert!(!loaded.main.enabled);
    assert_eq!(fs::read_to_string(&*path).unwrap(), future);
}

#[test]
fn template_matches_defaults_and_contains_no_routing_list() {
    let template = include_str!("../../../template/injector.toml");
    assert!(!template.contains("scoop"));
    assert!(!template.contains("[filter]"));
    let parsed = parse_config(template).expect("template injector config should parse");
    let defaults = InjectorConfig::default();
    assert_eq!(parsed.version, defaults.version);
    assert_eq!(parsed.main.enabled, defaults.main.enabled);
    assert_eq!(parsed.main.log_level, defaults.main.log_level);
    assert_eq!(
        parsed.main.attestation_generation_delay_ms,
        defaults.main.attestation_generation_delay_ms
    );
    assert_eq!(parsed.main.operation_start_delay_ms, defaults.main.operation_start_delay_ms);
    assert_eq!(parsed.intercept.get_key_entry, defaults.intercept.get_key_entry);
}

#[test]
fn replace_save_retry_only_retries_read_failures() {
    let path = temp_config_path("replace-save-retry");
    let mut attempts = 0usize;
    let mut sleeps = Vec::new();

    let loaded = load_with_read_race_retry(
        &path,
        LoadContext::Reload(WatchTrigger::ReplaceSave),
        |_path| {
            attempts += 1;
            match attempts {
                1 => Err(LoadError::Io(io::Error::from(io::ErrorKind::NotFound))),
                2 => Err(LoadError::Io(io::Error::from(io::ErrorKind::PermissionDenied))),
                _ => Ok(InjectorConfig::default()),
            }
        },
        |duration| sleeps.push(duration),
    )
    .expect("replace-save retry should eventually succeed");

    assert_eq!(loaded.retries, 2);
    assert_eq!(attempts, 3);
    assert_eq!(sleeps.len(), 2);
    assert!(sleeps.iter().all(|duration| *duration == REPLACE_SAVE_RETRY_INTERVAL));

    let path = temp_config_path("replace-save-parse");
    let mut sleeps = Vec::new();

    let error = load_with_read_race_retry(
        &path,
        LoadContext::Reload(WatchTrigger::ReplaceSave),
        |_path| Err(LoadError::Parse("broken".to_string())),
        |duration| sleeps.push(duration),
    )
    .expect_err("parse failures should bypass replace-save retries");

    assert!(matches!(error, LoadError::Parse(_)));
    assert!(sleeps.is_empty());
}
