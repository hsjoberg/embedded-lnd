#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::ptr;
use std::sync::mpsc::{channel, Sender};

use std::time::Duration;

pub struct LndClient;

use std::sync::Once;

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
            _length: ::std::os::raw::c_int,
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
            start(c_args.as_ptr() as *mut c_char, CALLBACK.unwrap());
        }
        Ok(())
    }
    pub fn get_info(&self) -> Result<String, String> {
        println!("Entering get_info function");
        let data = CString::new("").unwrap();
        let (tx, rx) = channel();

        extern "C" fn response_callback(
            context: *mut c_void,
            data: *const c_char,
            length: ::std::os::raw::c_int,
        ) {
            println!("Response callback invoked");
            let tx = unsafe { &*(context as *const Sender<Result<String, String>>) };
            let response =
                unsafe { std::slice::from_raw_parts(data as *const u8, length as usize) };
            let response_str = String::from_utf8_lossy(response).into_owned();
            println!("Response received: {}", response_str);
            tx.send(Ok(response_str)).unwrap();
        }

        extern "C" fn error_callback(context: *mut c_void, err: *const c_char) {
            println!("Error callback invoked");
            let tx = unsafe { &*(context as *const Sender<Result<String, String>>) };
            let error = unsafe { CStr::from_ptr(err).to_str().unwrap_or("").to_string() };
            println!("Error received: {}", error);
            tx.send(Err(error)).unwrap();
        }

        let callback = CCallback {
            onResponse: Some(response_callback),
            onError: Some(error_callback),
            responseContext: &tx as *const _ as *mut c_void,
            errorContext: &tx as *const _ as *mut c_void,
        };

        println!("Calling getInfo");
        unsafe {
            getInfo(data.as_ptr() as *mut c_char, 0, callback);
        }

        println!("getInfo called, waiting for response");
        match rx.recv_timeout(Duration::from_secs(5)) {
            Ok(result) => {
                println!("Received result within timeout");
                result
            }
            Err(e) => {
                println!("Timeout or error occurred: {:?}", e);
                Err("Timeout waiting for response".to_string())
            }
        }
    }
}
