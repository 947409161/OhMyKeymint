use std::sync::OnceLock;

pub const GOOGLE_PACKAGE_EXCEPTIONS: [&str; 3] = [
    "com.google.android.gsf",
    "com.google.android.gms",
    "com.android.vending",
];

#[derive(Debug, Clone)]
pub struct InjectorConfig {
    pub main: MainConfig,
    pub filter: FilterConfig,
    pub intercept: InterceptConfig,
}

#[derive(Debug, Clone)]
pub struct MainConfig {
    pub enabled: bool,
    pub attestation_generation_delay_ms: [u16; 2],
    pub operation_start_delay_ms: u16,
}

#[derive(Debug, Clone)]
pub struct FilterConfig {
    pub enabled: bool,
    pub deny_packages: Vec<String>,
    pub block_android_package: bool,
    pub allow_unknown_package: bool,
}

#[derive(Debug, Clone)]
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
            attestation_generation_delay_ms: [20, 30],
            operation_start_delay_ms: 0,
        }
    }
}

impl Default for FilterConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            deny_packages: Vec::new(),
            block_android_package: true,
            allow_unknown_package: false,
        }
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

static CONFIG: OnceLock<InjectorConfig> = OnceLock::new();

pub fn get() -> &'static InjectorConfig {
    CONFIG.get_or_init(InjectorConfig::default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn injector_configuration_is_hardcoded() {
        let config = get();
        assert_eq!(
            GOOGLE_PACKAGE_EXCEPTIONS,
            [
                "com.google.android.gsf",
                "com.google.android.gms",
                "com.android.vending",
            ]
        );
        assert!(config.main.enabled);
        assert!(config.filter.enabled);
        assert!(config.filter.block_android_package);
        assert!(!config.filter.allow_unknown_package);
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
    }
}
