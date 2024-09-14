#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::ptr;
use std::sync::mpsc::{channel, Sender};

pub struct LndClient;

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
                let response = CStr::from_ptr(data).to_string_lossy().into_owned();
                println!("Response: {}", response);
            }
        }

        extern "C" fn error_callback(_context: *mut c_void, error: *const c_char) {
            unsafe {
                let error_str = CStr::from_ptr(error).to_string_lossy().into_owned();
                eprintln!("Error: {}", error_str);
            }
        }

        let callback = CCallback {
            onResponse: Some(response_callback),
            onError: Some(error_callback),
            responseContext: ptr::null_mut(),
            errorContext: ptr::null_mut(),
        };

        unsafe {
            start(c_args.as_ptr() as *mut c_char, callback);
        }
        Ok(())
    }

    fn call_method<F>(&self, method: F, data: &str) -> Result<String, String>
    where
        F: Fn(*mut c_char, ::std::os::raw::c_int, CCallback),
    {
        let c_data = CString::new(data).unwrap();
        let (tx, rx) = channel();

        extern "C" fn response_callback(
            context: *mut c_void,
            data: *const c_char,
            _length: ::std::os::raw::c_int,
        ) {
            let tx = unsafe { &*(context as *const Sender<Result<String, String>>) };
            let response = unsafe { CStr::from_ptr(data).to_str().unwrap_or("").to_string() };
            tx.send(Ok(response)).unwrap();
        }

        extern "C" fn error_callback(context: *mut c_void, err: *const c_char) {
            let tx = unsafe { &*(context as *const Sender<Result<String, String>>) };
            let error = unsafe { CStr::from_ptr(err).to_str().unwrap_or("").to_string() };
            tx.send(Err(error)).unwrap();
        }

        let callback = CCallback {
            onResponse: Some(response_callback),
            onError: Some(error_callback),
            responseContext: &tx as *const _ as *mut c_void,
            errorContext: &tx as *const _ as *mut c_void,
        };

        method(
            c_data.as_ptr() as *mut c_char,
            c_data.as_bytes().len() as i32,
            callback,
        );

        rx.recv().unwrap_or(Err("No response received".to_string()))
    }

    pub fn get_info(&self) -> Result<String, String> {
        self.call_method(|ptr, len, cb| unsafe { getInfo(ptr, len, cb) }, "{}")
    }

    // Add more methods as needed
}
