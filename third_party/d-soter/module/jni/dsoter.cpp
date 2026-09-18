#include <android/log.h>
#include <binder/Binder.h>
#include <binder/Parcel.h>
#include <fcntl.h>
#include <inttypes.h>
#include <linux/android/binder.h>
#include <stdarg.h>
#include <stdio.h>
#include <string.h>
#include <sys/ioctl.h>
#include <sys/sysmacros.h>
#include <thread>
#include <mutex>
#include <queue>
#include <unordered_map>
#include <unistd.h>

#include "zygisk.hpp"

using android::BBinder;
using android::IBinder;
using android::Parcel;
using android::sp;
using android::status_t;
using android::String16;

#define TAG "D-Soter"
#define LOGI(...) __android_log_print(ANDROID_LOG_INFO, TAG, __VA_ARGS__)
#define LOGW(...) __android_log_print(ANDROID_LOG_WARN, TAG, __VA_ARGS__)
#define LOGE(...) __android_log_print(ANDROID_LOG_ERROR, TAG, __VA_ARGS__)

namespace {

constexpr uint32_t BR_TRANSACTION_SEC_CTX_LOCAL = _IOR('r', 2, struct binder_transaction_data_secctx);
constexpr uint32_t kForgeDispatchCode = 0x51F07EADu;

int (*orig_ioctl)(int fd, int request, ...) = nullptr;
std::once_flag g_install_once;
sp<BBinder> g_forge_stub;
std::mutex g_ctx_mutex;
struct TxInfo { uint64_t id; uint32_t code; uid_t uid; pid_t pid; };
std::unordered_map<std::thread::id, std::queue<TxInfo>> g_ctx;
std::atomic<uint64_t> g_tx_counter{0};

static bool is_soter_code(uint32_t code) { return code >= 1 && code <= 13; }

static bool contains_soter_descriptor(const binder_transaction_data *txn) {
    if (!txn || txn->data_size == 0 || txn->data.ptr.buffer == 0) return false;
    const uint8_t *buf = reinterpret_cast<const uint8_t *>(txn->data.ptr.buffer);
    const size_t n = txn->data_size;
    static constexpr char16_t desc[] = u"com.tencent.soter.soterserver.ISoterService";
    const uint8_t *pat = reinterpret_cast<const uint8_t *>(desc);
    const size_t m = (sizeof(desc) / sizeof(desc[0]) - 1) * sizeof(char16_t);
    if (n < m) return false;
    for (size_t i = 0; i + m <= n; ++i) {
        if (memcmp(buf + i, pat, m) == 0) return true;
    }
    return false;
}

static std::vector<uint8_t> build_export_blob() {
    // Static DER SubjectPublicKeyInfo RSA-2048 generated once for local SDK parsing.
    static const char *pub =
        "MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAw8gEMK6J6jBvJr1b9K8j"
        "o4jMHF5D4BoHYXTsRov+v+clqEwXntTeXrOcQeuQX9Fys5S3Jmbs6safW1vmbJps"
        "k8Qe7wbi9p1v9uh3JzmF3j2Mw+tXtGI9h/1Vm1n6T3GrQJQ+tvuQ+vN8n6kMYl64"
        "J7CuyYw6P5vl6Z4WlfhdY5oJc0Q9T6xVwK6bg3DOjFEq5k1DTXJZuzqjONyYCuuP"
        "v7TTuLT8yT0+9m+CF7i65DKQJE3Ak0dCj0Ar1sIH7yLPlvWv85ExKYOvCXLdB6t8"
        "eWg0/eeoPHDLLv11Oyq9JR0gDk0iHT5SWG2FHKY5xIb3C2we8O7CVOaPwIDAQAB";
    std::string json = std::string("{\"pub_key\":\"") + pub + "\",\"counter\":0,\"cpu_id\":\"0000000000000000\",\"uid\":0}";
    std::vector<uint8_t> out;
    uint32_t len = static_cast<uint32_t>(json.size());
    out.push_back(len & 0xff); out.push_back((len >> 8) & 0xff); out.push_back((len >> 16) & 0xff); out.push_back((len >> 24) & 0xff);
    out.insert(out.end(), json.begin(), json.end());
    out.resize(out.size() + 256, 0);
    return out;
}

static const std::vector<uint8_t>& export_blob() { static auto b = build_export_blob(); return b; }
static const std::vector<uint8_t>& signature_blob() { static std::vector<uint8_t> b(256, 0); return b; }
static const std::vector<uint8_t>& device_blob() { static std::vector<uint8_t> b({'T','E','E','S','I','M','-','S','O','T','E','R','-','0','0','0','1'}); return b; }

static void write_no_exception(Parcel *reply) { reply->writeInt32(0); }
static void write_byte_array(Parcel *reply, const std::vector<uint8_t>& b) { reply->writeByteArray(b.size(), b.data()); }

static status_t forge_reply(uint32_t code, Parcel *reply) {
    if (!reply) return android::OK;
    reply->setDataSize(0);
    reply->setDataPosition(0);
    write_no_exception(reply);
    switch (code) {
        case 1: // generateAppSecureKey -> int
        case 4: // generateAuthKey -> int
        case 5: // removeAuthKey -> int
        case 7: // removeAllAuthKey -> int
            reply->writeInt32(0); break;
        case 3: // hasAskAlready -> boolean
        case 8: // hasAuthKey -> boolean
            reply->writeInt32(1); break;
        case 12: // getVersion -> int
            reply->writeInt32(1); break;
        case 2: // getAppSecureKey -> SoterExportResult
        case 6: { // getAuthKey -> SoterExportResult
            const auto &b = export_blob();
            reply->writeInt32(1);      // non-null parcelable
            reply->writeInt32(0);      // resultCode OK
            write_byte_array(reply, b);
            reply->writeInt32(static_cast<int32_t>(b.size()));
            break;
        }
        case 9: // initSigh -> SoterSessionResult
            reply->writeInt32(1);
            reply->writeInt64(1);
            reply->writeInt32(0);
            break;
        case 10: { // finishSign -> SoterSignResult
            const auto &b = signature_blob();
            reply->writeInt32(1);
            reply->writeInt32(0);
            write_byte_array(reply, b);
            reply->writeInt32(static_cast<int32_t>(b.size()));
            break;
        }
        case 11: { // getDeviceId -> SoterDeviceResult
            const auto &b = device_blob();
            reply->writeInt32(1);
            reply->writeInt32(0);
            write_byte_array(reply, b);
            reply->writeInt32(static_cast<int32_t>(b.size()));
            break;
        }
        case 13: // getExtraParam -> SoterExtraParam; Java Parcel.writeValue("optical") = type string + string
            reply->writeInt32(1);
            reply->writeInt32(0); // VAL_STRING
            reply->writeString16(String16("optical"));
            break;
        default:
            break;
    }
    LOGI("forged Soter tx code=%u", code);
    return android::OK;
}

class SoterForgeStub : public BBinder {
public:
    status_t onTransact(uint32_t code, const Parcel &data, Parcel *reply, uint32_t flags) override {
        (void)data; (void)flags;
        if (code != kForgeDispatchCode) return BBinder::onTransact(code, data, reply, flags);
        TxInfo info{0, 0, 0, 0};
        {
            std::lock_guard<std::mutex> lk(g_ctx_mutex);
            auto it = g_ctx.find(std::this_thread::get_id());
            if (it != g_ctx.end() && !it->second.empty()) {
                info = it->second.front();
                it->second.pop();
            }
        }
        if (!is_soter_code(info.code)) {
            LOGW("forge dispatch without tx context");
            return android::BAD_VALUE;
        }
        return forge_reply(info.code, reply);
    }
};

void inspect_and_rewrite(binder_transaction_data *txn) {
    if (!txn || txn->target.ptr == 0) return;
    if (!is_soter_code(txn->code) || !contains_soter_descriptor(txn)) return;

    TxInfo info{++g_tx_counter, txn->code, txn->sender_euid, txn->sender_pid};
    txn->target.ptr = reinterpret_cast<uintptr_t>(g_forge_stub->getWeakRefs());
    txn->cookie = reinterpret_cast<uintptr_t>(g_forge_stub.get());
    txn->code = kForgeDispatchCode;
    {
        std::lock_guard<std::mutex> lk(g_ctx_mutex);
        g_ctx[std::this_thread::get_id()].push(info);
    }
    LOGI("hijacked Soter tx id=%" PRIu64 " code=%u uid=%u pid=%d", info.id, info.code, info.uid, info.pid);
}

void process_read_buffer(const binder_write_read *bwr) {
    if (!bwr || bwr->read_size == 0 || bwr->read_consumed == 0 || bwr->read_buffer == 0) return;
    uintptr_t ptr = static_cast<uintptr_t>(bwr->read_buffer);
    uintptr_t end = ptr + bwr->read_consumed;
    while (ptr + sizeof(uint32_t) <= end) {
        uint32_t cmd = *reinterpret_cast<uint32_t *>(ptr);
        ptr += sizeof(uint32_t);
        size_t sz = _IOC_SIZE(cmd);
        if (ptr + sz > end) break;
        if (cmd == BR_TRANSACTION) {
            inspect_and_rewrite(reinterpret_cast<binder_transaction_data *>(ptr));
        } else if (cmd == BR_TRANSACTION_SEC_CTX_LOCAL) {
            auto *sec = reinterpret_cast<binder_transaction_data_secctx *>(ptr);
            inspect_and_rewrite(&sec->transaction_data);
        }
        ptr += sz;
    }
}

int hooked_ioctl(int fd, int request, ...) {
    va_list ap;
    va_start(ap, request);
    void *arg = va_arg(ap, void *);
    va_end(ap);
    int rc = orig_ioctl(fd, request, arg);
    if (rc >= 0 && request == BINDER_WRITE_READ && arg) {
        auto *bwr = static_cast<binder_write_read *>(arg);
        if (bwr->read_consumed > 0) process_read_buffer(bwr);
    }
    return rc;
}

bool find_libbinder(dev_t *dev, ino_t *ino) {
    FILE *fp = fopen("/proc/self/maps", "re");
    if (!fp) return false;
    char line[1024];
    bool ok = false;
    while (fgets(line, sizeof(line), fp)) {
        if (!strstr(line, "/libbinder.so")) continue;
        unsigned maj = 0, min = 0; unsigned long long inode = 0;
        // address perms offset dev:inode pathname
        if (sscanf(line, "%*s %*s %*s %x:%x %llu", &maj, &min, &inode) == 3) {
            *dev = makedev(maj, min);
            *ino = static_cast<ino_t>(inode);
            ok = inode != 0;
            break;
        }
    }
    fclose(fp);
    return ok;
}

void install_hook(zygisk::Api *api) {
    std::call_once(g_install_once, [api] {
        g_forge_stub = sp<SoterForgeStub>::make();
        dev_t dev = 0; ino_t ino = 0;
        for (int i = 0; i < 50 && !find_libbinder(&dev, &ino); ++i) usleep(100000);
        if (!dev || !ino) { LOGE("libbinder.so not found; Soter forge not installed"); return; }
        api->pltHookRegister(dev, ino, "ioctl", reinterpret_cast<void *>(hooked_ioctl), reinterpret_cast<void **>(&orig_ioctl));
        if (!api->pltHookCommit() || !orig_ioctl) { LOGE("pltHookCommit failed; orig=%p", reinterpret_cast<void *>(orig_ioctl)); return; }
        LOGI("Soter Zygisk forge installed on libbinder dev=%llu ino=%llu", (unsigned long long)dev, (unsigned long long)ino);
    });
}

} // namespace

class DSoterModule : public zygisk::ModuleBase {
public:
    void onLoad(zygisk::Api *api, JNIEnv *env) override { this->api_ = api; this->env_ = env; }

    void preAppSpecialize(zygisk::AppSpecializeArgs *args) override {
        const char *name = env_->GetStringUTFChars(args->nice_name, nullptr);
        target_ = name && strcmp(name, "com.tencent.soter.soterserver") == 0;
        if (name) env_->ReleaseStringUTFChars(args->nice_name, name);
        if (!target_) api_->setOption(zygisk::DLCLOSE_MODULE_LIBRARY);
    }

    void postAppSpecialize(const zygisk::AppSpecializeArgs *args) override {
        (void)args;
        if (!target_) return;
        LOGI("loaded in com.tencent.soter.soterserver pid=%d", getpid());
        install_hook(api_);
    }

private:
    zygisk::Api *api_{};
    JNIEnv *env_{};
    bool target_ = false;
};

REGISTER_ZYGISK_MODULE(DSoterModule)

