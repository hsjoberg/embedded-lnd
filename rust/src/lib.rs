#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use std::ffi::CString;
use std::os::raw::{c_char, c_void};

pub struct LndClient;

impl LndClient {
    pub fn new() -> Self {
        LndClient
    }

    pub fn start(&self, args: &str) -> Result<(), String> {
        let c_args = CString::new(args).unwrap();

        extern "C" fn response_callback(_context: *mut c_void, data: *const c_char, _length: ::std::os::raw::c_int) {
            unsafe {
                let response = std::ffi::CStr::from_ptr(data).to_string_lossy().into_owned();
                println!("Response: {}", response);
            }
        }

        extern "C" fn error_callback(_context: *mut c_void, error: *const c_char) {
            unsafe {
                let error_str = std::ffi::CStr::from_ptr(error).to_string_lossy().into_owned();
                eprintln!("Error: {}", error_str);
            }
        }

        let callback = CCallback {
            onResponse: Some(response_callback),
            onError: Some(error_callback),
            responseContext: std::ptr::null_mut(),
            errorContext: std::ptr::null_mut(),
        };

        unsafe {
            start(c_args.as_ptr() as *mut c_char, callback);
        }
        Ok(())
    }

    // Implement other methods as needed
}
