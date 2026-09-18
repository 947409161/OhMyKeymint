# Third-Party Software

## miuix-vue WebUI components

The embedded WebUI is built with the Vue 3 components from
[YuKongA/miuix-vue](https://github.com/YuKongA/miuix-vue), version `0.1.1`.
The library is licensed under the Apache License 2.0. Oh My Keymint uses its
navigation bar, top app bar, cards, preferences, sheets, dialogs, progress
indicators, and icons; application behavior and native bridge calls remain in
the Oh My Keymint source.

## Tricky Addon - Update Target List

The embedded Oh My Keymint WebUI contains adapted source from
[KOWX712/Tricky-Addon-Update-Target-List](https://github.com/KOWX712/Tricky-Addon-Update-Target-List)
at commit
[`cf167849aaa7696972a3c7826ec94294e9e47fce`](https://github.com/KOWX712/Tricky-Addon-Update-Target-List/commit/cf167849aaa7696972a3c7826ec94294e9e47fce).

That source is licensed under the
[Apache License 2.0](https://www.apache.org/licenses/LICENSE-2.0).

The adapted source supplies the offline package-selection portion of the Oh My
Keymint WebUI. It reads and replaces the OMK `scoop` list through the native
`inject` helper. That adapted portion does not provide network access, module
self-update, property modification, trust editing, or automatic WebUI-host
installation. Oh My Keymint separately provides a keybox action that validates
and atomically installs a local XML file. Its security-patch action downloads
Google's official Android Security Bulletin, updates the four `[trust]`
patch-level fields, applies the selected date to both runtime security-patch
properties with `resetprop`, and records the original values for restore. Its
separate restore action restores those properties and resets the four fields to
`auto` without network access.

## D-soter Zygisk payload

The Soter spoofing payload is the vendored Zygisk module
[ajfkdk/D-soter](https://github.com/ajfkdk/D-soter), pinned at commit
[`6148e02ea5977cb95b5a162a405fc915e39c01db`](https://github.com/ajfkdk/D-soter/commit/6148e02ea5977cb95b5a162a405fc915e39c01db).
That project is licensed under the
[Apache License 2.0](https://www.apache.org/licenses/LICENSE-2.0). Its source is
vendored under `third_party/d-soter/`, together with the prebuilt AOSP libraries
the payload links against, so nothing is downloaded at build time or at run
time. Oh My Keymint compiles it from source with the Android NDK's `ndk-build`,
invoked by `build.py`. Upstream's Gradle project is not used, because it only
wraps `ndk-build`.

The payload is loaded by the Zygisk loader into
`com.tencent.soter.soterserver`, where it hooks `ioctl(BINDER_WRITE_READ)` and
forges healthy Soter AIDL transaction replies. It has no configuration file and
no runtime switch; switching it off means renaming the packaged payload so the
loader no longer finds it, which only takes effect on the next boot. The
user-facing switch is described in the [Configuration Guide](CONFIGURATION.md).
The payload is built for `arm64-v8a` only.

One vendored header,
`module/jni/external/linux-kernel/include/android/binder.h`, is licensed under
`GPL-2.0 WITH Linux-syscall-note` rather than Apache-2.0. The build ships its
license text as `THIRD_PARTY_LICENSES/linux-syscall-note-GPL-2.0.txt`, and the
payload's own license as `THIRD_PARTY_LICENSES/d-soter-Apache-2.0.txt`.
`third_party/d-soter/UPSTREAM.md` records exactly what was kept, dropped, and
why.

## Google attestation status snapshot

The module includes the public JSON snapshot used to seed the local
`/data/misc/keystore/omk/data/google_attestation_status.json` cache on a first
install. Later successful HTTPS responses from Google's fixed attestation
endpoint replace that cache atomically after the complete response passes the
same schema and serial validation used for live checks. The snapshot contains
only Google's public `REVOKED`/`SUSPENDED` entries; it does not contain private
keys or device identifiers.

## Specter interface and ADB Disabler reference

The WebUI information architecture and ADB Disabler workflow were checked against
[dpejoh/specter](https://github.com/dpejoh/specter) commit
[`829c4fa95ab5a08e4cd7e18dd686e73896d90a24`](https://github.com/dpejoh/specter/commit/829c4fa95ab5a08e4cd7e18dd686e73896d90a24).
That project is licensed under GPL-3.0. Oh My Keymint does not include or run
Specter's WebUI or shell scripts. Its WebUI and Rust implementation are
independent; they reproduce only the documented ADB Disabler settings.

The ADB Disabler action follows Specter's documented settings: it independently
controls developer options, USB debugging, and OEM unlock, persists four strict
0/1 values under OMK's data directory, and reapplies the selected properties at
boot. It does not bundle Specter's shell scripts.

## Native HTTPS client

The security-patch synchronization and keybox revocation-check WebUI actions
use the Rust
[ureq](https://github.com/algesten/ureq) HTTP client (version 3.4.0), licensed
under the MIT or Apache License 2.0. Its HTTPS implementation uses
[rustls](https://github.com/rustls/rustls) and
[rustls-webpki](https://github.com/rustls/webpki), licensed under their
Apache-2.0/ISC/MIT and ISC terms respectively, together with
[webpki-roots](https://github.com/rustls/webpki-roots), licensed under
CDLA-Permissive-2.0. These dependencies are built into the existing `keymint`
helper; no device-provided `curl` or `wget` is used. Each action has a separate
exact host and path allowlist, and redirects are validated before another
request is made. These two features are the only WebUI actions that use the
network; Soter spoofing makes no network request.
