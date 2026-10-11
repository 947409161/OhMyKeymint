use super::*;

#[test]
fn isolated_package_lookup_preserves_original_authorization_identity() {
    let caller = CallerInfo {
        uid: 99001,
        pid: 42,
        sid: "u:r:isolated_app:s0:c123,c456".into(),
    };
    let packages = resolve_packages_for_caller_with(
        &caller,
        |uid| {
            assert_eq!(uid, 99001);
            PackageResolution::Unknown
        },
        |forwarded| {
            assert_eq!(forwarded.uid, 99001);
            assert_eq!(forwarded.pid, 42);
            assert_eq!(forwarded.sid, caller.sid);
            Ok(vec!["com.example.owner".into()])
        },
    );
    assert!(
        matches!(packages, PackageResolution::Known(ref names) if names == &["com.example.owner"])
    );
    assert!(
        crate::filter::evaluate(
            &crate::config::FilterConfig::default(),
            99001,
            packages.clone()
        )
        .allowed
    );
    let mut config = crate::config::FilterConfig::default();
    config.deny_packages.push("com.example.owner".into());
    assert!(!crate::filter::evaluate(&config, 99001, packages).allowed);
    assert_eq!(caller.uid, 99001);
}

#[test]
fn isolated_package_lookup_is_only_an_unknown_uid_fallback() {
    for uid in [10371, 99001] {
        let caller = CallerInfo {
            uid,
            pid: 42,
            sid: "u:r:app:s0".into(),
        };
        assert!(matches!(
            resolve_packages_for_caller_with(
                &caller,
                |_| PackageResolution::Known(vec!["com.already.known".into()]),
                |_| panic!("known caller must not query AMS")
            ),
            PackageResolution::Known(_)
        ));
    }
    let mut caller = CallerInfo {
        uid: 10371,
        pid: 42,
        sid: "u:r:app:s0".into(),
    };
    assert!(matches!(
        resolve_packages_for_caller_with(
            &caller,
            |_| PackageResolution::Unknown,
            |_| panic!("ordinary UID must not query AMS")
        ),
        PackageResolution::Unknown
    ));
    caller.uid = 99001;
    for fail in [false, true] {
        assert!(matches!(
            resolve_packages_for_caller_with(
                &caller,
                |_| PackageResolution::Unknown,
                |_| if fail {
                    Err(anyhow::anyhow!("query unavailable"))
                } else {
                    Ok(Vec::new())
                }
            ),
            PackageResolution::Unknown
        ));
    }
}

#[test]
fn binder_status_classification() {
    let status = Status::from(StatusCode::DeadObject);
    assert!(is_dead_object_status(&status));

    let status = Status::from(StatusCode::Ok);
    assert!(!is_dead_object_status(&status));

    for status in [
        StatusCode::DeadObject,
        StatusCode::RpcError,
        StatusCode::NotEnoughData,
    ] {
        assert!(is_stale_rpc_status_code(status));
        assert!(is_rpc_cache_invalidating_error(&anyhow::Error::new(
            Status::from(status)
        )));
        assert!(is_rpc_cache_invalidating_error(&anyhow::Error::new(status)));
    }

    let shared = Arc::new(
        anyhow::Error::new(StatusCode::DeadObject).context("failed to connect to omk service"),
    );
    assert!(is_rpc_cache_invalidating_error(&shared_rpc_connect_error(
        &shared
    )));

    for status in [
        StatusCode::NoInit,
        StatusCode::Errno(libc::ECONNRESET),
        StatusCode::Errno(libc::ENOTCONN),
        StatusCode::Errno(libc::EPIPE),
    ] {
        assert!(is_rpc_cache_invalidating_status_code(status));
        assert!(is_rpc_cache_invalidating_error(&anyhow::Error::new(
            Status::from(status)
        )));
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

#[test]
fn runtime_package_classification_excludes_system_apps_and_keeps_google_exceptions() {
    let packages = filter_user_installed_packages(
        vec![
            "com.example.user_app".into(),
            "com.example.system_app".into(),
            "com.google.android.gms".into(),
        ],
        |package| match package {
            "com.example.user_app" => Ok(0),
            "com.example.system_app" => Ok(LOCATION_SYSTEM),
            _ => panic!("Google exception must not need package classification"),
        },
    );

    assert_eq!(packages, ["com.example.user_app", "com.google.android.gms"]);
}

#[test]
fn runtime_package_classification_fails_closed_when_location_is_unknown() {
    let packages = filter_user_installed_packages(vec!["com.example.app".into()], |_| {
        Err(anyhow::anyhow!("package_native unavailable"))
    });
    assert!(packages.is_empty());
}
