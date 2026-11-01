use super::*;

#[test]
fn package_uid_matches_android_process_uid_ranges() {
    for version in 13..=17 {
        for user in [0, 1, 10] {
            let base = user * kmr_common::consts::AID_USER_OFFSET;
            for app_id in [0, 1000, 9999, 10_000, 19_999, 40_000, 89_999] {
                assert_eq!(
                    non_isolated_package_uid(base + app_id, Some(version)),
                    Some(base + app_id)
                );
            }
            for app_id in [20_000, 20_123, 29_999] {
                assert_eq!(
                    non_isolated_package_uid(base + app_id, Some(version)),
                    Some(base + app_id - 10_000)
                );
            }
            for app_id in [30_000, 30_123, 39_999] {
                assert_eq!(
                    non_isolated_package_uid(base + app_id, Some(version)),
                    Some(base + app_id - if version >= 17 { 20_000 } else { 0 })
                );
            }
            for app_id in [90_000, 98_999, 99_000, 99_999] {
                assert_eq!(non_isolated_package_uid(base + app_id, Some(version)), None);
            }
        }
    }
    assert_eq!(non_isolated_package_uid(20_123, Some(12)), Some(20_123));
    assert_eq!(non_isolated_package_uid(20_123, None), Some(20_123));
}

#[test]
fn binder_status_classification() {
    let status = Status::from(StatusCode::DeadObject);
    assert!(is_dead_object_status(&status));

    let status = Status::from(StatusCode::Ok);
    assert!(!is_dead_object_status(&status));

    for status in [StatusCode::DeadObject, StatusCode::RpcError, StatusCode::NotEnoughData] {
        assert!(is_stale_rpc_status_code(status));
        assert!(is_rpc_cache_invalidating_error(&anyhow::Error::new(Status::from(status))));
        assert!(is_rpc_cache_invalidating_error(&anyhow::Error::new(status)));
    }

    let shared = Arc::new(
        anyhow::Error::new(StatusCode::DeadObject).context("failed to connect to omk service"),
    );
    assert!(is_rpc_cache_invalidating_error(&shared_rpc_connect_error(&shared)));

    for status in [
        StatusCode::NoInit,
        StatusCode::Errno(libc::ECONNRESET),
        StatusCode::Errno(libc::ENOTCONN),
        StatusCode::Errno(libc::EPIPE),
    ] {
        assert!(is_rpc_cache_invalidating_status_code(status));
        assert!(is_rpc_cache_invalidating_error(&anyhow::Error::new(Status::from(status))));
        assert!(is_rpc_cache_invalidating_error(&anyhow::Error::new(status)));
    }

    let stale = anyhow::Error::new(Status::from(StatusCode::Unknown));
    assert!(!is_stale_rpc_status_code(StatusCode::Unknown));
    assert!(is_rpc_cache_invalidating_error(&stale));
    assert!(!is_dead_object_error(&stale));

    let direct_unknown = anyhow::Error::new(StatusCode::Unknown);
    assert!(!is_rpc_cache_invalidating_error(&direct_unknown));

    let business = anyhow::Error::new(Status::new_service_specific_error(1, None));
    assert!(!is_rpc_cache_invalidating_error(&business));
}
