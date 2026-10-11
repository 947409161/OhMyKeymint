# Third-Party Software

## D-Soter compatibility

The default-enabled Tencent Soter compatibility reply contract is adapted from
[D-soter](https://github.com/947409161/D-soter), revision
`37c6e6a3c099326892883be7ae7eeb1b2c5801d2`, `module/jni/dsoter.cpp`,
`module/jni/soter_attk.h`, `module/jni/soter_keys.h`, and `module/jni/soter_jni.h`,
licensed under Apache-2.0. Android's RSA provider generates independent RSA-2048
ASK and AuthKey pairs per caller UID and exact UTF-16 alias. The reference ATTK
signs ASK exports; each ASK signs its AuthKey exports with RSASSA-PSS (SHA-256,
MGF1-SHA256, salt length 20, `emBits` 2047). The `cpu_id` and device identifier
are derived from the unit serial. Keys, stable exports, and their lifecycle are
process-local.
The Rust implementation in `soter-spoof/src/soter.rs` and
`soter-spoof/src/soter/` implements all 13 reply contracts with a separate native
Binder stub and bounded descriptor matching. The platform validates the AIDL header
before argument parsing on Android 12-17, and `soter-spoof/src/attk.rs` implements
JNI key generation and the reference signing parameters. No upstream prebuilt
library or C++ runtime component is bundled.
It does not alter KeyMint or injector routing. The replies are software-generated mock
responses: they are not genuine attestation or payment credentials.

The release includes this attribution in `THIRD_PARTY_LICENSES/D-soter.txt` and
the Apache-2.0 terms in `AOSP.Apache-license-2.0.txt`.

## Native HTTPS client

Security-patch bulletin checks use the Rust
[ureq](https://github.com/algesten/ureq) HTTP client (version 3.4.0), licensed
under the MIT or Apache License 2.0. Its HTTPS implementation uses
[rustls](https://github.com/rustls/rustls) and
[rustls-webpki](https://github.com/rustls/webpki), licensed under their
Apache-2.0/ISC/MIT and ISC terms respectively, together with
[webpki-roots](https://github.com/rustls/webpki-roots), licensed under
CDLA-Permissive-2.0. These dependencies are built into the existing `keymint`
helper; no device-provided `curl` or `wget` is used. Each request has a separate
exact host and path allowlist, and redirects are validated before another
request is made.
