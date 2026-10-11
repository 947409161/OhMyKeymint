# Frequently Asked Questions

This page is for people who use OhMyKeymint (OMK) without needing to
understand Android internals. It describes the current behavior. If a setting
is not explained here, leave it at its default value. See the
[Configuration Guide](CONFIGURATION.md) for complete annotated examples and a
separate explanation of every field.

## What OMK does

### What is OhMyKeymint?

Android has a secure key service that apps use to create keys and check device
information. OMK provides an alternative version for resolved user and system
app identities while core Android services remain on the System route.

OMK only covers this part of Android. It is not a general root-hiding tool, a
Play Integrity module, or a repair tool for damaged security hardware.

### Will OMK make every banking app, payment app, or detector work?

No. Apps may check root, installed modules, system properties, certificates,
hooks, their own files, and many other signals that OMK does not control. A
good result in one test app, including a Play Integrity `STRONG` result, does
not guarantee that another app will accept the device.

If an app shows exactly the same warning with and without OMK, that warning
probably comes from something OMK does not control. If the message or behavior
changes between the two tests, the cause is still unknown; collect fresh logs
from both attempts instead of guessing.

### Which devices are supported?

The supported production range is:

- Android 12 through Android 17;
- `arm64-v8a` devices; and
- Linux kernels 4.14 through 6.18, plus newer LTS kernels.

Android 11 and older are not supported. An installer may run on another
version or architecture, but that does not make it an officially supported
setup.

### Does OMK add StrongBox to a phone that does not have it?

No. StrongBox is hardware. OMK cannot create it on a device that does not have
it. Apps that require real StrongBox may still reject such a device.

### Can I use OMK with other root or KeyStore modules?

Use only one root implementation. KernelSU and Magisk are the supported ways
to install OMK; recovery installation is not supported.

Do not run OMK together with another module that replaces Android's KeyStore
or KeyMint service. When two modules change the same service, results are
unpredictable and cannot be diagnosed reliably.

## Installing, updating, and removing

### How should I install OMK?

Download an official ZIP, keep it as a ZIP, and install it from the KernelSU
or Magisk app while Android is running. Reboot before testing it.

Do not extract, edit, or repack the ZIP. If the installer says that a file is
missing or verification failed, download the package again from the official
source.

### How should I update OMK?

Install the current release over the existing installation in the root manager,
then reboot. The update keeps the active KeyMint settings and stored OMK keys; Injector defaults ship in the updated binary.

### Is downgrading safe?

No.

### What should I back up?

Before testing a development build, or making a large
configuration change, privately back up:

- `/data/adb/modules/oh_my_keymint/config/config.toml`
- `/data/adb/modules/oh_my_keymint/config/keybox.xml`
- `/data/adb/modules/oh_my_keymint/config/data/`

Treat the backup like a password. It can contain private keys and device
identifiers. A backup is a safety measure, not a portable copy that is
guaranteed to work on another device.

### What happens if I disable or uninstall OMK?

Disable it in the root manager when you need to keep OMK's persistent data.
Keys created through OMK are not converted into normal System keys, so some
selected apps may ask you to sign in or register again after OMK is disabled.

When **Oh My Keymint is uninstalled from KernelSU**, the module directory and
its `config/` contents are removed. This permanently removes OMK configuration
and OMK-created keys; affected apps may need to be registered or signed in
again. Reboot after the uninstall. Disabling the module does not remove data.

## Routing apps and changing settings

### Which apps use OMK?

Injector routing is compiled into the Rust source and applies globally across
Android users and profiles. Caller package identities and system-app status
are read at runtime. Resolved user-installed apps are in scope; system apps,
core Android/system UIDs, `android` and `android.*` package identities, and
unresolved package identities are excluded. The hardcoded exceptions are
`com.google.android.gsf`, `com.google.android.gms`, and `com.android.vending`.

### Can I change the Injector routing settings on the device?

No. The Injector does not read an `injector.toml` file. Its package scope,
safety rules, enabled KeyStore methods, and timing values are compiled into the
Rust source. To change them, rebuild and update the module, then restart the
injector and reopen affected apps.

### Where are the active settings?

The active settings file is `/data/adb/modules/oh_my_keymint/config/config.toml`. Edit
that file rather than a copy in the module ZIP. The
[Configuration Guide](CONFIGURATION.md) explains its fields and when changes
take effect.

### How do I apply a KeyMint setting change?

KeyMint loads valid `config.toml` edits automatically where supported. Changes
to `[crypto]`, most `[trust]` or `[device]` values, and the KeyMint log level
require a KeyMint restart. After changing compiled Injector behavior, update
the module and restart the injector. After installing or updating the whole
module, reboot the device.

### What happens if I make a mistake in `config.toml`?

A malformed file is rejected. While KeyMint is running, the last valid settings
remain active; a malformed file at startup prevents KeyMint from starting.
Restore a valid backup and restart KeyMint. Do not delete `config.toml` as a
reset method: newly generated `[crypto]` values may make existing OMK keys
unusable.

## Keybox and stored keys

### Can I replace the keybox?

Edit `/data/adb/modules/oh_my_keymint/config/keybox.xml` and restart KeyMint.
Trusted runtime keybox updates are written to this file.

Never download private keyboxes from an untrusted source and never publish a
keybox that you use.

### What makes a keybox valid?

OMK requires at least one complete RSA or EC entry. Every private key present
must match its certificate chain. A keybox containing both algorithms uses the
signer that matches the attested key when available. A keybox containing only
one algorithm uses its sole key to sign all attestation leaf certificates,
including leaves whose public key uses the other algorithm.

RKP-extracted keyboxes are legitimately EC-only and do not need an added RSA
entry. RSA-only keyboxes are also supported without an added EC entry. RSA
private keys can use unencrypted PKCS#1 (`RSA PRIVATE KEY`) or PKCS#8
(`PRIVATE KEY`) PEM encoding. Both encodings use the same keybox identity when
the private key and certificate chain are unchanged; XML export uses PKCS#1.
EC private keys can use unencrypted SEC1 (`EC PRIVATE KEY`) or PKCS#8
(`PRIVATE KEY`) PEM encoding. Both normalize to SEC1 for identity hashing and
XML export. Adding or removing the PKCS#8 wrapper without changing the inner
SEC1 fields or certificate chain preserves the keybox identity. Missing inner
curve parameters are filled from the PKCS#8 algorithm identifier; other optional
SEC1 fields are not regenerated.
The file must also be clean XML, without watermarks, comments inserted
into key data, invisible characters, or other extra content.

A file working in another module does not prove that it is valid. Some other
tools accept damaged or incomplete XML that OMK correctly rejects.

### Why does a detector say that the certificate is expired or revoked?

That is a keybox problem, not a security-patch setting. Changing
patch-level settings cannot extend a certificate's lifetime or remove a
revocation. Use a currently valid, complete keybox and do not alter certificate
dates.

### Should I delete `config/data/`?

Not during a normal update, keybox change, or first troubleshooting attempt.
`/data/adb/modules/oh_my_keymint/config/data/` contains OMK-created keys. Deleting it can sign apps out, make
encrypted app data unreadable, or force identity verification.

Only reset it when you deliberately accept those consequences and have already
made a private backup. Resetting all stored keys is not a general repair step.

### Why did an app lose a key after routing changed?

Normal System keys and OMK keys are kept separately. Moving an app between the
two services does not copy its existing keys. The app may then report a missing key,
fail to decrypt data, or ask to register again.

Choose the app's routing before signing in or setting it up, then keep that
choice stable. If it has already changed, use the app's own sign-out, reset, or
re-registration process. Be aware that clearing app data can remove local
information.

## Troubleshooting

### What should I check first when OMK appears not to work?

Check these in order:

1. Use a current official release or an identifiable build requested for
   testing.
2. Confirm that the device is in the supported Android and architecture range.
3. Keep the default safety filters and do not run another KeyStore replacement.
4. Confirm the app package is resolved and the compiled filter allows its UID.
5. Reboot after installation or a module update.
6. Reproduce the problem once, then inspect the fresh injector and keymint
   logs from that same time.

Changing several settings at once makes the cause harder to find. Keep your
existing `config.toml` and every generated `[crypto]` value. Restore only the
compiled filter and intercept behavior, then change only KeyMint settings
needed for the reproduction.

### Why does a test app still show "Bootloader unlocked" after installation?

Usually the test app is still using the normal System service because its
package identity was unresolved, the injector did not start, or the active filter
rejected it. The test app's screen alone cannot tell which service answered.

After following the checklist above, inspect fresh `logcat -d -s OhMyKeymint`
output for the test app and an OMK routing result.

### Why do I see "No attestation keys", "TEE damaged", or a similar message?

These messages do not by themselves prove that the phone's hardware is
damaged. They can appear when the test app was not routed to OMK, injection
failed, or the active keybox was missing, invalid, expired, or revoked.

Confirm the app's package and the related logcat output from the same test. OMK
cannot restore factory hardware provisioning if the device's real TEE is
actually damaged.

### Why can an app behave differently when routed through OMK?

The app may be trying to use a System key that OMK does not have, or it may
depend on behavior outside Android's standard KeyStore rules. First account
for the routing change and any existing app keys.

Test only when you can safely sign in or register again. If the same failure
still occurs after a fresh app setup on a current OMK build, report it with
matching logs.

### Why does an app still report root or show a security warning?

OMK does not hide root. A warning can come from an app's own root detection,
package checks, system-property checks, a blocked certificate, or another
module. Passing a key-attestation test does not cancel those checks.

If the warning is identical when OMK is disabled for the test, investigate the
rest of the root setup. If it appears only when OMK handles the app, provide a
routed and unrouted comparison from the same app and build.

### Why do two detector apps disagree?

They may be testing different systems. Key attestation, Play Integrity,
app-specific root detection, and a displayed `sdkVersion` are not the same
thing.

Compare the exact test names instead of treating every "locked", "tampered",
or "secure" label as the same result. Use OMK's routing and keymint logs to
decide whether OMK handled the request.

### Why does the displayed KeyMint or Android version look unexpected?

Android version, KeyMint interface version, and attested OS version are
different values. The Android release alone does not determine the exposed
KeyMint version; that also depends on the device maker's system files.

In `config.toml`, `os_version = "auto"` detects the Android major each time the
keymint process starts, including the first start after a system-version
upgrade. An integer keeps that major fixed instead. KeyMint packs the resolved
value in AOSP `MMmmss` form, so fixed `os_version = 16` is reported as `160000`.
The OS patch level uses `YYYYMM`, while vendor and boot patch levels use
`YYYYMMDD`. OS and vendor source properties use AOSP date strings and are
converted before they are sent to the TA. Boot auto mode can also preserve a
decimal bootloader wire value, including a legacy header value whose missing
day is `00`. These different widths and representations are expected.

If a service response omits a field such as `sdkVersion`, do not assume that
the KeyMint version explains it. The cause may still be unknown. Do not force a
setting merely to change a label; if the result causes a real failure, report
the exact field and matching logs.

### Are empty IMEI2 or MEID values a problem?

Not necessarily. A device can legitimately have one IMEI, no second IMEI, no
MEID, or no telephony identifiers. Leave automatic discovery enabled and do
not invent values just to fill empty fields.

An app should fail for a missing identifier only when it specifically asks for
that identifier. Empty optional fields do not invalidate the other device
information.

### Why can fingerprint authentication still fail?

The app may require the lock-screen PIN or password instead of a fingerprint,
or the authorization may have expired before the operation started. A log such
as `KEY_USER_NOT_AUTHENTICATED` does not always mean OMK is broken.

Authenticate in the way the app requests and retry promptly. Keep
`force_skip_system_biometric_hat_verification` at `false` unless a maintainer
is diagnosing a confirmed device-specific compatibility problem.

### Why do logs show `KEY_NOT_FOUND`, RSA errors, or repeated failures?

A missing key is an expected error when an app asks for a key that was deleted
or was created through the other service. Decryption errors can also mean that
the app supplied data that does not match the key. An error line is not proof
that OMK crashed.

Look for a real user-visible failure at the same time. Do not erase all OMK
keys to silence the log. If a current build fails with a newly created key and
a normal app flow, include the full matching operation in a bug report.

### Does every `WARN` or `ERROR` line mean something is broken?

No. Best-effort device probes, expired authentication attempts, missing
keys, and rejected invalid requests may be logged even when OMK is working as
intended. Judge the line together with the app behavior and nearby log entries.

A report is useful when it contains a repeatable symptom and logs from the
same time, not just an isolated word such as `ERROR`.

### What should I do if Android cannot unlock or the user interface does not start?

Restore the default module build, especially the compiled app scope and safety filter,
then reboot. If you allowed Android packages, system services, or unknown
callers, undo that change first.

If Android cannot start normally, use the root manager's documented safe mode
or module-disable method to disable OMK. Do not keep testing a broad
interception configuration on important data.

### Where can I read logs?

OMK does not write log files. Collect Android logcat output with
`logcat -d -s OhMyKeymint` after noting the exact reproduction time. The
KeyMint log level is configured in `config.toml`; Injector logging uses a fixed
level.

### What should I include in a bug report?

Use the
[bug report form](https://github.com/qwq233/OhMyKeymint/issues/new?template=bug_report.yml)
and include:

- the full release tag or build identity, not just "latest";
- Android version, ROM, kernel, root framework and version, and ABI;
- the affected app's package name;
- only the relevant, redacted settings;
- the shortest repeatable steps and exact reproduction time;
- matching keymint, injector, and `logcat` output; and
- a same-build comparison with and without OMK routing when the problem affects
  one app.

Screenshots alone are rarely enough. Do not upload the whole OMK data folder.

### Which information must never be shared?

Never publish:

- keybox private keys or an unredacted `keybox.xml`;
- any value under `[crypto]`;
- the OMK key database;
- IMEI, IMEI2, MEID, device serials, account details, or other personal data;
  or

If a maintainer needs information above, share it with proper measures.

## Licensing and help

### Is commercial use allowed?

No. Both the [GNU AGPL version 3 or later](../LICENSE.md) and the
[Oh My Keymint License](../LICENSE-2) apply together. The additional license
prohibits commercial use and contains other conditions. Read both before using
or distributing OMK.

### Where should I ask for help?

Use the
[Question form](https://github.com/qwq233/OhMyKeymint/issues/new?template=question.yml)
for a usage question, the
[Bug report form](https://github.com/qwq233/OhMyKeymint/issues/new?template=bug_report.yml)
for a repeatable defect, and the
[Feature request form](https://github.com/qwq233/OhMyKeymint/issues/new?template=feature_request.yml)
for a proposed change. Read this FAQ and search existing reports first.

For community discussion, use the Telegram link in the [README](../README.md).
Never share private keys, generated seeds, or personal identifiers in any
support channel.
