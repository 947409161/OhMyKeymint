#!/system/bin/sh

# KernelSU runs this script before removing the module directory. Disabling a
# module does not remove its config or stored keys.
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

exit 0
