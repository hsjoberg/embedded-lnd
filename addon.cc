#include <napi.h>
#include <iostream>
#include <string>
#include <dlfcn.h>
#include "lnd_functions.h"
// #include "lnd_streams.h"

#define LOG(x) std::cout << x << std::endl
#define ERROR(x) std::cerr << x << std::endl

void* libHandle = nullptr;

// LndStreamWrapper lndStreamWrapper;

Napi::Object Init(Napi::Env env, Napi::Object exports) {
    LOG("Initializing addon");

    const char* lndLibPath = std::getenv("LND_LIB_PATH");
    if (lndLibPath == nullptr) {
        LOG("LND_LIB_PATH env var is not set. Defaulting to ./liblnd.so");
        lndLibPath = "./liblnd.so";
    } else {
        LOG("LND_LIB_PATH env var is set: " << lndLibPath);
    }

    LOG("Loading LND library from: " << lndLibPath);
    libHandle = dlopen(lndLibPath, RTLD_LAZY);
    if (!libHandle) {
        ERROR("Failed to load library: " << dlerror());
        Napi::Error::New(env, "Failed to load library: " + std::string(dlerror())).ThrowAsJavaScriptException();
        return exports;
    }
    LOG("LND library loaded successfully");

    // Regular functions
    std::vector<std::string> functionNames = {"start", "getInfo"};
    for (const auto& name : functionNames) {
        // Get the function pointer
        void* func = dlsym(libHandle, name.c_str());
        if (!func) {
            ERROR("Failed to load function: " << name << " - " << dlerror());
        }

        if (func) {
            exports.Set(name, Napi::Function::New(env, [name, func](const Napi::CallbackInfo& info) {
                return CallLndFunction(info, name, reinterpret_cast<LndFuncPtr>(func));
            }, name));
            LOG("Loaded function: " << name);
        }
    }

    // Streaming functions (both server-side and bidirectional)
    // std::vector<std::string> streamFunctionNames = {"subscribeState", "channelAcceptor"};
    // for (const auto& name : streamFunctionNames) {
    //     void* func = GetLndFunction(name);
    //     if (func) {
    //         exports.Set(name, Napi::Function::New(env, [name](const Napi::CallbackInfo& info) {
    //             return lndStreamWrapper.CallStreamFunction(info, name, reinterpret_cast<LndStreamFuncPtr>(GetLndFunction(name)));
    //         }, name));
    //         LOG("Loaded stream function: " << name);
    //     }
    // }

    LOG("Addon initialized successfully");
    return exports;
}

NODE_API_MODULE(addon, Init)
