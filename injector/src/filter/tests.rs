use super::*;
use kmr_common::app_scope::{global_packages, AppScope};
use std::collections::{BTreeMap, BTreeSet};

fn base_config() -> FilterConfig {
    FilterConfig::default()
}

fn scope_with(users: &[(u32, &[&str])]) -> AppScope {
    let global = global_packages();
    let users: BTreeMap<u32, BTreeSet<String>> = users
        .iter()
        .map(|(user, packages)| {
            let packages: BTreeSet<String> = packages.iter().map(|name| name.to_string()).collect();
            (*user, packages)
        })
        .collect();
    AppScope::from_parts(global, users)
}

fn base_scope() -> AppScope {
    scope_with(&[(0, &["com.allowed"]), (10, &["com.allowed"])])
}

#[test]
fn disabled_filter_reports_disabled_and_allows() {
    let mut config = base_config();
    config.enabled = false;

    let decision = evaluate(&base_scope(), &config, 10_000, PackageResolution::Unknown);
    assert!(decision.allowed);
    assert_eq!(decision.reason, FilterReason::Disabled);
}

#[test]
fn android_package_is_rejected_when_blocking_is_enabled() {
    let config = base_config();

    let decision = evaluate(
        &base_scope(),
        &config,
        10_000,
        PackageResolution::Known(vec!["android".to_string()]),
    );
    assert!(!decision.allowed);
    assert_eq!(decision.reason, FilterReason::RejectedAndroidPackage);
}

#[test]
fn known_package_outside_scope_is_rejected() {
    let mut config = base_config();
    config.block_android_package = false;

    let decision = evaluate(
        &base_scope(),
        &config,
        10_000,
        PackageResolution::Known(vec!["com.other".to_string()]),
    );
    assert!(!decision.allowed);
    assert_eq!(decision.reason, FilterReason::RejectedNotInScope);
}

#[test]
fn unknown_package_policy_only_allows_app_uids() {
    let mut config = base_config();
    config.allow_unknown_package = true;

    for uid in [10_000, 110_000] {
        let decision = evaluate(&base_scope(), &config, uid, PackageResolution::Unknown);
        assert!(decision.allowed);
        assert_eq!(decision.reason, FilterReason::Allowed);
    }
    for uid in [9_999, 109_999] {
        let decision = evaluate(&base_scope(), &config, uid, PackageResolution::Unknown);
        assert!(!decision.allowed);
        assert_eq!(decision.reason, FilterReason::RejectedAndroidPackage);
    }
}

#[test]
fn root_follows_android_package_policy() {
    let mut config = base_config();
    config.allow_unknown_package = true;

    let decision = evaluate(&base_scope(), &config, 0, PackageResolution::Unknown);
    assert!(!decision.allowed);
    assert_eq!(decision.reason, FilterReason::RejectedAndroidPackage);

    config.block_android_package = false;
    let decision = evaluate(&base_scope(), &config, 0, PackageResolution::Unknown);
    assert!(decision.allowed);
    assert_eq!(decision.reason, FilterReason::Allowed);
}

#[test]
fn global_google_packages_are_allowed_in_every_user() {
    let config = base_config();
    for uid in [10_000, 1_010_000, 2_010_000] {
        let decision = evaluate(
            &scope_with(&[]),
            &config,
            uid,
            PackageResolution::Known(vec!["com.google.android.gms".to_string()]),
        );
        assert!(decision.allowed, "caller uid={uid}");
        assert_eq!(decision.reason, FilterReason::Allowed);
    }
}

#[test]
fn empty_scope_rejects_resolved_packages() {
    let mut config = base_config();
    config.block_android_package = false;
    let decision = evaluate(
        &scope_with(&[]),
        &config,
        10_000,
        PackageResolution::Known(vec!["com.anything".to_string()]),
    );
    assert!(!decision.allowed);
    assert_eq!(decision.reason, FilterReason::RejectedNotInScope);

    let config = base_config();
    let decision = evaluate(
        &scope_with(&[]),
        &config,
        10_000,
        PackageResolution::Known(vec!["android".to_string()]),
    );
    assert_eq!(decision.reason, FilterReason::RejectedAndroidPackage);
}

#[test]
fn scope_is_evaluated_per_android_user() {
    let config = base_config();
    let scope = scope_with(&[(10, &["com.allowed"])]);
    for (uid, allowed) in [(10_123, false), (1_010_123, true), (1_110_123, false)] {
        let decision = evaluate(
            &scope,
            &config,
            uid,
            PackageResolution::Known(vec!["com.allowed".to_string()]),
        );
        assert_eq!(decision.allowed, allowed, "caller uid={uid}");
    }
}

#[test]
fn any_in_scope_package_of_a_shared_uid_allows_the_caller() {
    let config = base_config();
    let decision = evaluate(
        &scope_with(&[(10, &["com.allowed"])]),
        &config,
        1_010_123,
        PackageResolution::Known(vec!["com.other".to_string(), "com.allowed".to_string()]),
    );
    assert!(decision.allowed);
    assert_eq!(decision.reason, FilterReason::Allowed);

    let decision = evaluate(
        &scope_with(&[(0, &["com.allowed"])]),
        &config,
        1_010_123,
        PackageResolution::Known(vec!["com.allowed".to_string()]),
    );
    assert!(!decision.allowed);
    assert_eq!(decision.reason, FilterReason::RejectedNotInScope);
}
