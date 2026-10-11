#!/system/bin/sh

MODDIR=${0%/*}
TARGET_DIR=$MODDIR/config
LEGACY_DIR=/data/misc/keystore/omk
STATE_DIR=$TARGET_DIR
SECURITY_PATCH_CONFIG=$TARGET_DIR/config.toml
SECURITY_PATCH_SNAPSHOT=$TARGET_DIR/security_patch_defaults.toml


mkdir -p "$TARGET_DIR"
chmod 0770 "$TARGET_DIR"
chown 1017:1017 "$TARGET_DIR"

mkdir -p "$STATE_DIR"
rm -f "$STATE_DIR/keymint-daemon.pid" "$STATE_DIR/injector-daemon.pid"
rm -f "$STATE_DIR/restart.keymint" "$STATE_DIR/restart.injector" "$STATE_DIR/restart.all"

# Move existing persistent state into the module config directory once. Logs
# are intentionally discarded; the runtime no longer writes log files.
move_legacy_file() {
  source_file=$1
  target_file=$2
  [ -f "$source_file" ] || return 0
  if [ ! -e "$target_file" ]; then
    mv "$source_file" "$target_file"
  elif cmp -s "$source_file" "$target_file"; then
    rm -f "$source_file"
  fi
}

if [ -d "$LEGACY_DIR" ]; then
  mkdir -p "$TARGET_DIR/data"
  for file in config.toml config.toml.bak crash_count timestamp keybox.xml; do
    move_legacy_file "$LEGACY_DIR/$file" "$TARGET_DIR/$file"
  done
  move_legacy_file "$LEGACY_DIR/data/security_patch_defaults.toml" \
    "$TARGET_DIR/security_patch_defaults.toml"
  rm -f "$LEGACY_DIR/data/google_attestation_status.json"
  if [ -d "$LEGACY_DIR/data" ]; then
    for file in "$LEGACY_DIR"/data/*; do
      [ -f "$file" ] || continue
      name=${file##*/}
      case "$name" in
        google_attestation_status.json|security_patch_defaults.toml) continue ;;
      esac
      move_legacy_file "$file" "$TARGET_DIR/data/$name"
    done
  fi
  # Runtime writes now target only MODDIR/config; retain other legacy files
  # until their contents are explicitly accounted for.
  rm -rf "$LEGACY_DIR/logs"
  rm -f "$LEGACY_DIR/keymint.log.lock" "$LEGACY_DIR/injector.log.lock"
fi

# Replay a synchronized security-patch override before Android framework caches
# Build.VERSION.SECURITY_PATCH. Restore removes the snapshot and changes all
# four fields to auto, so it naturally skips this block on the next boot.
reapply_security_patch_early() (
  [ -r "$SECURITY_PATCH_CONFIG" ] || return 0
  [ -f "$SECURITY_PATCH_SNAPSHOT" ] || return 0
  grep -Eq '^[[:space:]]*version[[:space:]]*=[[:space:]]*1[[:space:]]*$' \
    "$SECURITY_PATCH_SNAPSHOT" 2>/dev/null || return 0

  PATCH_FIELDS=$(awk '
    BEGIN { in_trust = 0; security = ""; os = ""; vendor = ""; boot = "" }
    /^[[:space:]]*\[[^]]+\][[:space:]]*$/ {
      in_trust = ($0 ~ /^[[:space:]]*\[trust\][[:space:]]*$/)
      next
    }
    in_trust {
      line = $0
      sub(/[[:space:]]*#.*/, "", line)
      key = line
      sub(/[[:space:]]*=.*/, "", key)
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", key)
      if (key != "security_patch" && key != "os_patchlevel" \
        && key != "vendor_patchlevel" && key != "boot_patchlevel") next
      value = line
      sub(/^[^=]*=[[:space:]]*/, "", value)
      if (substr(value, 1, 1) != "\"") next
      value = substr(value, 2)
      quote = index(value, "\"")
      if (quote == 0) next
      value = substr(value, 1, quote - 1)
      if (key == "security_patch") security = value
      else if (key == "os_patchlevel") os = value
      else if (key == "vendor_patchlevel") vendor = value
      else if (key == "boot_patchlevel") boot = value
    }
    END {
      if (security != "" && os != "" && vendor != "" && boot != "")
        printf "%s %s %s %s\n", security, os, vendor, boot
    }
  ' "$SECURITY_PATCH_CONFIG" 2>/dev/null)
  [ -n "$PATCH_FIELDS" ] || return 0

  # The config writer emits date/auto tokens. Disable globbing while splitting
  # the four values so malformed input cannot expand filenames.
  set -f
  IFS=' '
  set -- $PATCH_FIELDS
  [ "$#" -eq 4 ] || return 0
  [ "$1" = "$2" ] && [ "$1" = "$3" ] && [ "$1" = "$4" ] || return 0
  case "$1" in
    [0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]) ;;
    *) return 0 ;;
  esac
  PATCH_MONTH=${1#????-}
  PATCH_MONTH=${PATCH_MONTH%%-*}
  PATCH_DAY=${1##*-}
  case "$PATCH_MONTH" in
    01|02|03|04|05|06|07|08|09|10|11|12) ;;
    *) return 0 ;;
  esac
  case "$PATCH_DAY" in
    01|02|03|04|05|06|07|08|09|10|11|12|13|14|15|16|17|18|19|20|21|22|23|24|25|26|27|28|29|30|31) ;;
    *) return 0 ;;
  esac
  PATCH_DATE=$1

  RESETPROP_MODE=direct
  RESETPROP_BIN=
  for candidate in \
    /data/adb/ksu/bin/resetprop \
    /data/adb/magisk/resetprop \
    /data/adb/ap/bin/resetprop
  do
    if [ -x "$candidate" ]; then
      RESETPROP_BIN=$candidate
      break
    fi
  done
  if [ -z "$RESETPROP_BIN" ] && [ -x /data/adb/ksud ]; then
    RESETPROP_BIN=/data/adb/ksud
    RESETPROP_MODE=ksud
  elif [ -z "$RESETPROP_BIN" ] && [ -x /data/adb/apd ]; then
    RESETPROP_BIN=/data/adb/apd
    RESETPROP_MODE=apd
  fi
  if [ -z "$RESETPROP_BIN" ]; then
    RESETPROP_BIN=$(command -v resetprop 2>/dev/null)
  fi
  if [ -z "$RESETPROP_BIN" ]; then
    for candidate in /system_ext/bin/resetprop /system/bin/resetprop; do
      if [ -x "$candidate" ]; then
        RESETPROP_BIN=$candidate
        break
      fi
    done
  fi
  [ -n "$RESETPROP_BIN" ] || return 0

  if [ "$RESETPROP_MODE" = ksud ]; then
    "$RESETPROP_BIN" resetprop ro.vendor.build.security_patch "$PATCH_DATE" \
      >/dev/null 2>&1
    "$RESETPROP_BIN" resetprop ro.build.version.security_patch "$PATCH_DATE" \
      >/dev/null 2>&1
  elif [ "$RESETPROP_MODE" = apd ]; then
    "$RESETPROP_BIN" resetprop ro.vendor.build.security_patch "$PATCH_DATE" \
      >/dev/null 2>&1
    "$RESETPROP_BIN" resetprop ro.build.version.security_patch "$PATCH_DATE" \
      >/dev/null 2>&1
  else
    "$RESETPROP_BIN" ro.vendor.build.security_patch "$PATCH_DATE" >/dev/null 2>&1
    "$RESETPROP_BIN" ro.build.version.security_patch "$PATCH_DATE" >/dev/null 2>&1
  fi

  GETPROP_BIN=/system/bin/getprop
  [ -x "$GETPROP_BIN" ] || GETPROP_BIN=$(command -v getprop 2>/dev/null)
  if [ -n "$GETPROP_BIN" ]; then
    CURRENT_VENDOR=$("$GETPROP_BIN" ro.vendor.build.security_patch 2>/dev/null)
    CURRENT_SYSTEM=$("$GETPROP_BIN" ro.build.version.security_patch 2>/dev/null)
    if [ "$CURRENT_VENDOR" != "$PATCH_DATE" ] || [ "$CURRENT_SYSTEM" != "$PATCH_DATE" ]; then
      echo "[oh_my_keymint] unable to apply security patch $PATCH_DATE before framework startup" >&2
    fi
  fi
  return 0
)

reapply_security_patch_early || true
