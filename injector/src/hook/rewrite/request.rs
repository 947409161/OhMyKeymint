use super::*;
use kmr_common::consts::{AID_APP_START, AID_USER_OFFSET};

pub(super) fn target_from_transaction(tr: &binder_transaction_data) -> Option<LocalBinderTarget> {
    let ptr = unsafe { tr.target.ptr };
    if ptr == 0 || tr.cookie == 0 {
        return None;
    }
    Some(LocalBinderTarget { ptr, cookie: tr.cookie })
}

/// System identities keep using AOSP keystore2: the pre-unlock credential path
/// runs in system_server and cannot use OMK's separate key store.
fn is_app_caller(uid: u32) -> bool {
    uid % AID_USER_OFFSET >= AID_APP_START
}

fn route_for_service_request(
    request: &ParsedServiceRequest,
    intercept: &config::InterceptConfig,
    uid: u32,
) -> RouteTarget {
    if !is_app_caller(uid) {
        return RouteTarget::System;
    }
    if identify::is_omk_service_route_enabled(request.method(), intercept) {
        RouteTarget::Omk
    } else {
        RouteTarget::System
    }
}

fn route_for_maintenance_request(request: &ParsedMaintenanceRequest, uid: u32) -> RouteTarget {
    if !is_app_caller(uid) {
        return RouteTarget::System;
    }
    match request {
        ParsedMaintenanceRequest::MigrateKeyNamespace { .. } => RouteTarget::Omk,
        _ => RouteTarget::System,
    }
}

fn service_request_needs_identity(
    request: &ParsedServiceRequest,
    intercept: &config::InterceptConfig,
    uid: u32,
) -> bool {
    route_for_service_request(request, intercept, uid) == RouteTarget::Omk
}

pub(super) fn security_level_intercept_enabled(intercept: &config::InterceptConfig) -> bool {
    intercept.get_security_level || intercept.get_key_entry
}

fn route_for_security_level(intercept: &config::InterceptConfig, uid: u32) -> RouteTarget {
    if !is_app_caller(uid) {
        return RouteTarget::System;
    }
    if security_level_intercept_enabled(intercept) {
        RouteTarget::Omk
    } else {
        RouteTarget::System
    }
}

fn is_known_keystore_interface(interface: &str) -> bool {
    identify::KNOWN_KEYSTORE_INTERFACES.contains(&interface)
}

// None means the trusted ServiceManager identity is not ready yet. A known
// mismatch must reach the original AIDL interface check without OMK effects.
fn registered_interface_matches_target(
    target: LocalBinderTarget,
    interface: &str,
    lookup: &impl Fn(&str) -> Option<LocalBinderTarget>,
) -> Option<bool> {
    let registered_interfaces = [
        identify::KEYSTORE_SERVICE_INTERFACE,
        identify::KEYSTORE_MAINTENANCE_INTERFACE,
        identify::KEYSTORE_AUTHORIZATION_INTERFACE,
    ];
    if !registered_interfaces.contains(&interface) {
        // Security-level and operation nodes use their existing target maps.
        return Some(true);
    }
    if let Some(expected) = lookup(interface) {
        return Some(expected == target);
    }
    if registered_interfaces
        .iter()
        .any(|other| *other != interface && lookup(other) == Some(target))
    {
        return Some(false);
    }
    None
}

fn block_unverified_interface(
    connection: BinderStateKey,
    tr: &mut binder_transaction_data,
    caller: CallerInfo,
) -> bool {
    warn!(
        "event=interface registered Binder identity not ready code=0x{:x} uid={} pid={}; returning SYSTEM_ERROR before backend execution",
        tr.code, caller.uid, caller.pid
    );
    block_system_request(tr);
    if (tr.flags & super::super::binder::TF_ONE_WAY) == 0 {
        replace_top_pending(connection, PendingCall::InterfaceBoundaryFailure(caller));
    }
    true
}

pub(in crate::hook) unsafe fn handle_br_transaction(
    connection: BinderStateKey,
    tr: &mut binder_transaction_data,
    caller_sid: Option<String>,
    command_name: &str,
) -> bool {
    let expects_reply = (tr.flags & super::super::binder::TF_ONE_WAY) == 0;
    if expects_reply {
        push_pending_frame(connection);
    }

    if forward::is_bypassed() {
        debug!(
            "skipped {} because bypass is active: code=0x{:x} uid={} pid={}",
            command_name, tr.code, tr.sender_euid, tr.sender_pid
        );
        return false;
    }

    let cfg = config::get();
    if !cfg.main.enabled {
        debug!("event=decision injector disabled by config");
        return false;
    }

    handle_keystore_transaction(
        connection,
        tr,
        caller_sid,
        command_name,
        &cfg,
        super::super::binder::registered_service_target,
    )
}

unsafe fn handle_keystore_transaction(
    connection: BinderStateKey,
    tr: &mut binder_transaction_data,
    caller_sid: Option<String>,
    command_name: &str,
    cfg: &config::InjectorConfig,
    lookup_service_target: impl Fn(&str) -> Option<LocalBinderTarget>,
) -> bool {
    let expects_reply = (tr.flags & super::super::binder::TF_ONE_WAY) == 0;

    let Some(parcel_bytes) = super::super::binder::transaction_data_bytes(tr) else {
        warn!(
            "event=decision null parcel buffer for {} code=0x{:x} uid={} pid={}",
            command_name, tr.code, tr.sender_euid, tr.sender_pid
        );
        return false;
    };

    let (data, data_size, offsets, offsets_size) = transaction_parts(tr);
    let request_interface =
        match parcel::peek_request_interface(data, data_size, offsets, offsets_size) {
            Ok(interface) => interface,
            Err(error) => {
                if parcel::contains_known_keystore_interface(parcel_bytes) {
                    debug!(
                        "event=decision failed to read keystore interface code=0x{:x}: {:#}",
                        tr.code, error
                    );
                }
                return false;
            }
        };
    if !is_known_keystore_interface(&request_interface) {
        return false;
    }
    let Some(target) = target_from_transaction(tr) else {
        return false;
    };
    let interface_ready = match registered_interface_matches_target(
        target,
        &request_interface,
        &lookup_service_target,
    ) {
        Some(true) => true,
        Some(false) => {
            debug!(
                "event=interface request token does not match registered Binder target interface={} code=0x{:x} target={}; preserving system interface rejection",
                request_interface, tr.code, format_target(tr)
            );
            return false;
        }
        None => false,
    };
    let caller = CallerInfo {
        uid: i64::from(tr.sender_euid.max(0)),
        sid: caller_sid.unwrap_or_default(),
        pid: i64::from(tr.sender_pid),
    };
    let caller_uid = caller.uid;
    // Authorization events are emitted by system auth components, not by the
    // app that later uses an auth-bound key. Mirror this global keystore state
    // after the system service accepts it.
    if request_interface == identify::KEYSTORE_AUTHORIZATION_INTERFACE {
        let request = match parcel::parse_authorization_request(
            data,
            data_size,
            offsets,
            offsets_size,
            tr.code,
        ) {
            Ok(request) => request,
            Err(error) => {
                debug!(
                    "event=decision failed to parse IKeystoreAuthorization request code=0x{:x}: {:#}",
                    tr.code, error
                );
                return false;
            }
        };

        let method = request.method();
        if !interface_ready {
            return if authorization_requires_mirror(&request)
                && authorization_mirror_failure_policy(&request) == MirrorFailurePolicy::FailClosed
            {
                block_unverified_interface(connection, tr, caller)
            } else {
                false
            };
        }
        info!(
            "event=decision command={} authorization_method={:?} code=0x{:x} uid={} pid={} sid='{}'; mirroring auth state to OMK after system success",
            command_name,
            method,
            tr.code,
            caller.uid,
            caller.pid,
            caller.sid,
        );

        let requires_mirror = authorization_requires_mirror(&request);
        let failure_policy = authorization_mirror_failure_policy(&request);
        let mut pending = PendingAuthorizationCall { request, method, caller, mirror_update: None };
        if requires_mirror && !expects_reply {
            if failure_policy == MirrorFailurePolicy::BestEffort {
                warn!(
                    "event=mirror cannot confirm one-way best-effort authorization {:?} for uid={} pid={}; preserving System mutation without an OMK mirror",
                    method, caller_uid, pending.caller.pid
                );
                return false;
            }
            warn!(
                "event=mirror cannot confirm one-way authorization {:?} for uid={} pid={}; blocking System mutation",
                method, caller_uid, pending.caller.pid
            );
            block_system_request(tr);
            return true;
        }
        if requires_mirror {
            match reserve_mirror_update(MirrorStateKind::Authorization, failure_policy) {
                Ok(reservation) => pending.mirror_update = Some(reservation),
                Err(error) if failure_policy == MirrorFailurePolicy::BestEffort => {
                    warn!(
                        "event=mirror failed to reserve in-memory best-effort authorization {:?} update for uid={} pid={}: {:#}; preserving System mutation without an OMK mirror",
                        method, caller_uid, pending.caller.pid, error
                    );
                }
                Err(error) => {
                    warn!(
                        "event=mirror failed to reserve in-memory authorization {:?} update for uid={} pid={}: {:#}; blocking System mutation",
                        method, caller_uid, pending.caller.pid, error
                    );
                    block_system_request(tr);
                    replace_top_pending(
                        connection,
                        PendingCall::PrecomputedAuthorization(
                            pending,
                            Status::new_service_specific_error(ResponseCode::SYSTEM_ERROR.0, None),
                        ),
                    );
                    return true;
                }
            }
        }
        if expects_reply {
            replace_top_pending(connection, PendingCall::Authorization(pending));
        }
        return false;
    }

    // Maintenance calls mostly carry global keystore state, so mirror them after
    // system success. migrateKeyNamespace moves app keys, which now live in OMK,
    // so it always uses OMK as the authoritative business path.
    if request_interface == identify::KEYSTORE_MAINTENANCE_INTERFACE {
        let request = match parcel::parse_maintenance_request(
            data,
            data_size,
            offsets,
            offsets_size,
            tr.code,
        ) {
            Ok(request) => request,
            Err(error) => {
                debug!(
                    "event=decision failed to parse IKeystoreMaintenance request code=0x{:x}: {:#}",
                    tr.code, error
                );
                return false;
            }
        };

        let method = request.method();
        let route = route_for_maintenance_request(&request, caller.uid as u32);
        if !interface_ready {
            return if route == RouteTarget::Omk || maintenance_requires_mirror(&request) {
                block_unverified_interface(connection, tr, caller)
            } else {
                false
            };
        }
        let mut pending = PendingMaintenanceCall { request, caller, route, mirror_update: None };
        let mut precomputed = if pending.route == RouteTarget::Omk
            && matches!(&pending.request, ParsedMaintenanceRequest::MigrateKeyNamespace { .. })
        {
            match precompute_omk_migrate_reply(&pending) {
                OmkMigratePrecompute::Reply(reply) => {
                    block_system_request(tr);
                    Some(reply)
                }
                OmkMigratePrecompute::PreserveSystem => {
                    pending.route = RouteTarget::System;
                    None
                }
            }
        } else {
            None
        };
        let mut request_rewritten = precomputed.is_some();
        let route_note = match (&pending.request, pending.route) {
            (ParsedMaintenanceRequest::MigrateKeyNamespace { .. }, RouteTarget::Omk) => {
                "using OMK as the authoritative business path"
            }
            (ParsedMaintenanceRequest::MigrateKeyNamespace { .. }, RouteTarget::System) => {
                "system identity keeps migrateKeyNamespace on System"
            }
            _ => "mirroring maintenance state to OMK after system success",
        };
        info!(
            "event=decision command={} maintenance_method={:?} code=0x{:x} uid={} pid={} sid='{}' route={:?}; {}",
            command_name,
            method,
            tr.code,
            caller_uid,
            pending.caller.pid,
            pending.caller.sid,
            pending.route,
            route_note,
        );

        let requires_mirror =
            pending.route == RouteTarget::System && maintenance_requires_mirror(&pending.request);
        if requires_mirror && !expects_reply {
            warn!(
                "event=mirror cannot confirm one-way maintenance {:?} for uid={} pid={}; blocking System mutation",
                method, caller_uid, pending.caller.pid
            );
            block_system_request(tr);
            return true;
        }
        if requires_mirror {
            match reserve_mirror_update(
                MirrorStateKind::Maintenance,
                MirrorFailurePolicy::FailClosed,
            ) {
                Ok(reservation) => pending.mirror_update = Some(reservation),
                Err(error) => {
                    warn!(
                        "event=mirror failed to reserve in-memory maintenance {:?} update for uid={} pid={}: {:#}; blocking System mutation",
                        method, caller_uid, pending.caller.pid, error
                    );
                    block_system_request(tr);
                    precomputed = Some(PrecomputedMaintenanceReply::Error(
                        Status::new_service_specific_error(ResponseCode::SYSTEM_ERROR.0, None),
                    ));
                    request_rewritten = true;
                }
            }
        }
        if !expects_reply {
            return request_rewritten;
        }
        if let Some(reply) = precomputed {
            replace_top_pending(connection, PendingCall::PrecomputedMaintenance(pending, reply));
        } else {
            replace_top_pending(connection, PendingCall::Maintenance(pending));
        }
        return request_rewritten;
    }

    if request_interface == identify::KEYSTORE_SERVICE_INTERFACE {
        let request =
            match parcel::parse_service_request(data, data_size, offsets, offsets_size, tr.code) {
                Ok(request) => request,
                Err(error) => {
                    debug!(
                        "event=decision failed to parse IKeystoreService request code=0x{:x}: {:#}",
                        tr.code, error
                    );
                    return false;
                }
            };

        let method = request.method();
        let original_code = tr.code;
        if !interface_ready {
            return if service_request_needs_identity(&request, &cfg.intercept, caller.uid as u32) {
                block_unverified_interface(connection, tr, caller)
            } else {
                false
            };
        }
        let mut route = route_for_service_request(&request, &cfg.intercept, caller.uid as u32);

        let precomputed_service_reply = if route == RouteTarget::Omk
            && matches!(
                method,
                ServiceMethod::UpdateSubcomponent
                    | ServiceMethod::Grant
                    | ServiceMethod::Ungrant
                    | ServiceMethod::DeleteKey
            ) {
            match precompute_omk_service_mutator_reply(&request, &caller) {
                OmkServicePrecompute::Reply(reply) => {
                    block_system_request(tr);
                    Some(reply)
                }
                OmkServicePrecompute::PreserveSystem => {
                    info!(
                        "event=route method={:?} uid={} pid={} route={:?} omk_unavailable=true; preserving original system request",
                        method, caller.uid, caller.pid, route
                    );
                    route = RouteTarget::System;
                    None
                }
            }
        } else {
            None
        };
        let request_rewritten = precomputed_service_reply.is_some();

        info!(
            "event=decision command={} service_method={:?} code=0x{:x} uid={} pid={} sid='{}'",
            command_name, method, original_code, caller.uid, caller.pid, caller.sid,
        );
        info!(
            "event=route method={:?} uid={} pid={} route={:?}",
            method, caller.uid, caller.pid, route
        );

        let pending = PendingServiceCall { request, caller, route };
        if !expects_reply {
            if let Some(reply) = precomputed_service_reply.as_ref() {
                if let Err(error) = build_precomputed_service_reply(reply) {
                    warn!(
                        "event=route failed to build one-way OMK service {:?} reply for uid={} pid={}: {:#}",
                        method, caller_uid, pending.caller.pid, error
                    );
                }
                return request_rewritten;
            }
        }
        if expects_reply {
            if let Some(reply) = precomputed_service_reply {
                replace_top_pending(connection, PendingCall::PrecomputedService(pending, reply));
            } else {
                replace_top_pending(connection, PendingCall::Service(pending));
            }
        }
        return request_rewritten;
    }

    if request_interface == identify::KEYSTORE_SECURITY_LEVEL_INTERFACE {
        let Some(target_info) = tracker::lookup_security_level_target(target) else {
            debug!(
                "event=decision skipping IKeystoreSecurityLevel request for unmapped target ptr=0x{:x} cookie=0x{:x}",
                target.ptr, target.cookie
            );
            return false;
        };

        let request = match parcel::parse_security_level_request(
            data,
            data_size,
            offsets,
            offsets_size,
            tr.code,
        ) {
            Ok(request) => request,
            Err(error) => {
                debug!(
                    "event=decision failed to parse IKeystoreSecurityLevel request code=0x{:x}: {:#}",
                    tr.code, error
                );
                return false;
            }
        };

        let method = request.method();
        // Keystore2 shares each security-level Binder between getSecurityLevel and getKeyEntry.
        let intercept_enabled = security_level_intercept_enabled(&cfg.intercept);
        let route = route_for_security_level(&cfg.intercept, caller.uid as u32);

        info!(
            "event=decision command={} security_level_method={:?} code=0x{:x} uid={} pid={} sid='{}' target=ptr:0x{:x}/cookie:0x{:x} security_level={:?} intercept_enabled={}",
            command_name,
            method,
            tr.code,
            caller.uid,
            caller.pid,
            caller.sid,
            target.ptr,
            target.cookie,
            target_info.security_level,
            intercept_enabled,
        );
        info!(
            "event=route security_level_method={:?} uid={} pid={} route={:?} security_level={:?}",
            method, caller.uid, caller.pid, route, target_info.security_level
        );

        let mut pending = PendingSecurityLevelCall {
            request,
            caller,
            route,
            security_level: target_info.security_level,
        };
        if expects_reply && pending.route == RouteTarget::Omk {
            let reply = match build_omk_security_level_reply(&pending, true) {
                Ok(Some(reply)) => reply,
                Ok(None) => {
                    info!(
                        "event=route security-level {:?} OMK unavailable for uid={} pid={}; preserving original system request",
                        method, caller_uid, pending.caller.pid
                    );
                    pending.route = RouteTarget::System;
                    replace_top_pending(connection, PendingCall::SecurityLevel(pending));
                    return false;
                }
                Err(error) => {
                    warn!(
                        "event=route failed to precompute authoritative OMK security-level {:?} reply for uid={} pid={}: {:#}; returning SYSTEM_ERROR without executing System",
                        method, caller_uid, pending.caller.pid, error
                    );
                    synthetic_fallback_reply()
                }
            };
            block_system_request(tr);
            replace_top_pending(
                connection,
                PendingCall::PrecomputedSecurityLevel(pending, Box::new(Some(reply))),
            );
            return true;
        }
        if !expects_reply
            && route == RouteTarget::Omk
            && can_execute_one_way(SyntheticTargetKind::SecurityLevel, tr.code)
        {
            return handle_omk_one_way_security_level_request(tr, &pending);
        }
        if expects_reply {
            replace_top_pending(connection, PendingCall::SecurityLevel(pending));
        }
        return false;
    }

    if request_interface == identify::KEYSTORE_OPERATION_INTERFACE {
        let Some(operation_target) = lookup_operation_target(target) else {
            debug!(
                "event=decision skipping IKeystoreOperation request for unmapped target ptr=0x{:x} cookie=0x{:x}",
                target.ptr, target.cookie
            );
            return false;
        };

        let request = match parcel::parse_operation_request(
            data,
            data_size,
            offsets,
            offsets_size,
            tr.code,
        ) {
            Ok(request) => request,
            Err(error) => {
                debug!(
                    "event=decision failed to parse IKeystoreOperation request code=0x{:x}: {:#}",
                    tr.code, error
                );
                return false;
            }
        };

        let method = request.method();

        info!(
            "event=decision command={} operation_method={:?} code=0x{:x} uid={} pid={} sid='{}' target=ptr:0x{:x}/cookie:0x{:x}",
            command_name,
            method,
            tr.code,
            caller.uid,
            caller.pid,
            caller.sid,
            target.ptr,
            target.cookie,
        );
        info!(
            "event=route operation_method={:?} uid={} pid={} route={:?}",
            method, caller.uid, caller.pid, operation_target.route
        );

        let pending = PendingOperationCall { request, caller, target };
        if !expects_reply
            && operation_target.route == RouteTarget::Omk
            && can_execute_one_way(SyntheticTargetKind::Operation, tr.code)
        {
            return handle_omk_one_way_operation_request(tr, &pending);
        }
        if expects_reply {
            replace_top_pending(connection, PendingCall::Operation(pending));
        }
        return false;
    }

    debug!("event=decision skipping unsupported keystore interface request code=0x{:x}", tr.code);

    false
}

unsafe fn handle_omk_one_way_security_level_request(
    tr: &mut binder_transaction_data,
    pending: &PendingSecurityLevelCall,
) -> bool {
    match build_security_level_reply_rewrite(tr, pending) {
        Ok(Some(_)) => {
            block_system_request(tr);
            true
        }
        Ok(None) => false,
        Err(error) => {
            warn!(
                "event=route failed to execute one-way OMK security-level {:?} for uid={} pid={}: {:#}; consuming original system request",
                pending.request.method(), pending.caller.uid, pending.caller.pid, error
            );
            block_system_request(tr);
            true
        }
    }
}

unsafe fn handle_omk_one_way_operation_request(
    tr: &mut binder_transaction_data,
    pending: &PendingOperationCall,
) -> bool {
    if let Err(error) = build_operation_reply_rewrite(pending) {
        warn!(
            "event=route failed to execute one-way OMK operation {:?} for uid={} pid={}: {:#}; consuming original system request",
            pending.request.method(), pending.caller.uid, pending.caller.pid, error
        );
    }
    block_system_request(tr);
    true
}

fn block_system_request(tr: &mut binder_transaction_data) {
    // Keep the parcel intact; the interceptor owns any receive-buffer shadow.
    tr.code = u32::MAX;
}

#[cfg(test)]
mod tests;
