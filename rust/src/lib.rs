#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;
use std::sync::mpsc::{channel, Sender};
use std::sync::Once;
use std::time::Duration;

use lnd_grpc_rust::lnrpc;
use lnd_grpc_rust::prost::Message;

pub struct LndClient;

static INIT: Once = Once::new();
static mut CALLBACK: Option<CCallback> = None;

impl LndClient {
    pub fn new() -> Self {
        LndClient
    }

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

    pub fn get_info(&self) -> Result<lnrpc::GetInfoResponse, String> {
        let request = lnrpc::GetInfoRequest {};
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
            // Get the length before converting to raw pointer
            let c_args_len = c_args.as_bytes().len() as c_int;
            let c_args_ptr = c_args.into_raw();
            getInfo(c_args_ptr, c_args_len, callback);
            // Retake ownership of the CString so it will be properly dropped
            let _ = CString::from_raw(c_args_ptr);
        }

        match rx.recv_timeout(Duration::from_secs(30)) {
            Ok(result) => result.and_then(|bytes| {
                lnrpc::GetInfoResponse::decode(bytes.as_slice())
                    .map_err(|e| format!("Failed to decode response: {}", e))
            }),
            Err(_) => Err("Timeout waiting for response".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lnd_client() {
        let client = LndClient::new();

        // Test start function
        match client.start("--lnddir=./lnd --noseedbackup") {
            Ok(()) => println!("LND started successfully"),
            Err(e) => eprintln!("Start error: {}", e),
        }

        // Test getInfo function
        match client.get_info() {
            Ok(info) => println!("Node info: {:?}", info),
            Err(e) => eprintln!("GetInfo error: {}", e),
        }
    }
}
