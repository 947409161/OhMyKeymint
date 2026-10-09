# Third-Party Software

## miuix-vue WebUI components

The embedded WebUI is built with the Vue 3 components from
[YuKongA/miuix-vue](https://github.com/YuKongA/miuix-vue), version `0.1.1`.
The library is licensed under the Apache License 2.0. Oh My Keymint uses its
navigation bar, top app bar, cards, preferences, sheets, dialogs, progress
indicators, and icons; application behavior and native bridge calls remain in
the Oh My Keymint source.

## Monet color generation

The WebUI bundles Google's [Material Color Utilities](https://github.com/material-foundation/material-color-utilities),
version `0.4.0`, under the Apache License 2.0. It supplies HCT palette generation
and the 2021/2025 color specifications; UI components remain miuix-vue.
The MIUIX role mapping in `webui/src/appearance.ts` is adapted from
[compose-miuix-ui/miuix MonetMapping.kt](https://github.com/compose-miuix-ui/miuix/blob/26b37993ce6073ac7cab9b986ab57352c126dc81/miuix-ui/src/commonMain/kotlin/top/yukonga/miuix/kmp/theme/MonetMapping.kt),
copyright 2025 compose-miuix-ui contributors, licensed under Apache-2.0.
System seed selection and the accent choices follow
[KernelSU's theme implementation](https://github.com/tiann/KernelSU/tree/85cab5f841b55bed180c10dfdbe33f876aba7820/manager/app/src/main/java/me/weishu/kernelsu/ui/theme).

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

## Google attestation status snapshot

The module includes the public JSON snapshot used to seed the local
`/data/misc/keystore/omk/data/google_attestation_status.json` cache on a first
install. Later successful HTTPS responses from Google's fixed attestation
endpoint replace that cache atomically after the complete response passes the
same schema and serial validation used for live checks. The snapshot contains
only Google's public `REVOKED`/`SUSPENDED` entries; it does not contain private
keys or device identifiers.

## Specter interface reference

The WebUI information architecture was checked against
[dpejoh/specter](https://github.com/dpejoh/specter) commit
[`829c4fa95ab5a08e4cd7e18dd686e73896d90a24`](https://github.com/dpejoh/specter/commit/829c4fa95ab5a08e4cd7e18dd686e73896d90a24).
That project is licensed under GPL-3.0. Oh My Keymint does not include or run
Specter's WebUI or shell scripts. Its WebUI and Rust implementation are
independent.

## Native HTTPS client

The security-patch WebUI action uses the Rust
[ureq](https://github.com/algesten/ureq) HTTP client (version 3.4.0), licensed
under the MIT or Apache License 2.0. Its HTTPS implementation uses
[rustls](https://github.com/rustls/rustls) and
[rustls-webpki](https://github.com/rustls/webpki), licensed under their
Apache-2.0/ISC/MIT and ISC terms respectively, together with
[webpki-roots](https://github.com/rustls/webpki-roots), licensed under
CDLA-Permissive-2.0. These dependencies are built into the existing `keymint`
helper; no device-provided `curl` or `wget` is used. Each action has a separate
exact host and path allowlist, and redirects are validated before another
request is made.
