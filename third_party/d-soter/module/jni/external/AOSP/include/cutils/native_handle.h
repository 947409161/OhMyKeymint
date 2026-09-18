#pragma once
#ifdef __cplusplus
extern "C" {
#endif
typedef struct native_handle {
    int version;
    int numFds;
    int numInts;
    int data[0];
} native_handle_t;
#ifdef __cplusplus
}
#endif
