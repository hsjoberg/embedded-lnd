#include <napi.h>
#include <iostream>
#include <string>
#include <dlfcn.h>
#include "lnd_functions.h"
#include "lnd_server_streams.h"

#define LOG(x) std::cout << x << std::endl
#define ERROR(x) std::cerr << x << std::endl

typedef void (*LndStartFuncPtr)(char*, CCallback);
typedef int (*SendStreamFuncPtr)(uintptr_t, char*, int);
typedef int (*StopStreamFuncPtr)(uintptr_t);

void* libHandle = nullptr;
SendStreamFuncPtr sendStreamFunc = nullptr;
StopStreamFuncPtr stopStreamFunc = nullptr;

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

    // Fix start function
    void* startFunc = dlsym(libHandle, "start");
    if (!startFunc) {
        ERROR("Failed to load function: start - " << dlerror());
    } else {
        LndStartFuncPtr startFuncCasted = reinterpret_cast<LndStartFuncPtr>(startFunc);

        exports.Set("start", Napi::Function::New(env, [startFuncCasted](const Napi::CallbackInfo& info) {
            Napi::Env env = info.Env();
            Napi::Promise::Deferred deferred = Napi::Promise::Deferred::New(env);
            std::string args;

            if (info.Length() > 0) {
                args = info[0].As<Napi::String>().Utf8Value();
            } else {
                deferred.Reject(Napi::Error::New(env, "Invalid argument type for start").Value());
                return deferred.Promise();
            }

            auto tsfn = Napi::ThreadSafeFunction::New(
                env,
                Napi::Function::New(env, [deferred](const Napi::CallbackInfo& info) {
                    if (info[0].IsNull()) {
                        deferred.Resolve(info[1]);
                    } else {
                        deferred.Reject(info[0]);
                    }
                }),
                "LND Callback",
                0,
                1
            );

            auto* callbackData = new CallbackData{tsfn};

            CCallback callback = {
                ResponseCallback,
                ErrorCallback,
                callbackData,
                callbackData
            };

            startFuncCasted(const_cast<char*>(args.c_str()), callback);

            return deferred.Promise();
        }, "start"));
    }

    // Regular functions
    std::vector<std::string> functionNames = {"getInfo"};
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

    // Streaming functions (server-side)
    std::vector<std::string> streamFunctionNames = {"subscribeState"};
    for (const auto& name : streamFunctionNames) {
        void* func = dlsym(libHandle, name.c_str());
        if (!func) {
            ERROR("Failed to load function: " << name << " - " << dlerror());
        }

        if (func) {
            exports.Set(name, Napi::Function::New(env, [name, func](const Napi::CallbackInfo& info) {
                return CallLndStream(info, name, reinterpret_cast<LndStreamFuncPtr>(func));
            }, name));
            LOG("Loaded stream function: " << name);
        }
    }


    sendStreamFunc = reinterpret_cast<SendStreamFuncPtr>(dlsym(libHandle, "SendStreamC"));
    if (!sendStreamFunc) {
        ERROR("Failed to load function: SendStreamC - " << dlerror());
    }

    stopStreamFunc = reinterpret_cast<StopStreamFuncPtr>(dlsym(libHandle, "StopStreamC"));
    if (!stopStreamFunc) {
        ERROR("Failed to load function: StopStreamC - " << dlerror());
    }

    // Bi-directional streaming functions
    std::vector<std::string> biStreamFunctionNames = {"channelAcceptor"};
    for (const auto& name : biStreamFunctionNames) {
        void* func = dlsym(libHandle, name.c_str());
        if (!func) {
            ERROR("Failed to load function: " << name << " - " << dlerror());
        }

        if (func) {
            exports.Set(name, Napi::Function::New(env, [name, func](const Napi::CallbackInfo& info) {
                return CallLndBiStream(info, name, reinterpret_cast<LndBiStreamFuncPtr>(func), sendStreamFunc, stopStreamFunc);
            }, name));
            LOG("Loaded bi-directional stream function: " << name);
        }
    }

    LOG("Addon initialized successfully");
    return exports;
}

NODE_API_MODULE(addon, Init)
