#include "lnd_server_streams.h"
#include <iostream>
#include <string>

#include "base64.hpp"

#define LOG(x) std::cout << x << std::endl
#define ERROR(x) std::cerr << x << std::endl

Napi::Value CallLndStream(const Napi::CallbackInfo& info, const std::string& functionName, LndStreamFuncPtr func) {
    Napi::Env env = info.Env();

    if (info.Length() <3 || !info[0].IsString() || !info[1].IsFunction() || !info[2].IsFunction()) {
        throw Napi::Error::New(env, "Invalid arguments for " +  functionName + ". Expected (string, function, function)");
    }

    std::string dataByteArray = base64::from_base64(info[0].As<Napi::String>().Utf8Value());
    Napi::Function dataCallback = info[1].As<Napi::Function>();
    Napi::Function errorCallback = info[2].As<Napi::Function>();

    auto callbackData = std::make_shared<StreamCallbackData>();

    callbackData->active = true;

    callbackData->dataTsfn = Napi::ThreadSafeFunction::New(
        env,
        dataCallback,
        "LND Stream Data Callback",
        0,
        1
    );

    callbackData->errorTsfn = Napi::ThreadSafeFunction::New(
        env,
        errorCallback,
        "LND Stream Error Callback",
        0,
        1
    );

    CRecvStream callback = {
        StreamResponseCallback,
        StreamErrorCallback,
        callbackData.get(),
        callbackData.get()
    };

    callbackData->cleanup = [functionName]() {
        // TODO(hsjoberg): server streams can't be closed in falafel
        LOG(functionName << " stream closed");
    };

    func(const_cast<char*>(dataByteArray.c_str()), static_cast<int>(dataByteArray.size()), callback);
    LOG(functionName << " stream started successfully");

    return CreateUnsubscribeFunction(env, callbackData);
}

void StreamResponseCallback(void* context, const char* data, int length) {
    auto* callbackData = static_cast<StreamCallbackData*>(context);
    if (!callbackData->active) {
        LOG("Stream is not active");
        return;
    }

    std::string encoded = base64::to_base64(std::string_view(data, length));

    auto callback = [encoded](Napi::Env env, Napi::Function jsCallback) {
        jsCallback.Call({Napi::String::New(env, encoded)});
    };

    callbackData->dataTsfn.NonBlockingCall(callback);
}

void StreamErrorCallback(void* context, const char* error) {
    auto* callbackData = static_cast<StreamCallbackData*>(context);
    if (!callbackData->active) {
        LOG("Stream is not active");
        return;
    }

    std::string errorStr(error);

    auto callback = [errorStr](Napi::Env env, Napi::Function jsCallback) {
        jsCallback.Call({Napi::String::New(env, errorStr)});
    };

    callbackData->errorTsfn.NonBlockingCall(callback);
}

Napi::Function CreateUnsubscribeFunction(const Napi::Env& env, std::shared_ptr<StreamCallbackData> callbackData) {
    return Napi::Function::New(env, [callbackData](const Napi::CallbackInfo& info) {
        if (callbackData->active) {
            callbackData->active = false;
            callbackData->cleanup();
            callbackData->dataTsfn.Release();
            callbackData->errorTsfn.Release();
            // delete callbackData; TODO(hsjoberg): I think this is leaking memory
        }
        return info.Env().Undefined();
    });
}


// Bidi streams
Napi::Value CallLndBiStream(const Napi::CallbackInfo& info, const std::string& functionName, LndBiStreamFuncPtr func, SendStreamFuncPtr sendStreamFunc, StopStreamFuncPtr stopStreamFunc) {
    Napi::Env env = info.Env();

    if (info.Length() < 2 || !info[0].IsFunction() || !info[1].IsFunction()) {
        throw Napi::Error::New(env, "Invalid arguments. Expected (function, function)");
    }

    Napi::Function dataCallback = info[0].As<Napi::Function>();
    Napi::Function errorCallback = info[1].As<Napi::Function>();

    LOG(functionName << " called");

    auto callbackData = std::make_shared<BiStreamCallbackData>();

    callbackData->active = true;
    callbackData->sendStreamFunc = sendStreamFunc;
    callbackData->stopStreamFunc = stopStreamFunc;

    callbackData->dataTsfn = Napi::ThreadSafeFunction::New(
        env,
        dataCallback,
        "LND BiStream Data Callback",
        0,
        1
    );

    callbackData->errorTsfn = Napi::ThreadSafeFunction::New(
        env,
        errorCallback,
        "LND BiStream Error Callback",
        0,
        1
    );

    CRecvStream callback = {
        StreamResponseCallback,
        StreamErrorCallback,
        callbackData.get(),
        callbackData.get()
    };

    callbackData->streamPtr = func(callback);

    callbackData->cleanup = [functionName, callbackData]() {
        LOG(functionName << " stream closed");
        callbackData->stopStreamFunc(callbackData->streamPtr);
    };

    LOG(functionName << " bi-directional stream started successfully");

    return CreateBiStreamFunctions(env, callbackData);
}

Napi::Object CreateBiStreamFunctions(const Napi::Env& env, std::shared_ptr<BiStreamCallbackData> callbackData) {
    Napi::Object result = Napi::Object::New(env);

    // Send function
    result.Set("send", Napi::Function::New(env, [callbackData](const Napi::CallbackInfo& info) {
        Napi::Env env = info.Env();
        if (info.Length() < 1 || !info[0].IsString()) {
            throw Napi::Error::New(env, "Invalid argument. Expected a string.");
        }

        if (!callbackData->active) {
            throw Napi::Error::New(env, "Stream is not active.");
        }

        std::string dataByteArray = base64::from_base64(info[0].As<Napi::String>().Utf8Value());
        int result = callbackData->sendStreamFunc(callbackData->streamPtr, const_cast<char*>(dataByteArray.c_str()), static_cast<int>(dataByteArray.length()));

        return Napi::Number::New(env, result);
    }));

    // Stop function
    result.Set("stop", Napi::Function::New(env, [callbackData](const Napi::CallbackInfo& info) {
        Napi::Env env = info.Env();

        if (callbackData->active) {
            callbackData->active = false;
            callbackData->cleanup();
            callbackData->dataTsfn.Release();
            callbackData->errorTsfn.Release();

            int result = callbackData->stopStreamFunc(callbackData->streamPtr);
            return Napi::Number::New(env, result);
        }

        return Napi::Number::New(env, 0);
    }));

    return result;
}
