use super::*;

#[test]
fn update_subcomponent_preserves_empty_optional_blobs() {
    let key = blob_key_descriptor(None);
    let parsed = parse_service_request_with_payload(
        crate::android::system::keystore2::IKeystoreService::transactions::r#updateSubcomponent,
        |parcel| {
            parcel.write(&key).unwrap();
            parcel.write(&Some(Vec::<u8>::new())).unwrap();
            parcel.write(&Some(Vec::<u8>::new())).unwrap();
        },
    )
    .expect("empty updateSubcomponent blobs should parse");
    let ParsedServiceRequest::UpdateSubcomponent {
        public_cert,
        certificate_chain,
        ..
    } = parsed
    else {
        panic!("updateSubcomponent request should parse");
    };
    assert_eq!(public_cert, Some(Vec::new()));
    assert_eq!(certificate_chain, Some(Vec::new()));
}

#[test]
fn service_request_accepts_trailing_payload() {
    let app_key = KeyDescriptor {
        domain: crate::android::system::keystore2::Domain::Domain::APP,
        nspace: 0,
        alias: Some("alias".to_string()),
        blob: None,
    };
    let parsed = parse_service_request_with_payload(
        crate::android::system::keystore2::IKeystoreService::transactions::r#getKeyEntry,
        |parcel| {
            parcel.write(&app_key).unwrap();
            parcel.write(&0x4f4d4bi32).unwrap();
        },
    )
    .expect("service request with trailing payload should parse");
    let ParsedServiceRequest::GetKeyEntry { key } = parsed else {
        panic!("getKeyEntry request should parse as GetKeyEntry");
    };
    assert_eq!(key.alias.as_deref(), Some("alias"));
}

#[test]
fn service_key_descriptor_accepts_nonstandard_non_null_presence_flag() {
    use rsbinder::Parcelable;

    let key = KeyDescriptor {
        domain: crate::android::system::keystore2::Domain::Domain::APP,
        nspace: 0,
        alias: Some("alias".to_string()),
        blob: None,
    };
    let parsed = parse_service_request_with_payload(
        crate::android::system::keystore2::IKeystoreService::transactions::r#getKeyEntry,
        |parcel| {
            parcel.write(&2i32).unwrap();
            key.write_to_parcel(parcel).unwrap();
        },
    )
    .expect("nonstandard nonzero presence flag should parse");
    let ParsedServiceRequest::GetKeyEntry { key } = parsed else {
        panic!("getKeyEntry request should parse as GetKeyEntry");
    };
    assert_eq!(key.alias.as_deref(), Some("alias"));
}

#[test]
fn service_key_descriptor_rejects_null_presence_flag() {
    assert_status_code(
        parse_service_request_with_payload(
            crate::android::system::keystore2::IKeystoreService::transactions::r#getKeyEntry,
            |parcel| {
                parcel.write(&0i32).unwrap();
            },
        ),
        StatusCode::UnexpectedNull,
    );
}
