use super::*;
use crate::identify::{AuthorizationMethod, MaintenanceMethod};
use rsbinder::Parcel;

const INTERFACES: [&str; 3] = [
    identify::KEYSTORE_SERVICE_INTERFACE,
    identify::KEYSTORE_MAINTENANCE_INTERFACE,
    identify::KEYSTORE_AUTHORIZATION_INTERFACE,
];

fn target_for(interface: &str) -> Option<LocalBinderTarget> {
    INTERFACES.iter().position(|known| *known == interface).map(|index| LocalBinderTarget {
        ptr: 0x1000 + index as libc::c_ulong,
        cookie: 0x2000 + index as libc::c_ulong,
    })
}

fn request_for(interface: &str) -> (u32, Parcel) {
    let mut request = Parcel::new();
    request.write(&0i32).unwrap();
    request.write(&0i32).unwrap();
    request.write(&rsbinder::INTERFACE_HEADER).unwrap();
    request.write(&interface.to_owned()).unwrap();
    let code = match interface {
        identify::KEYSTORE_SERVICE_INTERFACE => {
            request.write(&sample_key_descriptor()).unwrap();
            service_tx::r#getKeyEntry
        }
        identify::KEYSTORE_MAINTENANCE_INTERFACE => {
            request.write(&10i32).unwrap();
            (1..=32)
                .find(|code| {
                    identify::maintenance_method_from_code(*code)
                        == Some(MaintenanceMethod::OnUserAdded)
                })
                .unwrap()
        }
        identify::KEYSTORE_AUTHORIZATION_INTERFACE => {
            request.write(&0i64).unwrap();
            request.write(&0i64).unwrap();
            request.write(&0i64).unwrap();
            (1..=32)
                .find(|code| {
                    identify::authorization_method_from_code(*code)
                        == Some(AuthorizationMethod::GetAuthTokensForCredStore)
                })
                .unwrap()
        }
        _ => panic!("unexpected interface"),
    };
    (code, request)
}

#[test]
fn cross_interface_requests_leave_original_transaction_and_reply_untouched() {
    let _guard = route_state_test_guard();
    for requested in INTERFACES {
        let (code, mut request) = request_for(requested);
        for actual in INTERFACES.into_iter().filter(|actual| *actual != requested) {
            let mut tr = transaction_for_parcel(target_for(actual).unwrap(), code, &mut request);
            let original = tr;
            push_pending_frame(101);
            assert!(!unsafe {
                handle_keystore_transaction(
                    101,
                    &mut tr,
                    None,
                    "BR_TRANSACTION",
                    &config::InjectorConfig::default(),
                    target_for,
                )
            });
            assert_eq!(tr.code, code);
            assert_eq!(tr.flags, original.flags);
            assert_eq!(tr.data_size, original.data_size);
            assert_eq!(tr.offsets_size, original.offsets_size);
            assert_eq!(unsafe { tr.data.ptr.buffer }, unsafe { original.data.ptr.buffer });
            assert_eq!(unsafe { tr.data.ptr.offsets }, unsafe { original.data.ptr.offsets });
            assert!(
                matches!(take_top_pending(101), Some(None)),
                "wrong interface must not schedule OMK execution or mirroring"
            );
        }
    }
}

#[test]
fn matching_service_maintenance_and_authorization_requests_keep_their_routes() {
    let _guard = route_state_test_guard();
    for requested in INTERFACES {
        let (code, mut request) = request_for(requested);
        let mut tr = transaction_for_parcel(target_for(requested).unwrap(), code, &mut request);
        tr.sender_euid = 1000;
        push_pending_frame(102);
        assert!(!unsafe {
            handle_keystore_transaction(
                102,
                &mut tr,
                None,
                "BR_TRANSACTION",
                &config::InjectorConfig::default(),
                target_for,
            )
        });
        let pending = take_top_pending(102)
            .flatten()
            .expect("matching target should reach its ordinary dispatch");
        match pending {
            PendingCall::Service(call) => {
                assert_eq!(requested, identify::KEYSTORE_SERVICE_INTERFACE);
                assert_eq!(call.route, RouteTarget::System);
            }
            PendingCall::Maintenance(call) => {
                assert_eq!(requested, identify::KEYSTORE_MAINTENANCE_INTERFACE);
                assert_eq!(call.route, RouteTarget::System);
                assert!(matches!(call.request, ParsedMaintenanceRequest::OnUserAdded { .. }));
            }
            PendingCall::Authorization(call) => {
                assert_eq!(requested, identify::KEYSTORE_AUTHORIZATION_INTERFACE);
                assert_eq!(call.method, AuthorizationMethod::GetAuthTokensForCredStore);
            }
            _ => panic!("matching request should keep its normal non-mutating route"),
        }
    }
}

#[test]
fn known_target_rejects_other_token_even_when_other_service_is_not_ready() {
    let target = target_for(identify::KEYSTORE_SERVICE_INTERFACE).unwrap();
    assert_eq!(
        registered_interface_matches_target(
            target,
            identify::KEYSTORE_MAINTENANCE_INTERFACE,
            &|interface| { (interface == identify::KEYSTORE_SERVICE_INTERFACE).then_some(target) }
        ),
        Some(false)
    );
    assert_eq!(
        registered_interface_matches_target(
            target,
            identify::KEYSTORE_MAINTENANCE_INTERFACE,
            &|_| None
        ),
        None
    );
}

#[test]
fn target_identity_includes_both_pointer_and_cookie() {
    let target = target_for(identify::KEYSTORE_SERVICE_INTERFACE).unwrap();
    for changed in [
        LocalBinderTarget { ptr: target.ptr + 1, ..target },
        LocalBinderTarget { cookie: target.cookie + 1, ..target },
    ] {
        assert_eq!(
            registered_interface_matches_target(
                changed,
                identify::KEYSTORE_SERVICE_INTERFACE,
                &target_for
            ),
            Some(false)
        );
    }
}

#[test]
fn missing_local_target_cannot_enter_service_dispatch() {
    let _guard = route_state_test_guard();
    let (code, mut request) = request_for(identify::KEYSTORE_MAINTENANCE_INTERFACE);
    for target in
        [LocalBinderTarget { ptr: 0, cookie: 123 }, LocalBinderTarget { ptr: 123, cookie: 0 }]
    {
        let mut tr = transaction_for_parcel(target, code, &mut request);
        push_pending_frame(103);
        assert!(!unsafe {
            handle_keystore_transaction(
                103,
                &mut tr,
                None,
                "BR_TRANSACTION",
                &config::InjectorConfig::default(),
                |_| panic!("missing local target must be rejected first"),
            )
        });
        assert_eq!(tr.code, code);
        assert!(matches!(take_top_pending(103), Some(None)));
    }
}

#[test]
fn unverified_maintenance_mutation_is_blocked_and_returns_system_error() {
    let _guard = route_state_test_guard();
    let mut request = Parcel::new();
    request.write(&0i32).unwrap();
    request.write(&0i32).unwrap();
    request.write(&rsbinder::INTERFACE_HEADER).unwrap();
    request.write(&identify::KEYSTORE_MAINTENANCE_INTERFACE.to_owned()).unwrap();
    request.write(&10i32).unwrap();
    for one_way in [false, true] {
        let mut tr = transaction_for_parcel(
            target_for(identify::KEYSTORE_MAINTENANCE_INTERFACE).unwrap(),
            1,
            &mut request,
        );
        if one_way {
            tr.flags = crate::hook::binder::TF_ONE_WAY;
        } else {
            push_pending_frame(104);
        }
        assert!(unsafe {
            handle_keystore_transaction(
                104,
                &mut tr,
                None,
                "BR_TRANSACTION",
                &config::InjectorConfig::default(),
                |_| None,
            )
        });
        assert_eq!(tr.code, u32::MAX, "neither backend may perform an unverified mutation");
        if one_way {
            assert!(take_top_pending(104).is_none());
            continue;
        }
        let success = parcel::build_void_reply().unwrap();
        let mut reply: binder_transaction_data = unsafe { std::mem::zeroed() };
        reply.data.ptr.buffer = success.data_ptr() as libc::c_ulong;
        reply.data_size = success.data_size();
        let frame = unsafe { handle_bc_reply(104, &mut reply) }.unwrap();
        let (data, size, offsets, offsets_size) = unsafe { transaction_parts(&reply) };
        let status =
            unsafe { parcel::parse_reply_status(data, size, offsets, offsets_size) }.unwrap();
        assert_eq!(status.exception_code(), ExceptionCode::ServiceSpecific);
        assert_eq!(status.service_specific_error(), ResponseCode::SYSTEM_ERROR.0);
        let _ = commit_bc_reply(104, Some(frame), unsafe { reply.data.ptr.buffer } as usize);
        clear_outbound_reply_buffers(104);
    }
}

#[test]
fn unverified_system_routed_service_requests_are_unchanged() {
    let _guard = route_state_test_guard();
    let (code, mut request) = request_for(identify::KEYSTORE_SERVICE_INTERFACE);
    let mut tr = transaction_for_parcel(
        target_for(identify::KEYSTORE_SERVICE_INTERFACE).unwrap(),
        code,
        &mut request,
    );
    tr.sender_euid = 1000;
    push_pending_frame(105);
    assert!(!unsafe {
        handle_keystore_transaction(
            105,
            &mut tr,
            None,
            "BR_TRANSACTION",
            &config::InjectorConfig::default(),
            |_| None,
        )
    });
    assert_eq!(tr.code, code);
    assert!(matches!(take_top_pending(105), Some(None)));
}

#[test]
fn unverified_best_effort_auth_tokens_preserve_system_authentication() {
    use crate::android::hardware::security::keymint::HardwareAuthToken::HardwareAuthToken;

    let _guard = route_state_test_guard();
    let mut request = Parcel::new();
    request.write(&0i32).unwrap();
    request.write(&0i32).unwrap();
    request.write(&rsbinder::INTERFACE_HEADER).unwrap();
    request.write(&identify::KEYSTORE_AUTHORIZATION_INTERFACE.to_owned()).unwrap();
    request.write(&HardwareAuthToken::default()).unwrap();
    for one_way in [false, true] {
        let mut tr = transaction_for_parcel(
            target_for(identify::KEYSTORE_AUTHORIZATION_INTERFACE).unwrap(),
            1,
            &mut request,
        );
        if one_way {
            tr.flags = crate::hook::binder::TF_ONE_WAY;
        } else {
            push_pending_frame(106);
        }
        assert!(!unsafe {
            handle_keystore_transaction(
                106,
                &mut tr,
                None,
                "BR_TRANSACTION",
                &config::InjectorConfig::default(),
                |_| None,
            )
        });
        assert_eq!(tr.code, 1);
        if one_way {
            assert!(take_top_pending(106).is_none());
        } else {
            assert!(matches!(take_top_pending(106), Some(None)));
        }
    }
}

#[test]
fn service_identity_requirement_follows_intercept_configuration() {
    let request = ParsedServiceRequest::DeleteKey { key: sample_key_descriptor() };
    let enabled = config::InterceptConfig::default();
    let disabled = disabled_intercept_config();
    assert!(service_request_needs_identity(&request, &enabled, 10_123));
    assert!(!service_request_needs_identity(&request, &disabled, 10_123));
    assert!(!service_request_needs_identity(&request, &enabled, 1000));
}

#[test]
fn app_callers_route_to_omk_and_system_identities_stay_on_system() {
    let _guard = route_state_test_guard();

    let enabled = config::InjectorConfig::default();
    let disabled =
        config::InjectorConfig { intercept: disabled_intercept_config(), ..Default::default() };

    for uid in [0, 1000, 1017, 2000] {
        for cfg in [&enabled, &disabled] {
            assert_service_route(uid, cfg, RouteTarget::System);
        }
    }
    for uid in [10_000, 10_123, 20_123, 99_001] {
        assert_service_route(uid, &enabled, RouteTarget::Omk);
        assert_service_route(uid, &disabled, RouteTarget::System);
    }
}

fn assert_service_route(uid: i32, cfg: &config::InjectorConfig, expected: RouteTarget) {
    let (code, mut request) = request_for(identify::KEYSTORE_SERVICE_INTERFACE);
    let mut tr = transaction_for_parcel(
        target_for(identify::KEYSTORE_SERVICE_INTERFACE).unwrap(),
        code,
        &mut request,
    );
    tr.sender_euid = uid;
    push_pending_frame(107);
    assert!(!unsafe {
        handle_keystore_transaction(107, &mut tr, None, "BR_TRANSACTION", cfg, target_for)
    });
    let pending =
        take_top_pending(107).flatten().expect("every caller should reach service dispatch");
    match pending {
        PendingCall::Service(call) => assert_eq!(call.route, expected, "uid {uid}"),
        _ => panic!("expected service dispatch for uid {uid}"),
    }
}
