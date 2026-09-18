#!/usr/bin/env python3
"""
Build script for OhMyKeymint Android targets.
"""

from __future__ import annotations

import argparse
import glob
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import zipfile

try:
    import tomllib as toml
except ModuleNotFoundError:
    import toml


REPO_ROOT = Path(__file__).resolve().parent
TARGET_ROOT = REPO_ROOT / "target"
DEFAULT_PLATFORM = 24

ABI_TO_TARGET = {
    "arm64-v8a": "aarch64-linux-android",
}

BORINGSSL_BUILD_DIRS = {
    "aarch64-linux-android": Path.home() / ".cargo" / "boringssl" / "build",
}

BINARY_SPECS = (
    {"package": None, "bin": "keymint", "output_name": "keymint"},
    {"package": "injector", "bin": "inject", "output_name": "inject"},
)
SOTER_ZYGISK_SOURCE_DIR = REPO_ROOT / "third_party" / "d-soter" / "module"
SOTER_ZYGISK_LIBRARY = "libdsoter.so"
# Mirrors module/jni/Android.mk and module/jni/Application.mk. The native-compiler
# fallback below rebuilds these by hand, so they must be kept in step with the
# vendored files; third_party/d-soter/UPSTREAM.md records the mapping.
SOTER_ZYGISK_SOURCE_NAME = "dsoter.cpp"
SOTER_ZYGISK_API = 29
SOTER_ZYGISK_INCLUDE_DIRS = ("external/AOSP/include", "external/linux-kernel/include")
SOTER_ZYGISK_COMPILE_FLAGS = (
    "-shared",
    "-fPIC",
    "-O2",
    "-std=c++20",
    "-fexceptions",
    "-fno-rtti",
    "-Wall",
    "-Wextra",
    "-Wno-unused-parameter",
    "-Wno-deprecated-declarations",
    '-DLOG_TAG="D-Soter"',
    "-DBINDER_DISABLE_NATIVE_HANDLE",
)
SOTER_ZYGISK_LINK_LIBS = ("-llog", "-ldl", "-lbinder", "-lutils", "-lcutils", "-lbase")
SOTER_ZYGISK_LICENSE = REPO_ROOT / "third_party" / "d-soter" / "LICENSE"
SOTER_ZYGISK_SYSCALL_NOTE = (
    REPO_ROOT
    / "third_party"
    / "d-soter"
    / "LICENSES"
    / "GPL-2.0-with-Linux-syscall-note.txt"
)

REQUIRED_TEMPLATE_FILES = (
    "customize.sh",
    "daemon",
    "daemon-injector",
    "injector.toml",
    "module.prop",
    "uninstall.sh",
    "post-fs-data.sh",
    "service.sh",
    "verify.sh",
    "webroot",
)

MODULE_TEXT_FILES = (
    "AOSP.Apache-license-2.0.txt",
    "LICENSE-2",
    "LICENSE.md",
    "README.md",
    "THIRD_PARTY_LICENSES/Tricky-Addon-Update-Target-List.txt",
    "THIRD_PARTY_LICENSES/d-soter-Apache-2.0.txt",
    "THIRD_PARTY_LICENSES/linux-syscall-note-GPL-2.0.txt",
    "customize.sh",
    "daemon",
    "daemon-injector",
    "injector.toml",
    "keybox.xml",
    "google_attestation_status.json",
    "module.prop",
    "uninstall.sh",
    "post-fs-data.sh",
    "sepolicy.rule",
    "service.sh",
    "verify.sh",
    "META-INF/com/google/android/update-binary",
    "META-INF/com/google/android/updater-script",
)


def run(cmd: list[str], *, env: dict[str, str] | None = None) -> None:
    print("+", " ".join(cmd))
    result = subprocess.run(cmd, cwd=REPO_ROOT, env=env)
    if result.returncode != 0:
        raise RuntimeError(f"command failed: {' '.join(cmd)}")


def get_version_from_cargo_toml() -> str:
    with (REPO_ROOT / "Cargo.toml").open("r", encoding="utf-8") as fh:
        cargo_toml = toml.loads(fh.read())
    return cargo_toml["package"]["version"]


def get_git_commit_count() -> str:
    result = subprocess.run(
        ["git", "rev-list", "--count", "HEAD"],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        raise RuntimeError("Failed to get git commit count")
    git_count = result.stdout.strip()
    if not git_count.isdigit():
        raise ValueError(f"Git commit count must be numeric only, got: {git_count}")
    return git_count


def get_git_commit_hash() -> str:
    result = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        raise RuntimeError("Failed to get git commit hash")
    git_hash = result.stdout.strip()[:7]
    status = subprocess.run(
        ["git", "status", "--porcelain", "--untracked-files=normal"],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
    )
    if status.returncode != 0:
        raise RuntimeError("Failed to inspect git worktree state")
    return f"{git_hash}-dirty" if status.stdout.strip() else git_hash


def cargo_context_for_target(target: str) -> tuple[dict[str, str], str]:
    env = os.environ.copy()
    configured_dir = env.get("BORINGSSL_BUILD_DIR")
    if configured_dir:
        build_dir = Path(configured_dir).expanduser()
        if not build_dir.is_absolute():
            build_dir = REPO_ROOT / build_dir
    else:
        build_dir = BORINGSSL_BUILD_DIRS.get(target)
        if build_dir is None:
            raise ValueError(f"No default BoringSSL build directory for target {target}")

    build_dir = build_dir.resolve()
    source_dir = build_dir.parent
    bssl_sys_dir = source_dir / "rust" / "bssl-sys"
    required_paths = (
        bssl_sys_dir / "Cargo.toml",
        build_dir / "libcrypto.a",
        build_dir / "libssl.a",
        build_dir / "rust" / "bssl-sys" / "librust_wrapper.a",
        build_dir / "rust" / "bssl-sys" / f"wrapper_{target}.rs",
    )
    missing = [path for path in required_paths if not path.is_file()]
    if missing:
        missing_text = ", ".join(os.fspath(path) for path in missing)
        raise FileNotFoundError(
            f"BoringSSL is not built for {target}; missing: {missing_text}. "
            f"Build it with -DRUST_BINDINGS={target}, or set BORINGSSL_BUILD_DIR "
            "to the matching build directory."
        )

    env["BORINGSSL_BUILD_DIR"] = os.fspath(build_dir)
    bssl_sys_path = json.dumps(bssl_sys_dir.as_posix())
    cargo_patch = f"patch.crates-io.bssl-sys.path={bssl_sys_path}"
    return env, cargo_patch


def build_binary(
    *,
    abi: str,
    target: str,
    release: bool,
    package: str | None,
    bin_name: str,
) -> Path:
    build_type = "release" if release else "debug"
    print(f"Building {bin_name} for {abi} ({target}, {build_type})...")

    env, cargo_patch = cargo_context_for_target(target)
    cmd = ["cargo", "--config", cargo_patch, "build", "--target", target]
    if package:
        cmd.extend(["-p", package, "--bin", bin_name])
    else:
        cmd.extend(["--bin", bin_name])
    if release:
        cmd.append("--release")

    run(cmd, env=env)

    binary_path = TARGET_ROOT / target / build_type / bin_name
    if not binary_path.exists():
        raise FileNotFoundError(f"Built binary not found at {binary_path}")
    return binary_path


def copy_binary(binary: Path, output_name: str, abi: str, stage_dir: Path) -> None:
    dest_dir = stage_dir / "libs" / abi
    dest_dir.mkdir(parents=True, exist_ok=True)
    dest_path = dest_dir / output_name
    shutil.copy2(binary, dest_path)
    print(f"Copied {binary} to {dest_path}")


def resolve_ndk_root(cli_value: str | None) -> Path:
    """Reuse the NDK discovery the Cargo config generator already relies on."""
    sys.path.insert(0, os.fspath(REPO_ROOT / "scripts"))
    from setup_cargo_config import detect_ndk_root

    return detect_ndk_root(cli_value)


def ndk_host_can_run(ndk_root: Path) -> bool:
    """Whether this machine can execute the NDK's own host tools.

    The NDK ships host toolchains for Linux x86_64 and the Darwin architectures
    only. On an aarch64 Linux host its launcher stops with "Unknown host CPU
    architecture: aarch64" before any tool runs, so the payload has to be
    compiled by the machine's own compiler instead.
    """
    sys.path.insert(0, os.fspath(REPO_ROOT / "scripts"))
    from setup_cargo_config import resolve_host_tag

    host_tag = resolve_host_tag(ndk_root, None)
    machine = platform.machine()
    if host_tag == "linux-x86_64":
        return machine in ("x86_64", "amd64")
    if host_tag == "linux-aarch64":
        return machine in ("aarch64", "arm64")
    # Darwin, and anything unrecognised: let ndk-build speak for itself.
    return True


def build_soter_zygisk_ndk_build(*, abi: str, ndk_root: Path) -> Path:
    """Compile the payload with the NDK's own ndk-build.

    ndk-build defaults its intermediate and library directories to
    ``<project>/obj`` and ``<project>/libs``. Both are redirected under
    ``target/`` instead, because leaving them inside the vendored tree would
    make the worktree dirty and flip ``get_git_commit_hash()`` to its
    ``-dirty`` form on every build after the first.
    """
    project_dir = SOTER_ZYGISK_SOURCE_DIR
    if not project_dir.is_dir():
        raise FileNotFoundError(f"D-soter source directory not found at {project_dir}")

    ndk_build = ndk_root / "ndk-build"
    if not ndk_build.is_file():
        raise FileNotFoundError(f"ndk-build not found at {ndk_build}")

    output_dir = TARGET_ROOT / "dsoter" / abi
    print(f"Building D-soter Zygisk payload for {abi} with NDK {ndk_root}...")
    run(
        [
            os.fspath(ndk_build),
            "-C",
            os.fspath(project_dir),
            f"NDK_OUT={output_dir / 'obj'}",
            f"NDK_LIBS_OUT={output_dir / 'libs'}",
        ]
    )

    library_path = output_dir / "libs" / abi / SOTER_ZYGISK_LIBRARY
    if not library_path.is_file():
        raise FileNotFoundError(f"Built D-soter payload not found at {library_path}")
    return library_path


def soter_zygisk_builtins(ndk_root: Path) -> Path:
    """The NDK's compiler-rt builtins archive for aarch64."""
    pattern = os.fspath(
        ndk_root
        / "toolchains"
        / "llvm"
        / "prebuilt"
        / "*"
        / "lib"
        / "clang"
        / "*"
        / "lib"
        / "linux"
        / "libclang_rt.builtins-aarch64-android.a"
    )
    matches = sorted(glob.glob(pattern))
    if not matches:
        raise FileNotFoundError(
            f"NDK compiler-rt builtins for aarch64 not found under {ndk_root}"
        )
    return Path(matches[-1])


def find_native_cxx() -> str:
    candidates = ["clang++", "clang++-21", "g++"]
    for name in candidates:
        found = shutil.which(name)
        if found:
            return found
    matches = sorted(glob.glob("/usr/bin/clang++-*"))
    if matches:
        return matches[-1]
    raise FileNotFoundError(
        "no native C++ compiler found to build the D-soter payload"
    )


def strip_shared_library(library: Path) -> None:
    """Strip the payload when a native stripper is available.

    ndk-build strips as part of its own build, so the fallback does the same to
    keep the two paths producing payloads of comparable size.
    """
    candidates = ["llvm-strip", "llvm-strip-21", "strip"]
    tool = next((shutil.which(name) for name in candidates if shutil.which(name)), None)
    if tool is None:
        matches = sorted(glob.glob("/usr/bin/llvm-strip-*"))
        tool = matches[-1] if matches else None
    if tool is None:
        print("note: no native stripper found; shipping the unstripped payload")
        return
    subprocess.run([tool, "--strip-unneeded", os.fspath(library)], check=False)


def build_soter_zygisk_native(*, abi: str, ndk_root: Path) -> Path:
    """Compile the payload with this machine's compiler against the NDK sysroot.

    This reproduces module/jni/Android.mk and Application.mk by hand, for hosts
    whose architecture the NDK does not ship tools for. It exists only as a
    local-build convenience: ndk-build remains the canonical path, and the two
    do not produce byte-identical payloads, because the compiler differs.
    """
    sys.path.insert(0, os.fspath(REPO_ROOT / "scripts"))
    from setup_cargo_config import resolve_host_tag

    host_tag = resolve_host_tag(ndk_root, None)
    sysroot = ndk_root / "toolchains" / "llvm" / "prebuilt" / host_tag / "sysroot"
    if not sysroot.is_dir():
        raise FileNotFoundError(f"NDK sysroot not found at {sysroot}")

    project_dir = SOTER_ZYGISK_SOURCE_DIR
    jni_dir = project_dir / "jni"
    source = jni_dir / SOTER_ZYGISK_SOURCE_NAME
    if not source.is_file():
        raise FileNotFoundError(f"D-soter source not found at {source}")

    prebuilts = project_dir.parent / "prebuilts" / abi
    if not prebuilts.is_dir():
        raise FileNotFoundError(f"D-soter prebuilt libraries not found at {prebuilts}")

    output_dir = TARGET_ROOT / "dsoter" / abi / "native"
    output_dir.mkdir(parents=True, exist_ok=True)

    # The host driver still emits -lgcc for this target, where the NDK's own
    # driver would use compiler-rt. Alias it to the NDK builtins rather than
    # depend on a system libgcc, which does not exist for aarch64 Android.
    shim_dir = output_dir / "hostlibs"
    shim_dir.mkdir(parents=True, exist_ok=True)
    shutil.copy2(soter_zygisk_builtins(ndk_root), shim_dir / "libgcc.a")

    includes = [jni_dir]
    includes.extend(jni_dir / relative for relative in SOTER_ZYGISK_INCLUDE_DIRS)

    library_path = output_dir / SOTER_ZYGISK_LIBRARY
    print(f"Building D-soter Zygisk payload for {abi} with the host compiler...")
    run(
        [
            find_native_cxx(),
            f"--target=aarch64-linux-android{SOTER_ZYGISK_API}",
            f"--sysroot={sysroot}",
            *SOTER_ZYGISK_COMPILE_FLAGS,
            *(f"-I{include}" for include in includes),
            os.fspath(source),
            "-o",
            os.fspath(library_path),
            "-static-libstdc++",
            f"-L{shim_dir}",
            f"-L{prebuilts}",
            *SOTER_ZYGISK_LINK_LIBS,
        ]
    )

    if not library_path.is_file():
        raise FileNotFoundError(f"Built D-soter payload not found at {library_path}")
    strip_shared_library(library_path)
    return library_path


def build_soter_zygisk(*, abi: str, ndk_root: Path) -> Path:
    """Compile the vendored D-soter Zygisk payload, preferring ndk-build."""
    if not SOTER_ZYGISK_SOURCE_DIR.is_dir():
        raise FileNotFoundError(
            f"D-soter source directory not found at {SOTER_ZYGISK_SOURCE_DIR}"
        )

    if ndk_host_can_run(ndk_root):
        return build_soter_zygisk_ndk_build(abi=abi, ndk_root=ndk_root)

    print(
        f"note: the NDK's host tools target a different architecture than this "
        f"{platform.machine()} host, so the payload is built with the native compiler"
    )
    return build_soter_zygisk_native(abi=abi, ndk_root=ndk_root)


def copy_zygisk_payload(library: Path, abi: str, stage_dir: Path) -> None:
    payload_dir = stage_dir / "zygisk"
    payload_dir.mkdir(parents=True, exist_ok=True)
    destination = payload_dir / f"{abi}.so"
    shutil.copy2(library, destination)
    print(f"Copied {library} to {destination}")


def copy_template_files(stage_dir: Path) -> None:
    template_dir = REPO_ROOT / "template"
    if not template_dir.exists():
        raise FileNotFoundError("Template directory not found")

    missing = [name for name in REQUIRED_TEMPLATE_FILES if not (template_dir / name).exists()]
    if missing:
        raise FileNotFoundError(f"Template is missing required file(s): {', '.join(missing)}")

    print(f"Copying template files into {stage_dir}...")
    for item in template_dir.iterdir():
        dst = stage_dir / item.name
        if item.is_dir():
            shutil.copytree(item, dst, dirs_exist_ok=True)
        else:
            shutil.copy2(item, dst)


def copy_project_documents(stage_dir: Path) -> None:
    documents = (
        (REPO_ROOT / "README.md", stage_dir / "README.md"),
        (REPO_ROOT / "LICENSE.md", stage_dir / "LICENSE.md"),
        (REPO_ROOT / "LICENSE-2", stage_dir / "LICENSE-2"),
        (
            REPO_ROOT / "webui" / "LICENSE.upstream",
            stage_dir / "THIRD_PARTY_LICENSES" / "Tricky-Addon-Update-Target-List.txt",
        ),
        (
            SOTER_ZYGISK_LICENSE,
            stage_dir / "THIRD_PARTY_LICENSES" / "d-soter-Apache-2.0.txt",
        ),
        (
            SOTER_ZYGISK_SYSCALL_NOTE,
            stage_dir / "THIRD_PARTY_LICENSES" / "linux-syscall-note-GPL-2.0.txt",
        ),
    )
    for source, destination in documents:
        if not source.is_file():
            raise FileNotFoundError(f"Required license file not found: {source}")
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, destination)


def write_text_lf(path: Path, content: str) -> None:
    with path.open("w", encoding="utf-8", newline="\n") as fh:
        fh.write(content)


def normalize_module_text_files(stage_dir: Path) -> None:
    for relative_path in MODULE_TEXT_FILES:
        path = stage_dir / relative_path
        if not path.exists():
            continue
        content = path.read_text(encoding="utf-8")
        content = content.replace("\r\n", "\n").replace("\r", "\n")
        write_text_lf(path, content)


def modify_module_prop(
    stage_dir: Path,
    version: str,
    git_count: str,
    git_hash: str,
    release: bool,
) -> None:
    module_prop_path = stage_dir / "module.prop"
    if not module_prop_path.exists():
        raise FileNotFoundError(f"module.prop not found at {module_prop_path}")

    build_type = "release" if release else "debug"
    version_name = f"{version} ({git_count}-{git_hash}-{build_type})"
    content = module_prop_path.read_text(encoding="utf-8")
    content = content.replace("${versionName}", version_name)
    content = content.replace("${versionCode}", git_count)
    write_text_lf(module_prop_path, content)
    print(f"Updated module.prop: versionName={version_name}, versionCode={git_count}")


def generate_hash_for_file(file_path: Path) -> None:
    digest = hashlib.sha256()
    with file_path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(1024 * 1024), b""):
            digest.update(chunk)

    hash_path = file_path.with_name(f"{file_path.name}.sha256")
    hash_path.write_text(digest.hexdigest(), encoding="utf-8")
    print(f"Created hash file: {hash_path}")


def generate_hash_files(stage_dir: Path) -> None:
    print(f"Generating SHA256 hash files under {stage_dir}...")
    for item in stage_dir.rglob("*"):
        if item.is_file() and not item.name.endswith(".sha256"):
            generate_hash_for_file(item)


def generate_webroot_manifest(stage_dir: Path) -> None:
    webroot = stage_dir / "webroot"
    required = (webroot / "index.html", webroot / "config.json")
    missing = [path.relative_to(stage_dir).as_posix() for path in required if not path.is_file()]
    if missing:
        raise FileNotFoundError(f"WebUI build is incomplete; missing: {', '.join(missing)}")

    assets = sorted(
        path.relative_to(stage_dir).as_posix()
        for path in webroot.rglob("*")
        if path.is_file() and not path.name.endswith(".sha256")
    )
    if not assets:
        raise FileNotFoundError("WebUI build did not produce any files")
    write_text_lf(stage_dir / "webroot.manifest", "\n".join(assets) + "\n")
    print(f"Created WebUI manifest with {len(assets)} asset(s)")


def delete_old_zips(release: bool, selected_abis: list[str], version: str) -> None:
    build_type = "release" if release else "debug"
    old_zips: set[str] = set()
    for abi in selected_abis:
        # Remove packages produced by the previous naming scheme as well as
        # packages for this version produced by an earlier build.  The latter
        # is useful when the working tree hash changes between builds.
        old_pattern = TARGET_ROOT / f"OhMyKeymint-{build_type}-{abi}-*.zip"
        old_zips.update(glob.glob(os.fspath(old_pattern)))

        if abi == "arm64-v8a":
            new_pattern = TARGET_ROOT / f"OhMyKeymint-{version}-*-{build_type}.zip"
            new_candidates = glob.glob(os.fspath(new_pattern))
            # The arm64 package intentionally omits its ABI in the filename;
            # retain a separately built x86_64 package with the same version.
            old_zips.update(
                path
                for path in new_candidates
                if not path.endswith(f"-x86_64-{build_type}.zip")
            )
        else:
            new_pattern = TARGET_ROOT / f"OhMyKeymint-{version}-*-{abi}-{build_type}.zip"
            old_zips.update(glob.glob(os.fspath(new_pattern)))
    if not old_zips:
        print(f"No old zip files found for build type {build_type} and ABIs {selected_abis}")
        return

    print(f"Found {len(old_zips)} old zip file(s) to delete:")
    for old_zip in old_zips:
        print(f"  Deleting: {old_zip}")
        os.remove(old_zip)


def create_zip_package(
    *,
    stage_dir: Path,
    version: str,
    git_count: str,
    git_hash: str,
    abi: str,
    release: bool,
) -> Path:
    build_type = "release" if release else "debug"
    # The default arm64 package follows the same version identity order as
    # Trickystore: version, commit count, short commit hash, build type.  An
    # explicit x86_64 build gets an ABI discriminator so separate ABI builds
    # cannot overwrite one another.
    abi_suffix = f"-{abi}" if abi != "arm64-v8a" else ""
    zip_name = f"OhMyKeymint-{version}-{git_count}-{git_hash}{abi_suffix}-{build_type}.zip"
    zip_path = TARGET_ROOT / zip_name
    print(f"Creating zip package: {zip_path}")

    with zipfile.ZipFile(zip_path, "w", zipfile.ZIP_DEFLATED) as zipf:
        for root, _, files in os.walk(stage_dir):
            for file_name in files:
                file_path = Path(root) / file_name
                arcname = file_path.relative_to(stage_dir)
                zipf.write(file_path, arcname)

    return zip_path


def build_package_for_abi(
    *,
    abi: str,
    release: bool,
    platform: int,
    ndk_root: Path,
    version: str,
    git_count: str,
    git_hash: str,
) -> Path:
    target = ABI_TO_TARGET[abi]
    stage_dir = TARGET_ROOT / "temp" / abi
    # Kept for compatibility with old invocations; plain Cargo uses .cargo/config.toml.
    _ = platform
    if stage_dir.exists():
        shutil.rmtree(stage_dir)
    stage_dir.mkdir(parents=True, exist_ok=True)

    try:
        built_binaries: dict[str, Path] = {}
        for spec in BINARY_SPECS:
            built_binaries[spec["output_name"]] = build_binary(
                abi=abi,
                target=target,
                release=release,
                package=spec["package"],
                bin_name=spec["bin"],
            )
        soter_payload = build_soter_zygisk(abi=abi, ndk_root=ndk_root)

        copy_template_files(stage_dir)
        copy_project_documents(stage_dir)
        normalize_module_text_files(stage_dir)
        for spec in BINARY_SPECS:
            copy_binary(
                built_binaries[spec["output_name"]],
                spec["output_name"],
                abi,
                stage_dir,
            )
        copy_zygisk_payload(soter_payload, abi, stage_dir)

        modify_module_prop(stage_dir, version, git_count, git_hash, release)
        normalize_module_text_files(stage_dir)
        generate_webroot_manifest(stage_dir)
        generate_hash_files(stage_dir)
        return create_zip_package(
            stage_dir=stage_dir,
            version=version,
            git_count=git_count,
            git_hash=git_hash,
            abi=abi,
            release=release,
        )
    finally:
        if stage_dir.exists():
            shutil.rmtree(stage_dir)


def main() -> None:
    parser = argparse.ArgumentParser(description="Build OhMyKeymint Magisk packages for Android")
    parser.add_argument("--release", action="store_true", help="Build in release mode")
    parser.add_argument("--debug", action="store_true", help="Build in debug mode (default)")
    parser.add_argument(
        "--abi",
        dest="abis",
        action="append",
        choices=sorted(ABI_TO_TARGET),
        help="Build only the selected Android ABI(s). Defaults to arm64-v8a.",
    )
    parser.add_argument(
        "--platform",
        type=int,
        default=DEFAULT_PLATFORM,
        help=(
            "Compatibility option; ordinary cargo builds use .cargo/config.toml "
            f"for the Android API/linker (default: {DEFAULT_PLATFORM})"
        ),
    )
    parser.add_argument(
        "--ndk-root",
        help=(
            "Path to the Android NDK root used to build the Zygisk payload. "
            "Defaults to ANDROID_NDK_ROOT, ANDROID_NDK_HOME, or the newest NDK "
            "under ANDROID_HOME/ANDROID_SDK_ROOT."
        ),
    )
    args = parser.parse_args()

    version = get_version_from_cargo_toml()
    git_count = get_git_commit_count()
    git_hash = get_git_commit_hash()
    selected_abis = args.abis or ["arm64-v8a"]

    try:
        ndk_root = resolve_ndk_root(args.ndk_root)
    except (FileNotFoundError, RuntimeError) as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1) from error

    print(f"Building OhMyKeymint version {version} (commit {git_count}, hash {git_hash})")
    print(f"Build mode: {'Release' if args.release else 'Debug'}")
    print(f"Target ABIs: {', '.join(selected_abis)}")

    delete_old_zips(args.release, selected_abis, version)
    built_packages = []
    for abi in selected_abis:
        built_packages.append(
            build_package_for_abi(
                abi=abi,
                release=args.release,
                platform=args.platform,
                ndk_root=ndk_root,
                version=version,
                git_count=git_count,
                git_hash=git_hash,
            )
        )

    print("Build completed successfully!")
    for zip_path in built_packages:
        print(f"Output: {zip_path}")


if __name__ == "__main__":
    main()
