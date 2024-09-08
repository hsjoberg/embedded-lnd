#include "lnd_functions.h"
#include <napi.h>
#include <iostream>
#include <string>

#define LOG(x) std::cout << x << std::endl
#define ERROR(x) std::cerr << x << std::endl

Napi::Value CallLndFunction(const Napi::CallbackInfo& info, const std::string& functionName, LndFuncPtr func) {
    Napi::Env env = info.Env();
    Napi::Promise::Deferred deferred = Napi::Promise::Deferred::New(env);

    try {
        std::string args;
        if (info.Length() > 0) {
            args = info[0].As<Napi::String>().Utf8Value();
        } else {
            deferred.Reject(Napi::Error::New(env, "Invalid argument type for " + functionName).Value());
            return deferred.Promise();
        }

        LOG(functionName << " called with args: " << args);

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

        auto* callbackData = new CallbackData{tsfn, ""};

        CCallback callback = {
            ResponseCallback,
            ErrorCallback,
            callbackData,
            callbackData
        };

        func(const_cast<char*>(args.c_str()), callback);
        LOG(functionName << " called successfully");
    } catch (const Napi::Error& e) {
        ERROR("Napi error: " << e.what());
        deferred.Reject(e.Value());
    } catch (const std::exception& e) {
        ERROR("Standard exception: " << e.what());
        deferred.Reject(Napi::Error::New(env, e.what()).Value());
    } catch (...) {
        ERROR("Unknown error occurred");
        deferred.Reject(Napi::Error::New(env, "Unknown error occurred").Value());
    }

    return deferred.Promise();
}

void ResponseCallback(void* context, const char* data, int length) {
    auto* callbackData = static_cast<CallbackData*>(context);
    callbackData->result = std::string(data, length);

    auto callback = [](Napi::Env env, Napi::Function jsCallback, CallbackData* data) {
        jsCallback.Call({env.Null(), Napi::String::New(env, data->result)});
        delete data;
    };

    callbackData->tsfn.BlockingCall(callbackData, callback);
    callbackData->tsfn.Release();
}

void ErrorCallback(void* context, const char* error) {
    auto* callbackData = static_cast<CallbackData*>(context);
    callbackData->result = error;

    auto callback = [](Napi::Env env, Napi::Function jsCallback, CallbackData* data) {
        jsCallback.Call({Napi::Error::New(env, data->result).Value(), env.Null()});
        delete data;
    };

    callbackData->tsfn.BlockingCall(callbackData, callback);
    callbackData->tsfn.Release();
}
