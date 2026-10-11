# Configuration Guide

OhMyKeymint (OMK) uses two active configuration files:

- `/data/adb/modules/oh_my_keymint/config/config.toml` controls the KeyMint service, the
  identity it reports, and the secrets used for OMK-created keys.
- `/data/adb/modules/oh_my_keymint/config/keybox.xml` supplies the attestation
  keybox loaded when KeyMint starts. KeyMint updates this file when it receives
  a replacement keybox from its trusted provisioning path.

Injector settings are compiled into the Rust source. It reads caller package
identities at runtime and globally includes resolved user-installed apps
across Android users and profiles. It excludes system apps, blocks core
Android identities, rejects unknown package identities, and hardcodes Google
Services Framework, Google Play services, and Play Store as exceptions.

**Jump to:** [`config.toml`](#configtoml) | [Injector routing](#injector-routing)

## Before editing

For ordinary use, change only the documented `config.toml` settings you need.
Keep generated `[crypto]` values unchanged.

Before making a change:

1. Make a private backup of the active file.
2. Edit `/data/adb/modules/oh_my_keymint/config/config.toml`, not a copy in the module ZIP.
3. Keep ordinary strings inside quotes and booleans as `true` or `false`.
4. Change one thing at a time, save the complete file, and check the matching
  Android logcat after the change.

Never publish `[crypto]` values, IMEI, IMEI2, MEID, serial numbers, or an
unredacted copy of either active file.

## How changes are loaded

KeyMint watches its active file for valid changes. A valid `config.toml` is
read automatically. The four patch-level fields, biometric compatibility
switch, and three TA delay ranges can take full effect without restarting
KeyMint. The field reference below states when a restart is required.

A malformed file saved while KeyMint is running is rejected and the last valid
in-memory configuration remains active. A malformed `config.toml` present
when KeyMint starts prevents KeyMint from starting. Fix the file and restart
KeyMint.

If `config.toml` is missing when KeyMint starts, OMK creates a new file with
generated defaults. This is not a safe way to reset a working setup:
regenerated secrets do not restore keys protected by the previous secrets.
The restart commands are documented in
[Restarting keymint and injector](../README.md#restarting-keymint-and-injector).
After changing app routing, close and reopen the affected app to avoid mixing
an already-open operation with the new route. After changing the compiled
Injector configuration, rebuild and update the module, then restart the
injector and reopen the affected app.

## `config.toml`

### Complete annotated example

The values under `[crypto]` below are deliberately non-working redaction
placeholders. A real active file contains unique generated hexadecimal values.
Never paste the placeholder values into a device and never replace the values
already present in a working file. The `[trust]` and `[device]` values are also
examples; keep the values from the active file unless you intend to change the
reported identity.

```toml
# Configuration format. Keep this at 2.
version = 2

[main]
# The supported service connection. Keep this value unchanged.
backend = "injector"
# KeyMint log detail: off, error, warn, info, debug, or trace.
log_level = "debug"
# Insecure biometric compatibility switch. Keep false for normal use.
force_skip_system_biometric_hat_verification = false
# Extra random wait per software TA call: inclusive [minimum, maximum] in ms.
# These defaults also apply when fields are absent; [0, 0] disables a category.
ta_operation_delay_ms = [9, 21]
ta_generation_delay_ms = [6, 16]
ta_control_delay_ms = [1, 4]

[crypto]
# Redacted placeholders only. Keep the generated 64-character values.
root_kek_seed = "KEEP_THE_VALUE_FROM_THE_ACTIVE_FILE"
kak_seed = "KEEP_THE_VALUE_FROM_THE_ACTIVE_FILE"
shared_secret_seed = "KEEP_THE_VALUE_FROM_THE_ACTIVE_FILE"
shared_secret_nonce = "KEEP_THE_VALUE_FROM_THE_ACTIVE_FILE"
# Optional expert override. Normally leave this line absent.
# auth_token_hmac_key = "KEEP_THE_VALUE_FROM_THE_ACTIVE_FILE"

[trust]
# Detect the Android major at each keymint start; use an integer to fix it.
os_version = "auto"
# Use auto, latest, or an exact YYYY-MM-DD date; boot also accepts decimal u32.
security_patch = "auto"
os_patchlevel = "auto"
vendor_patchlevel = "auto"
boot_patchlevel = "auto"
# Use auto, random, or exactly 64 hexadecimal characters.
vb_key = "auto"
vb_hash = "auto"
# Report verified boot and a locked bootloader when true.
verified_boot_state = true
device_locked = true

[device]
# Device identity strings reported when an app requests attestation IDs.
brand = "KEEP_THE_VALUE_FROM_THE_ACTIVE_FILE"
device = "KEEP_THE_VALUE_FROM_THE_ACTIVE_FILE"
product = "KEEP_THE_VALUE_FROM_THE_ACTIVE_FILE"
manufacturer = "KEEP_THE_VALUE_FROM_THE_ACTIVE_FILE"
model = "KEEP_THE_VALUE_FROM_THE_ACTIVE_FILE"
serial = "KEEP_THE_VALUE_FROM_THE_ACTIVE_FILE"
# false fills only empty telephony fields from the device when available.
overrideTelephonyProperties = false
# Empty optional identifiers are valid; do not invent missing values.
meid = ""
imei = ""
imei2 = ""
```

### Top-level field

#### `version`

This identifies the configuration format. The supported value is the integer
`2`. It is not an Android version or an OMK release number. Do not increment it;
the current file should keep this value unchanged. Live reload rejects other
values and keeps the last valid runtime configuration.

At keymint startup, a missing `version` is treated as `0`. Versions `0` and `1`
are migrated in place to `2` before the service starts, and `os_version` is set
to `"auto"` so later Android upgrades are detected on the next keymint start.
Startup also removes the obsolete `trust_record`. For version `0`, missing
patch-level fields inherit the configured `security_patch`. Other configured
and unknown values are preserved. Migration is not performed during live
reload, so restart keymint to migrate an older file. An unsupported future
version is never overwritten.

### `[main]`

#### `backend`

Use `"injector"`. It is the only supported user choice and there is no
alternative runtime backend to select, so this field should be left unchanged.

#### `log_level`

This controls messages written by keymint. Use one of `"off"`, `"error"`,
`"warn"`, `"info"`, `"debug"`, or `"trace"`. `"debug"` is the default and is
the most useful level for a bug report. `"trace"` is more verbose; `"off"`
suppresses normal logging.

Changing this field requires a keymint restart. An unrecognized value falls
back to `debug`, but relying on that fallback can hide a spelling mistake.

#### `force_skip_system_biometric_hat_verification`

This is an insecure compatibility switch for a device whose System KeyMint
cannot verify biometric authentication tokens correctly. When `true`, OMK
accepts a token whose structure is valid without asking System KeyMint to
verify its authentication code.

Keep it `false` unless a maintainer is diagnosing a confirmed device-specific
problem. It does not hide root and is not a general fix for fingerprint or
lock-screen failures. A valid save applies to new checks without restarting
keymint.

#### TA delay ranges

These three `[main]` fields configure extra random waits before OMK dispatches
calls to its software TA:

| Field | Default range (ms) | Covered TA calls |
| --- | --- | --- |
| `ta_operation_delay_ms` | `[9, 21]` | `begin`, `updateAad`, `update`, `finish`, `abort`, `getKeyCharacteristics` |
| `ta_generation_delay_ms` | `[6, 16]` | `generateKey`, `importKey`, `importWrappedKey`, `upgradeKey`, `convertStorageKeyToEphemeral` |
| `ta_control_delay_ms` | `[1, 4]` | `getHardwareInfo`, `addRngEntropy`, `deleteKey`, `deleteAllKeys`, `destroyAttestationIds`, `earlyBootEnded`, `getRootOfTrustChallenge`, `getRootOfTrust`, `sendRootOfTrust`, `setAdditionalAttestationInfo` |

Each value must be an array of exactly two integers, `[minimum, maximum]`,
with `0 <= minimum <= maximum <= 250`. Both endpoints are milliseconds and
are inclusive. `[0, 0]` disables that category; equal nonzero endpoints request
a fixed wait. Omitting a field enables its default range from the table.

For every covered TA invocation, OMK samples a new wait uniformly at
microsecond granularity between the configured endpoints. Calls in a category
share the same range, but do not reuse one sample for the category or thread.
The wait is added to processing time; it is neither a target total duration
nor a cap. Scheduling can extend the wait beyond the selected value, and one
app request that makes several covered TA calls accumulates their waits.

The wait occurs before TA dispatch and before acquiring the TA mutex, with the
configuration read guard released. The caller's Binder worker, RPC connection,
per-operation concurrency guard, or database guard can remain occupied during
the wait. Concurrent requests can therefore delay one another. A dispatched
call receives its wait even if the TA returns an error; validation failures
that return before TA dispatch do not. Raw TA initialization, local cache and
authentication helpers, and calls routed to System are excluded.

Valid changes apply to new TA calls without restarting keymint. A malformed
array, non-integer value, out-of-range endpoint, or reversed range rejects the
configuration; live reload keeps the previous valid configuration. Edit these
fields in the existing active file while preserving its other values. OMK
generates `config.toml` on the device; the module does not ship a reusable main
configuration template.

These waits are independent of the compiled Injector delays. The Injector
adds a sampled 20-30 ms wait before challenged generation and no extra wait
before operation starts. Changing software timing does not provide hardware
security, constrain total calls to a hardware timing interval, or guarantee a
detector result.

### `[crypto]`

Every value in this section is private. Each value is exactly 32 bytes written
as 64 hexadecimal characters using `0-9` and `a-f`. OMK generates these values
when it creates a new configuration.

Keep all four generated seed and nonce fields present and stable, and back them
up privately with the OMK data. Changing or removing any of them requires a
keymint restart and can make existing keys or authentication-bound operations
unusable. Values from another device and the redacted placeholders in this
guide are not replacements for the active values.

If `shared_secret_seed` or `shared_secret_nonce` is missing, OMK generates a
new random replacement when it reads the file. That is not a stable active
configuration, so make sure both generated values remain present.

#### `root_kek_seed`

This seed is used to derive key material that protects OMK key blobs. If it
changes, OMK may no longer be able to open keys created with the previous
value. It must be present and must remain unchanged.

#### `kak_seed`

This seed is used for OMK's key-agreement protection. It belongs to the same
device-specific secret set as `root_kek_seed`. It must be present and must
remain unchanged.

#### `shared_secret_seed`

This is the seed half of the shared-secret parameters used for authentication
token verification. Preserve it together with `shared_secret_nonce`; changing
only one side still changes the resulting secret.

#### `shared_secret_nonce`

This is the nonce half of the shared-secret parameters. It is also a full
64-character hexadecimal value, not a short counter or a value to regenerate
manually.

#### `auth_token_hmac_key`

This optional field supplies an explicit authentication-token HMAC key.
When it is absent, OMK derives the required key through
`shared_secret_seed` and `shared_secret_nonce`. Ordinary users should leave the
field absent. If it is explicitly present, it must also contain exactly 64
hexadecimal characters and must be kept private and stable.

### `[trust]`

These fields control values reported through key attestation. They do not
repair hardware, renew a certificate, remove a keybox revocation, or hide root.

#### `os_version`

Use `"auto"` to detect the current Android major each time the keymint process
starts, or use an integer from `0` through `99` to keep a fixed major such as
`12`, `16`, or `17`. Do not write a dotted release, SDK number, or security
patch date here. KeyMint encodes the resolved major with the AOSP `MMmmss`
formula, so fixed `16` is reported as `160000`. Changing this field requires a
keymint restart.

#### `security_patch`

This controls `ro.build.version.security_patch`. It accepts:

- `"auto"`: use the current `ro.build.version.security_patch` value without
  writing it;
- `"latest"`: use the fifth day of the current calendar month when the value
  is resolved; or
- an actual date written as `"YYYY-MM-DD"`, including leading zeroes.

`"auto"` first uses a nonempty runtime property, then the exact key from the
standard `build.prop` locations, and finally `2025-06-05` if neither source is
available. A present runtime value is used as-is rather than replaced by a
`build.prop` value. `"latest"` and an exact date intentionally overwrite an
existing runtime property, but OMK never creates or deletes it. `"auto"` never
writes the property. After an explicit or `"latest"` override, manually
switching back to `"auto"` in the same boot keeps the current runtime value;
reboot to restore the system-provided value.

#### `os_patchlevel`

This controls the KeyMint OS patch level. `"auto"` follows the effective
`security_patch`; `"latest"` and an exact `"YYYY-MM-DD"` date override it for
KeyMint without writing another property. The final value is parsed with the
AOSP `YYYY-MM-DD` parser and encoded as `YYYYMM`.

#### `vendor_patchlevel`

This controls the KeyMint vendor patch level. `"auto"` first reads the nonempty
runtime `ro.vendor.build.security_patch`, then the exact key from the standard
`build.prop` locations, and finally falls back to the effective
`os_patchlevel`. `"latest"` and an exact `"YYYY-MM-DD"` date are also accepted.
The final value is parsed with the AOSP `YYYY-MM-DD` parser and encoded as
`YYYYMMDD`. A present nonempty source is not replaced by a lower-priority source
merely because parsing later fails. Normal patch-level resolution does not write the vendor property.

#### `boot_patchlevel`

This controls the KeyMint boot patch level. `"auto"` first reads
`com.android.build.boot.security_patch` from the active top-level vbmeta image.
If the property is absent, OMK reads the same property from the active boot
image's standalone or AVB-footer embedded vbmeta, then falls back to the boot
header. The legacy header field stores a year and month but no day, so its wire
value ends in `00`; an all-zero field therefore becomes `20000000`, but only
after neither vbmeta location supplied the property. If boot-metadata resolution
fails, OMK uses this fallback order: nonempty runtime
`ro.vendor.boot_security_patch`, the exact key from the standard `build.prop`
locations, then the effective `os_patchlevel`.

`"latest"`, an exact `"YYYY-MM-DD"` date, and a decimal `u32` wire value are
also accepted. The decimal form preserves bootloader wire values such as
`"20000000"` without interpreting them as dates. These explicit modes do not
read boot metadata. Boot patch-level resolution does not read the system TEE or
write the boot property. During hot reload, an unchanged `"auto"` keeps the
value resolved before keymint dropped privileges; switching from an override
back to `"auto"` takes effect after keymint restarts. Explicit dates are encoded
as `YYYYMMDD`.
If the selected value cannot be converted, startup fails; a failed hot update
keeps the previous runtime configuration.

The four patch-level fields are resolved and applied together while keymint is
running when no other `[trust]` field changes in the same save. Existing TAs are
updated in place so in-flight operations and per-boot counters remain intact;
equivalent boot representations do not trigger an update. If the matching log
reports that the live update failed, restart keymint.

#### `vb_key`

This controls the 32-byte verified-boot public-key digest:

- `"auto"` first reads `ro.boot.vbmeta.public_key_digest`, then tries to
  calculate the top-level vbmeta key digest, and uses a random fallback only if
  neither source is available;
- `"random"` generates a new value whenever keymint starts; or
- a 64-character hexadecimal string pins an exact value.

Keep `"auto"` unless you understand the attestation profile being configured.
Changing this field requires a keymint restart. If `"random"` was active and
you change it back to `"auto"`, reboot the whole device so Android restores the
original boot property before `"auto"` reads it.

#### `vb_hash`

This controls the 32-byte verified-boot hash:

- `"auto"` first reads `ro.boot.vbmeta.digest`, then tries the original System
  attestation hash, and uses a random fallback only if neither source is
  available;
- `"random"` generates a new value whenever keymint starts; or
- a 64-character hexadecimal string pins an exact value.

The same restart rule applies as for `vb_key`: restart keymint after a normal
change, and reboot the whole device when returning from `"random"` to
`"auto"`.

#### `verified_boot_state`

`true` reports the verified boot state as verified; `false` reports it as
unverified. This is independent of the `device_locked` switch. Changing it
requires a keymint restart.

#### `device_locked`

`true` reports that the device boot state is locked; `false` reports it as
unlocked. This does not actually lock or unlock the bootloader. Changing it
requires a keymint restart.

### `[device]`

This section supplies device identity strings when an app explicitly requests
attestation IDs. These values are personal data. Use the values already
generated for the device, and restart keymint after changing this section so
the one-shot attestation-ID snapshot is rebuilt.

#### `brand`

The product brand reported in an attestation ID request, normally based on
`ro.product.brand` when a new configuration is created.

#### `device`

The device code name reported in an attestation ID request, normally based on
`ro.product.device`.

#### `product`

The product name reported in an attestation ID request, normally based on
`ro.product.name`.

#### `manufacturer`

The manufacturer name reported in an attestation ID request, normally based on
`ro.product.manufacturer`.

#### `model`

The model name reported in an attestation ID request, normally based on
`ro.product.model`.

#### `serial`

The device serial reported in an attestation ID request, normally based on
`ro.serialno`. Treat it as private information and redact it from reports.

#### `overrideTelephonyProperties`

With the recommended value `false`, OMK tries to fill only empty `imei`,
`imei2`, and `meid` fields from the device's telephony services and property
fallbacks. A configured non-empty value is preserved, and OMK attempts to write
each successfully discovered value back to the active `config.toml`.

With `true`, OMK skips telephony discovery and uses the three configured fields
exactly as written, including empty strings. Use this only when intentionally
pinning the values.

#### `imei`

The primary IMEI. Leave it empty when the device has no IMEI or when automatic
discovery should fill it. Do not invent a value to satisfy an app.

#### `imei2`

The second IMEI. Single-SIM and some dual-SIM devices can legitimately leave it
empty. Its absence does not invalidate `imei` or the non-telephony device
fields.

#### `meid`

The MEID used by devices that provide one. Many devices have no MEID, so an
empty value is valid. Its absence does not invalidate an available IMEI.

### `config.toml` apply summary

| Fields | Required action |
| --- | --- |
| `[main].log_level` | Restart keymint. |
| `[main].force_skip_system_biometric_hat_verification` | Applies to new checks after a valid save. |
| `[main].ta_operation_delay_ms`, `ta_generation_delay_ms`, `ta_control_delay_ms` | Apply to new TA calls after a valid save; invalid ranges retain the previous configuration. |
| All `[crypto]` fields | Restart keymint; changing values can make keys unusable. |
| `[trust].security_patch`, `os_patchlevel`, `vendor_patchlevel`, `boot_patchlevel` | Hot-apply as a group when no other `[trust]` field changes; otherwise restart keymint. |
| `[trust].os_version` | Restart keymint. |
| Other `[trust]` fields | Restart keymint. |
| All `[device]` fields | Restart keymint to rebuild the cached ID snapshot. |
| `vb_key` or `vb_hash` from `"random"` to `"auto"` | Reboot the whole device. |

## Tencent Soter compatibility

The D-Soter compatibility simulation is enabled by default for
`com.tencent.soter.soterserver`. It has no configuration file. Restart the
Soter service process after updating the module.

## Injector routing

Injector configuration is compiled into the Rust source. The global defaults
enable all KeyStore interception methods, read caller package identities at
runtime, include resolved user-installed apps across Android users and
profiles, reject unknown package identities, and block core Android/system
UIDs and `android`/`android.*` package identities. System apps are excluded
except for the hardcoded `com.google.android.gsf`,
`com.google.android.gms`, and `com.android.vending` exceptions. The deny list
is empty.

There is no configurable package whitelist to maintain. To change Injector
behavior, edit `injector/src/config.rs`, rebuild the module, update it on the
device, and restart the injector. The Injector does not read, create, or watch
`injector.toml`.
