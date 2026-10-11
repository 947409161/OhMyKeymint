# shellcheck disable=SC2034
SKIPUNZIP=1

SONAME="Oh My Keymint"
SUPPORTED_ABIS="arm64 x64"
MIN_SDK=29

if [ "$BOOTMODE" ] && [ "$KSU" ]; then
  ui_print "- Installing from KernelSU app"
  ui_print "- KernelSU version: $KSU_KERNEL_VER_CODE (kernel) + $KSU_VER_CODE (ksud)"
  if [ "$(which magisk)" ]; then
    ui_print "*********************************************************"
    ui_print "! Multiple root implementation is NOT supported!"
    ui_print "! Please uninstall Magisk before installing Oh My Keymint"
    abort    "*********************************************************"
  fi
elif [ "$BOOTMODE" ] && [ "$MAGISK_VER_CODE" ]; then
  ui_print "- Installing from Magisk app"
else
  ui_print "*********************************************************"
  ui_print "! Install from recovery is not supported"
  ui_print "! Please install from KernelSU or Magisk app"
  abort    "*********************************************************"
fi

VERSION=$(grep_prop version "${TMPDIR}/module.prop")
ui_print "- Installing $SONAME $VERSION"

# check architecture
support=false
for abi in $SUPPORTED_ABIS
do
  if [ "$ARCH" == "$abi" ]; then
    support=true
  fi
done
if [ "$support" == "false" ]; then
  abort "! Unsupported platform: $ARCH"
else
  ui_print "- Device platform: $ARCH"
fi

# check android
if [ "$API" -lt $MIN_SDK ]; then
  ui_print "! Unsupported sdk: $API"
  abort "! Minimal supported sdk is $MIN_SDK"
else
  ui_print "- Device sdk: $API"
fi

ui_print "- Extracting verify.sh"
unzip -o "$ZIPFILE" 'verify.sh' -d "$TMPDIR" >&2
if [ ! -f "$TMPDIR/verify.sh" ]; then
  ui_print "*********************************************************"
  ui_print "! Unable to extract verify.sh!"
  ui_print "! This zip may be corrupted, please try downloading again"
  abort    "*********************************************************"
fi
. "$TMPDIR/verify.sh"
extract "$ZIPFILE" 'customize.sh'  "$TMPDIR/.vunzip"
extract "$ZIPFILE" 'verify.sh'     "$TMPDIR/.vunzip"

ui_print "- Extracting module files"
extract "$ZIPFILE" 'module.prop'     "$MODPATH"
extract "$ZIPFILE" 'post-fs-data.sh' "$MODPATH"
extract "$ZIPFILE" 'service.sh'      "$MODPATH"
extract "$ZIPFILE" 'uninstall.sh'    "$MODPATH"
extract "$ZIPFILE" 'sepolicy.rule'   "$MODPATH"
extract "$ZIPFILE" 'daemon'          "$MODPATH"
extract "$ZIPFILE" 'daemon-injector' "$MODPATH"
chmod 755 "$MODPATH/daemon" "$MODPATH/daemon-injector" \
  "$MODPATH/post-fs-data.sh" "$MODPATH/service.sh" "$MODPATH/uninstall.sh"
if [ "$ARCH" = "x64" ] || [ "$ARCH" = "x86_64" ]; then
  ui_print "- Using packaged x64 binaries"
  BINDIR="$MODPATH/libs/x86_64"
  ZYGISK_ABI="x86_64"
  extract "$ZIPFILE" 'libs/x86_64/keymint' "$MODPATH"
  extract "$ZIPFILE" 'libs/x86_64/inject'  "$MODPATH"
elif [ "$ARCH" = "arm64" ] || [ "$ARCH" = "arm64-v8a" ]; then
  ui_print "- Using packaged arm64 binaries"
  BINDIR="$MODPATH/libs/arm64-v8a"
  ZYGISK_ABI="arm64-v8a"
  extract "$ZIPFILE" 'libs/arm64-v8a/keymint' "$MODPATH"
  extract "$ZIPFILE" 'libs/arm64-v8a/inject'  "$MODPATH"
else
  abort "! Unsupported platform: $ARCH"
fi

[ -f "$BINDIR/keymint" ] || abort "! Missing $BINDIR/keymint"
[ -f "$BINDIR/inject" ] || abort "! Missing $BINDIR/inject"
chmod 755 "$BINDIR/keymint" "$BINDIR/inject"

# Release the Qualcomm Soter HAL if an earlier module version took the service
# name over. The stock HAL is restarted so it is never left unanswered; both
# calls are best-effort because the HAL may already be the stock one.
pkill -9 -f 'soterta?-svc --mode=' 2>/dev/null || true
setprop ctl.stop vendor.soter 2>/dev/null || true
setprop ctl.start vendor.soter 2>/dev/null || true

ui_print "- Extracting Zygisk Soter payload"
extract "$ZIPFILE" "zygisk/$ZYGISK_ABI.so" "$MODPATH"
[ -f "$MODPATH/zygisk/$ZYGISK_ABI.so" ] || abort "! Missing Zygisk Soter payload"
chmod 755 "$MODPATH/zygisk/$ZYGISK_ABI.so"

CONFIG_DIR=$MODPATH/config
OLD_CONFIG_DIR=/data/adb/modules/oh_my_keymint/config
mkdir -p "$CONFIG_DIR"
if [ -d "$OLD_CONFIG_DIR" ] && [ "$OLD_CONFIG_DIR" != "$CONFIG_DIR" ]; then
  cp -an "$OLD_CONFIG_DIR"/. "$CONFIG_DIR"/ 2>/dev/null || true
fi
if [ ! -f "$CONFIG_DIR/keybox.xml" ] \
  && [ -f /data/misc/keystore/omk/keybox.xml ]; then
  cp -p /data/misc/keystore/omk/keybox.xml "$CONFIG_DIR/keybox.xml"
fi
if [ ! -f "$CONFIG_DIR/keybox.xml" ]; then
  extract "$ZIPFILE" 'config/keybox.xml' "$MODPATH"
fi
rm -f "$CONFIG_DIR/keymint-daemon.pid" "$CONFIG_DIR/injector-daemon.pid"
rm -f "$CONFIG_DIR/restart.keymint" "$CONFIG_DIR/restart.injector" "$CONFIG_DIR/restart.all"
rm -f "$CONFIG_DIR/injector.payload"
rm -f "$CONFIG_DIR/keymint" "$CONFIG_DIR/inject" "$CONFIG_DIR/injector"
chmod 0770 "$CONFIG_DIR"
chown 1017:1017 "$CONFIG_DIR"
