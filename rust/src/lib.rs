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

pub struct LndClient;

static INIT: Once = Once::new();
static mut CALLBACK: Option<CCallback> = None;

macro_rules! generate_lnd_function {
    ($func_name:ident) => {
        pub fn $func_name(&self, args: &str) -> Result<String, String> {
            let c_args = CString::new(args).unwrap();
            let (tx, rx) = channel();

            extern "C" fn response_callback(
                context: *mut c_void,
                data: *const c_char,
                length: c_int,
            ) {
                let tx = unsafe { &*(context as *const Sender<Result<String, String>>) };
                let response =
                    unsafe { std::slice::from_raw_parts(data as *const u8, length as usize) };
                let response_str = String::from_utf8_lossy(response).into_owned();
                tx.send(Ok(response_str)).unwrap();
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

            unsafe {
                $func_name(
                    c_args.as_ptr() as *mut c_char,
                    c_args.as_bytes().len() as c_int,
                    callback,
                );
            }

            match rx.recv_timeout(Duration::from_secs(30)) {
                Ok(result) => result,
                Err(_) => Err("Timeout waiting for response".to_string()),
            }
        }
    };
}

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
            start(c_args.as_ptr() as *mut c_char, CALLBACK.unwrap());
        }
        Ok(())
    }

    generate_lnd_function!(getInfo);
    generate_lnd_function!(walletBalance);
    generate_lnd_function!(channelBalance);
    generate_lnd_function!(listChannels);
    generate_lnd_function!(pendingChannels);
    generate_lnd_function!(listPayments);
    generate_lnd_function!(decodePayReq);
    generate_lnd_function!(addInvoice);
    generate_lnd_function!(lookupInvoice);
    generate_lnd_function!(listInvoices);
    // Add more functions here as needed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lnd_client() {
        let client = LndClient::new();

        let start_args = "--lnddir=./lnd \
            --noseedbackup \
            --nolisten \
            --bitcoin.active \
            --bitcoin.regtest \
            --bitcoin.node=neutrino \
            --feeurl=\"https://nodes.lightning.computer/fees/v1/btc-fee-estimates.json\" \
            --routing.assumechanvalid \
            --tlsdisableautofill \
            --db.bolt.auto-compact \
            --db.bolt.auto-compact-min-age=0 \
            --neutrino.connect=localhost:19444";

        // Test start function
        match client.start(start_args) {
            Ok(()) => println!("LND started successfully"),
            Err(e) => eprintln!("Start error: {}", e),
        }

        // Test getInfo function
        match client.getInfo("") {
            Ok(info) => println!("Node info: {}", info),
            Err(e) => eprintln!("GetInfo error: {}", e),
        }

        // Add more tests for other functions as needed
    }
}
