use super::*;

#[test]
fn dynamically_resolved_user_apps_are_in_global_scope() {
    let config = FilterConfig::default();
    for uid in [10_000, 110_000] {
        let decision = evaluate(
            &config,
            uid,
            PackageResolution::Known(vec!["com.example.user_app".to_string()]),
        );

        assert!(decision.allowed, "expected UID {uid} to be in global scope");
        assert_eq!(decision.reason, FilterReason::Allowed);
    }
}

#[test]
fn disabled_filter_reports_disabled_and_allows() {
    let mut config = FilterConfig::default();
    config.enabled = false;

    let decision = evaluate(&config, 10_000, PackageResolution::Unknown);
    assert!(decision.allowed);
    assert_eq!(decision.reason, FilterReason::Disabled);
}

#[test]
fn android_package_is_rejected_when_blocking_is_enabled() {
    let config = FilterConfig::default();

    let decision = evaluate(
        &config,
        10_000,
        PackageResolution::Known(vec!["android".to_string()]),
    );
    assert!(!decision.allowed);
    assert_eq!(decision.reason, FilterReason::RejectedAndroidPackage);
}

#[test]
fn denylist_rejection_takes_precedence_over_scope() {
    let mut config = FilterConfig::default();
    config.block_android_package = false;
    config.deny_packages = vec!["com.example.app".to_string()];
    let decision = evaluate(
        &config,
        10_000,
        PackageResolution::Known(vec!["com.example.app".to_string()]),
    );
    assert!(!decision.allowed);
    assert_eq!(decision.reason, FilterReason::RejectedByDenylist);
}

#[test]
fn unknown_package_policy_only_allows_app_uids() {
    let mut config = FilterConfig::default();
    config.allow_unknown_package = true;

    for uid in [10_000, 110_000] {
        let decision = evaluate(&config, uid, PackageResolution::Unknown);
        assert!(decision.allowed);
        assert_eq!(decision.reason, FilterReason::Allowed);
    }
    for uid in [9_999, 109_999] {
        let decision = evaluate(&config, uid, PackageResolution::Unknown);
        assert!(!decision.allowed);
        assert_eq!(decision.reason, FilterReason::RejectedAndroidPackage);
    }
}

#[test]
fn root_follows_android_package_policy() {
    let mut config = FilterConfig::default();
    config.allow_unknown_package = true;

    let decision = evaluate(&config, 0, PackageResolution::Unknown);
    assert!(!decision.allowed);
    assert_eq!(decision.reason, FilterReason::RejectedAndroidPackage);

    config.block_android_package = false;
    let decision = evaluate(&config, 0, PackageResolution::Unknown);
    assert!(decision.allowed);
    assert_eq!(decision.reason, FilterReason::Allowed);
}

#[test]
fn known_package_is_allowed_without_a_whitelist() {
    let config = FilterConfig::default();
    let decision = evaluate(
        &config,
        10_000,
        PackageResolution::Known(vec!["com.example.user_app".to_string()]),
    );
    assert!(decision.allowed);
    assert_eq!(decision.reason, FilterReason::Allowed);
}

#[test]
fn android_and_denylist_rejections_remain_active() {
    let config = FilterConfig::default();
    let decision = evaluate(
        &config,
        10_000,
        PackageResolution::Known(vec!["android".to_string()]),
    );
    assert_eq!(decision.reason, FilterReason::RejectedAndroidPackage);

    let mut config = FilterConfig::default();
    config.block_android_package = false;
    config.deny_packages = vec!["com.blocked".to_string()];
    let decision = evaluate(
        &config,
        10_000,
        PackageResolution::Known(vec!["com.blocked".to_string()]),
    );
    assert_eq!(decision.reason, FilterReason::RejectedByDenylist);
}
