#pragma once
#include <napi.h>
#include <string>
#include <functional>
#include <memory>
#include "liblnd.h"

typedef void (*LndStreamFuncPtr)(char*, int length, CRecvStream);


typedef void (*LndStreamFuncPtr)(char*, int length, CRecvStream);
typedef uintptr_t (*LndBiStreamFuncPtr)(CRecvStream);
typedef int (*SendStreamFuncPtr)(uintptr_t, char*, int);
typedef int (*StopStreamFuncPtr)(uintptr_t);

struct StreamCallbackData {
    Napi::ThreadSafeFunction dataTsfn;
    Napi::ThreadSafeFunction errorTsfn;
    bool active;
    std::function<void()> cleanup;
};

struct BiStreamCallbackData : public StreamCallbackData {
    uintptr_t streamPtr;
    SendStreamFuncPtr sendStreamFunc;
    StopStreamFuncPtr stopStreamFunc;
};

Napi::Value CallLndStream(const Napi::CallbackInfo& info, const std::string& functionName, LndStreamFuncPtr func);
Napi::Value CallLndBiStream(const Napi::CallbackInfo& info, const std::string& functionName, LndBiStreamFuncPtr func, SendStreamFuncPtr sendStreamFunc, StopStreamFuncPtr stopStreamFunc);

void StreamResponseCallback(void* context, const char* data, int length);
void StreamErrorCallback(void* context, const char* error);

Napi::Function CreateUnsubscribeFunction(const Napi::Env& env, std::shared_ptr<StreamCallbackData> callbackData);
Napi::Object CreateBiStreamFunctions(const Napi::Env& env, std::shared_ptr<BiStreamCallbackData> callbackData);
