# Vendored source

This directory vendors [ajfkdk/D-soter](https://github.com/ajfkdk/D-soter) at commit
`6148e02ea5977cb95b5a162a405fc915e39c01db` (`main`, 2026-07-10).

The payload is licensed under the Apache License 2.0; `LICENSE` is the upstream file
unmodified, and the build copies it into the module as
`THIRD_PARTY_LICENSES/d-soter-Apache-2.0.txt`.

One vendored header is not Apache-2.0: `module/jni/external/linux-kernel/include/android/binder.h`
carries `SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note` in its own first line.
The syscall note permits inclusion in non-GPL user-space code, but the licence text has to
travel with it, so `LICENSES/GPL-2.0-with-Linux-syscall-note.txt` reproduces both the
exception and GPL-2.0, and the build ships it as
`THIRD_PARTY_LICENSES/linux-syscall-note-GPL-2.0.txt`. The vendored
`module/jni/external/AOSP/include/**` headers are Apache-2.0 and are already covered by the
module's existing `AOSP.Apache-license-2.0.txt`.

Dropping the vendored kernel header would not remove the obligation: the NDK sysroot ships
a `linux/android/binder.h` under the same licence, and `Android.mk` deliberately keeps the
vendored copy in front of it (`LOCAL_C_INCLUDES` becomes `-I`, which clang searches before
the sysroot's `-isystem`), because that is the header upstream built and tested against.

## What was kept

Only files reachable from `module/jni/Android.mk` and its `LOCAL_C_INCLUDES`, plus the
prebuilt libraries it links against:

- `module/jni/{Android.mk, Application.mk, dsoter.cpp, zygisk.hpp}`
  — `dsoter.cpp` includes `"zygisk.hpp"` by bare name, resolved through
  `$(LOCAL_PATH)`, which is the first entry in `LOCAL_C_INCLUDES`.
- `module/jni/external/AOSP/` — binder and libutils headers.
- `module/jni/external/linux-kernel/include/android/binder.h` — kernel UAPI binder ioctl
  definitions.
- `prebuilts/arm64-v8a/{libbase,libbinder,libcutils,libutils}.so` — the prebuilt AOSP
  libraries named by `LOCAL_LDLIBS`. These are opaque binaries from upstream; they are
  vendored so builds do not depend on a live Git checkout.

The directory layout mirrors upstream exactly. `Android.mk` reaches the prebuilts via
`$(LOCAL_PATH)/../../prebuilts/arm64-v8a`, so moving `module/` or `prebuilts/` relative
to each other will break the link step.

## What was dropped, and why

- `module/jni/external/LSPlt/` — `dsoter.cpp` includes no LSPlt header and `Android.mk`
  compiles only `dsoter.cpp`, so none of it is reachable. Dropping it saves about 4 MB,
  most of which is a Doxygen tag file under `docs/`.
- `dist/` — upstream's own prebuilt module zip and `arm64-v8a.so`. OMK compiles the
  payload itself, so shipping a second copy would only invite the two to drift.
- All Gradle files (`build.gradle`, `settings.gradle`, `gradlew`, `gradlew.bat`,
  `gradle/`, `gradle.properties`, `module/build.gradle`) — upstream's Gradle project
  contains no Java or Kotlin sources and only wraps `ndk-build`, pinning
  `ndkVersion '27.3.13750724'`. OMK invokes `ndk-build` directly from `build.py`, so the
  Gradle wrapper adds a JDK and an AGP download for an identical result.
- `module.prop`, `README.md`, `.github/`, `.gitignore` — packaging and project metadata
  for upstream's standalone module. OMK bundles the payload inside its own module rather
  than shipping D-soter as a second one.

## Building

```
cd third_party/d-soter/module
"$ANDROID_NDK_ROOT/ndk-build" NDK_PROJECT_PATH=. NDK_APPLICATION_MK=jni/Application.mk
```

Output: `module/libs/arm64-v8a/libdsoter.so`. `build.py` renames it to
`zygisk/arm64-v8a.so` inside the module zip, which is where Zygisk loads it from.

`APP_ABI` is `arm64-v8a` only, which is why OMK no longer ships x86_64.

### Host requirement

`ndk-build` resolves its toolchain from the NDK's *host* prebuilts, and the NDK ships
`linux-x86_64` (plus the Darwin equivalents) only. On an aarch64 Linux host its launcher
stops with `Unknown host CPU architecture: aarch64` before any of its tools run.

`build.py` therefore prefers `ndk-build` and, when the host cannot execute it, falls back
to compiling `dsoter.cpp` with this machine's own compiler against the NDK sysroot
(`build_soter_zygisk_native`). The fallback mirrors `Android.mk` and `Application.mk`
flag for flag; its one deliberate difference is that it synthesises a `libgcc.a` under
`target/` from the NDK's `libclang_rt.builtins-aarch64-android.a`, because the host clang
driver still asks for `-lgcc` where the NDK's own driver would use compiler-rt.

The two paths are **not** byte-identical. The fallback produces a ~352 KB payload against
upstream's 404 KB — a different clang, and `-O2` applied explicitly rather than through
ndk-build's release default. The exported symbol set and the `DT_NEEDED` list match
upstream exactly, which is what makes the payload loadable, but `ndk-build` stays the
canonical path and is what CI uses. A release packaged on an aarch64 host is therefore
not bit-reproducible against one built on CI; prefer a CI artifact when that matters.

## Runtime behaviour

D-soter has no configuration file and no runtime switch. Once Zygisk loads the payload it
unconditionally hooks `ioctl(BINDER_WRITE_READ)` inside `com.tencent.soter.soterserver`
and forges Soter AIDL transaction replies. OMK's "Soter spoofing" switch therefore works
by renaming the payload out of `zygisk/` and requiring a reboot; see
`template/post-fs-data.sh`.
