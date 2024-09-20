#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use once_cell::sync::Lazy;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;
use std::sync::mpsc::{channel, Sender};

use std::sync::Mutex;
use std::sync::Once;
use std::time::Duration;

use lnd_grpc_rust::lnrpc;
use lnd_grpc_rust::prost::Message;

pub struct LndClient;

static GLOBAL_CALLBACK: Lazy<
    Mutex<Option<Box<dyn Fn(Result<lnrpc::PeerEvent, String>) + Send + Sync>>>,
> = Lazy::new(|| Mutex::new(None));

static INIT: Once = Once::new();
static mut CALLBACK: Option<CCallback> = None;

impl LndClient {
    pub fn new() -> Self {
        LndClient
    }

    // pub fn channel_acceptor<F, G>(&self, on_receive: F, on_send: G) -> Result<(), String>
    // where
    //     F: Fn(Result<lnrpc::ChannelAcceptRequest, String>) + Send + Sync + 'static,
    //     G: Fn() -> Option<lnrpc::ChannelAcceptResponse> + Send + Sync + 'static,
    // {
    //     let on_receive = Arc::new(Mutex::new(on_receive));
    //     let on_send = Arc::new(Mutex::new(on_send));

    //     extern "C" fn response_callback(context: *mut c_void, data: *const c_char, length: c_int) {
    //         let context = unsafe {
    //             &*(context
    //                 as *const (
    //                     Arc<Mutex<dyn Fn(Result<lnrpc::ChannelAcceptRequest, String>)>>,
    //                     Arc<Mutex<dyn Fn() -> Option<lnrpc::ChannelAcceptResponse>>>,
    //                 ))
    //         };
    //         let (on_receive, on_send) = context;
    //         let response =
    //             unsafe { std::slice::from_raw_parts(data as *const u8, length as usize) };

    //         match lnrpc::ChannelAcceptRequest::decode(response) {
    //             Ok(request) => on_receive.lock().unwrap()(Ok(request)),
    //             Err(e) => {
    //                 on_receive.lock().unwrap()(Err(format!("Failed to decode request: {}", e)))
    //             }
    //         }

    //         if let Some(response) = on_send.lock().unwrap()() {
    //             let encoded = response.encode_to_vec();
    //             // Here you would typically send the response back to LND
    //             // For now, we'll just print it
    //             println!("Sending response: {:?}", encoded);
    //         }
    //     }

    //     extern "C" fn error_callback(context: *mut c_void, err: *const c_char) {
    //         let context = unsafe {
    //             &*(context
    //                 as *const (
    //                     Arc<Mutex<dyn Fn(Result<lnrpc::ChannelAcceptRequest, String>)>>,
    //                     Arc<Mutex<dyn Fn() -> Option<lnrpc::ChannelAcceptResponse>>>,
    //                 ))
    //         };
    //         let (on_receive, _) = context;
    //         let error = unsafe {
    //             CStr::from_ptr(err)
    //                 .to_str()
    //                 .unwrap_or("Unknown error")
    //                 .to_string()
    //         };
    //         on_receive.lock().unwrap()(Err(error));
    //     }

    //     let context: Box<(
    //         Arc<Mutex<dyn Fn(Result<lnrpc::ChannelAcceptRequest, String>)>>,
    //         Arc<Mutex<dyn Fn() -> Option<lnrpc::ChannelAcceptResponse>>>,
    //     )> = Box::new((on_receive, on_send));
    //     let context_ptr = Box::into_raw(context);

    //     let recv_stream = CRecvStream {
    //         on_response: Some(response_callback),
    //         on_error: Some(error_callback),
    //         response_context: context_ptr as *mut c_void,
    //         error_context: context_ptr as *mut c_void,
    //     };

    //     let send_stream = unsafe { channelAcceptor(recv_stream) };

    //     if send_stream.is_null() {
    //         // Clean up the context if channelAcceptor fails
    //         unsafe { Box::from_raw(context_ptr) };
    //         return Err("Failed to create send stream".to_string());
    //     }

    //     Ok(())
    // }

    pub fn start(&self, args: &str) -> Result<(), String> {
        let c_args = CString::new(args).unwrap();

        extern "C" fn response_callback(
            _context: *mut c_void,
            data: *const c_char,
            _length: c_int,
        ) {
            unsafe {
                println!("Start response callback invoked");
                let response = CStr::from_ptr(data).to_string_lossy().into_owned();
                println!("Start Response: {}", response);
            }
        }

        extern "C" fn error_callback(_context: *mut c_void, error: *const c_char) {
            unsafe {
                println!("Start error callback invoked");
                let error_str = CStr::from_ptr(error).to_string_lossy().into_owned();
                eprintln!("Start Error: {}", error_str);
            }
        }

        let callback = CCallback {
            onResponse: Some(response_callback),
            onError: Some(error_callback),
            responseContext: ptr::null_mut(),
            errorContext: ptr::null_mut(),
        };

        unsafe {
            INIT.call_once(|| {
                CALLBACK = Some(callback);
            });
            let c_args_ptr = c_args.into_raw();
            start(c_args_ptr, CALLBACK.unwrap());
            // Retake ownership of the CString so it will be properly dropped
            let _ = CString::from_raw(c_args_ptr);
        }
        Ok(())
    }

    pub fn call_lnd_method<Req, Resp>(
        &self,
        request: Req,
        lnd_func: unsafe extern "C" fn(*mut c_char, c_int, CCallback) -> (),
    ) -> Result<Resp, String>
    where
        Req: Message,
        Resp: Message + Default,
    {
        let encoded = request.encode_to_vec();
        let c_args = CString::new(encoded).unwrap();
        let (tx, rx) = channel::<Result<Vec<u8>, String>>();

        extern "C" fn response_callback(context: *mut c_void, data: *const c_char, length: c_int) {
            let tx = unsafe { &*(context as *const Sender<Result<Vec<u8>, String>>) };
            let response =
                unsafe { std::slice::from_raw_parts(data as *const u8, length as usize) };
            tx.send(Ok(response.to_vec())).unwrap();
        }

        extern "C" fn error_callback(context: *mut c_void, err: *const c_char) {
            let tx = unsafe { &*(context as *const Sender<Result<Vec<u8>, String>>) };
            let error = unsafe { CStr::from_ptr(err).to_str().unwrap_or("").to_string() };
            tx.send(Err(error)).unwrap();
        }

        let callback = CCallback {
            onResponse: Some(response_callback),
            onError: Some(error_callback),
            responseContext: &tx as *const _ as *mut c_void,
            errorContext: &tx as *const _ as *mut c_void,
        };

        unsafe {
            let c_args_len = c_args.as_bytes().len() as c_int;
            let c_args_ptr = c_args.into_raw();
            lnd_func(c_args_ptr, c_args_len, callback);
            let _ = CString::from_raw(c_args_ptr);
        }

        match rx.recv_timeout(Duration::from_secs(30)) {
            Ok(result) => result.and_then(|bytes| {
                Resp::decode(bytes.as_slice())
                    .map_err(|e| format!("Failed to decode response: {}", e))
            }),
            Err(_) => Err("Timeout waiting for response".to_string()),
        }
    }

    pub fn subscribe_peer_events<F>(&self, callback: F)
    where
        F: Fn(Result<lnrpc::PeerEvent, String>) + Send + Sync + 'static,
    {
        *GLOBAL_CALLBACK.lock().unwrap() = Some(Box::new(callback));

        extern "C" fn response_callback(_context: *mut c_void, data: *const c_char, length: c_int) {
            let response =
                unsafe { std::slice::from_raw_parts(data as *const u8, length as usize) };
            println!("Received peer event data, length: {}", length);

            match lnrpc::PeerEvent::decode(response) {
                Ok(event) => {
                    println!("Successfully decoded peer event: {:?}", event);
                    if let Some(callback) = GLOBAL_CALLBACK.lock().unwrap().as_ref() {
                        callback(Ok(event));
                    }
                }
                Err(e) => {
                    eprintln!("Failed to decode peer event: {}", e);
                    if let Some(callback) = GLOBAL_CALLBACK.lock().unwrap().as_ref() {
                        callback(Err(format!("Failed to decode event: {}", e)));
                    }
                }
            }
        }

        extern "C" fn error_callback(_context: *mut c_void, err: *const c_char) {
            let error = unsafe {
                CStr::from_ptr(err)
                    .to_str()
                    .unwrap_or("Unknown error")
                    .to_string()
            };
            eprintln!("Received error in peer event stream: {}", error);
            if let Some(callback) = GLOBAL_CALLBACK.lock().unwrap().as_ref() {
                callback(Err(error));
            }
        }

        let recv_stream = CRecvStream {
            onResponse: Some(response_callback),
            onError: Some(error_callback),
            responseContext: std::ptr::null_mut(),
            errorContext: std::ptr::null_mut(),
        };

        let request = lnrpc::PeerEventSubscription {};
        let encoded = request.encode_to_vec();
        let c_args = CString::new(encoded).unwrap();
        unsafe {
            let c_args_len = c_args.as_bytes().len() as c_int;
            let c_args_ptr = c_args.into_raw();
            subscribePeerEvents(c_args_ptr, c_args_len, recv_stream);
        }
        println!("Subscribed to peer events");
    }

    pub fn get_info(
        &self,
        request: lnrpc::GetInfoRequest,
    ) -> Result<lnrpc::GetInfoResponse, String> {
        self.call_lnd_method(request, getInfo)
    }

    pub fn add_invoice(
        &self,
        request: lnrpc::Invoice,
    ) -> Result<lnrpc::AddInvoiceResponse, String> {
        self.call_lnd_method(request, addInvoice)
    }

    pub fn connect_peer(
        &self,
        request: lnrpc::ConnectPeerRequest,
    ) -> Result<lnrpc::ConnectPeerResponse, String> {
        self.call_lnd_method(request, connectPeer)
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_lnd_client() {
//         // Your test code here...
//     }
// }
