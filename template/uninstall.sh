#!/system/bin/sh

# KernelSU runs this script before removing the module directory. Keep the
# cleanup limited to OMK's two persistent roots; disabling a module must not
# remove data.
case "${KSU:-}" in
  true|1) ;;
  *) exit 0 ;;
esac

# Release the Qualcomm Soter HAL if an earlier module version took the service
# name over. The stock HAL is restarted so it is never left unanswered; both
# calls are best-effort because the HAL may already be the stock one.
pkill -9 -f 'soterta?-svc --mode=' 2>/dev/null || true
setprop ctl.stop vendor.soter 2>/dev/null || true
setprop ctl.start vendor.soter 2>/dev/null || true

remove_omk_directory() {
  target="$1"
  case "$target" in
    /data/adb/omk|/data/misc/keystore/omk) ;;
    *)
      echo "[oh_my_keymint] refusing unexpected cleanup path: $target" >&2
      return 1
      ;;
  esac

  if [ -e "$target" ] || [ -L "$target" ]; then
    if ! rm -rf "$target"; then
      echo "[oh_my_keymint] failed to remove $target" >&2
      return 1
    fi
  fi
}

status=0
remove_omk_directory /data/adb/omk || status=1
remove_omk_directory /data/misc/keystore/omk || status=1
exit "$status"
