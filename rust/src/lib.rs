use once_cell::sync::Lazy;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Once;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use lnd_grpc_rust::lnrpc;
use lnd_grpc_rust::prost::Message;

pub struct LndClient;

#[repr(C)]
#[derive(Clone)]
pub struct CCallback {
    pub on_response: Option<extern "C" fn(*mut c_void, *const c_char, c_int)>,
    pub on_error: Option<extern "C" fn(*mut c_void, *const c_char)>,
    pub response_context: *mut c_void,
    pub error_context: *mut c_void,
}

#[repr(C)]
pub struct CRecvStream {
    pub on_response: Option<extern "C" fn(*mut c_void, *const c_char, c_int)>,
    pub on_error: Option<extern "C" fn(*mut c_void, *const c_char)>,
    pub response_context: *mut c_void,
    pub error_context: *mut c_void,
}

extern "C" {
    fn start(extra_args: *const c_char, callback: CCallback);
    fn subscribePeerEvents(data: *const c_char, length: c_int, r_stream: CRecvStream);
    fn getInfo(data: *const c_char, length: c_int, callback: CCallback);
    fn addInvoice(data: *const c_char, length: c_int, callback: CCallback);
    fn connectPeer(data: *const c_char, length: c_int, callback: CCallback);

}
static GLOBAL_CALLBACK: Lazy<
    Mutex<Option<Box<dyn Fn(Result<lnrpc::PeerEvent, String>) + Send + Sync>>>,
> = Lazy::new(|| Mutex::new(None));

static INIT: Once = Once::new();
static mut CALLBACK: Option<CCallback> = None;

impl LndClient {
    pub fn new() -> Self {
        LndClient
    }

    pub fn start(&self, args: &str) -> Result<(), String> {
        let c_args = CString::new(args).map_err(|e| format!("Failed to create CString: {}", e))?;

        extern "C" fn response_callback(
            _context: *mut c_void,
            data: *const c_char,
            _length: c_int,
        ) {
            unsafe {
                if !data.is_null() {
                    let response = CStr::from_ptr(data).to_string_lossy().into_owned();
                    println!("Start Response: {}", response);
                } else {
                    eprintln!("Received null data in response");
                }
            }
        }

        extern "C" fn error_callback(_context: *mut c_void, error: *const c_char) {
            unsafe {
                if !error.is_null() {
                    let error_str = CStr::from_ptr(error).to_string_lossy().into_owned();
                    eprintln!("Start Error: {}", error_str);
                } else {
                    eprintln!("Received null error pointer");
                }
            }
        }

        let callback = CCallback {
            on_response: Some(response_callback),
            on_error: Some(error_callback),
            response_context: ptr::null_mut(),
            error_context: ptr::null_mut(),
        };

        unsafe {
            INIT.call_once(|| {
                CALLBACK = Some(callback.clone());
            });

            let c_args_ptr = c_args.as_ptr();
            if let Some(ref cb) = CALLBACK {
                start(c_args_ptr, cb.clone());
            } else {
                return Err("Callback not initialized".to_string());
            }
        }

        Ok(())
    }

    pub fn call_lnd_method<Req, Resp>(
        &self,
        request: Req,
        lnd_func: unsafe extern "C" fn(*const c_char, c_int, CCallback),
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
            on_response: Some(response_callback),
            on_error: Some(error_callback),
            response_context: &tx as *const _ as *mut c_void,
            error_context: &tx as *const _ as *mut c_void,
        };

        unsafe {
            let c_args_len = c_args.as_bytes().len() as c_int;
            let c_args_ptr = c_args.as_ptr();
            lnd_func(c_args_ptr, c_args_len, callback);
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
            on_response: Some(response_callback),
            on_error: Some(error_callback),
            response_context: std::ptr::null_mut(),
            error_context: std::ptr::null_mut(),
        };

        let request = lnrpc::PeerEventSubscription {};
        let encoded = request.encode_to_vec();
        let c_args = CString::new(encoded).unwrap();
        unsafe {
            let c_args_len = c_args.as_bytes().len() as c_int;
            let c_args_ptr = c_args.as_ptr();
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
