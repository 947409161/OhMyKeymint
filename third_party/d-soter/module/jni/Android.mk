LOCAL_PATH := $(call my-dir)

include $(CLEAR_VARS)
LOCAL_MODULE := dsoter
LOCAL_SRC_FILES := dsoter.cpp
LOCAL_C_INCLUDES := \
    $(LOCAL_PATH) \
    $(LOCAL_PATH)/external/AOSP/include \
    $(LOCAL_PATH)/external/linux-kernel/include
LOCAL_LDFLAGS := -L$(LOCAL_PATH)/../../prebuilts/arm64-v8a
LOCAL_LDLIBS := -llog -ldl -lbinder -lutils -lcutils -lbase
LOCAL_CPPFLAGS := -DLOG_TAG=\"D-Soter\" -DBINDER_DISABLE_NATIVE_HANDLE
include $(BUILD_SHARED_LIBRARY)
