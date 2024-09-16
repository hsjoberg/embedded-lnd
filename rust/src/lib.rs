#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

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

    pub fn subscribe_peer_events(
        &self,
    ) -> Result<Receiver<Result<lnrpc::PeerEvent, String>>, String> {
        let (tx, rx) = channel::<Result<lnrpc::PeerEvent, String>>();
        let tx = Arc::new(Mutex::new(tx));

        extern "C" fn response_callback(context: *mut c_void, data: *const c_char, length: c_int) {
            let tx = unsafe {
                &*(context as *const Arc<Mutex<Sender<Result<lnrpc::PeerEvent, String>>>>)
            };
            let response =
                unsafe { std::slice::from_raw_parts(data as *const u8, length as usize) };
            match lnrpc::PeerEvent::decode(response) {
                Ok(update) => {
                    if let Err(e) = tx.lock().unwrap().send(Ok(update)) {
                        eprintln!("Failed to send PeerEvent: {}", e);
                    }
                }
                Err(e) => {
                    if let Err(e) = tx
                        .lock()
                        .unwrap()
                        .send(Err(format!("Failed to decode response: {}", e)))
                    {
                        eprintln!("Failed to send error: {}", e);
                    }
                }
            }
        }

        extern "C" fn error_callback(context: *mut c_void, err: *const c_char) {
            let tx = unsafe {
                &*(context as *const Arc<Mutex<Sender<Result<lnrpc::PeerEvent, String>>>>)
            };
            let error = unsafe { CStr::from_ptr(err).to_str().unwrap_or("").to_string() };
            if let Err(e) = tx.lock().unwrap().send(Err(error)) {
                eprintln!("Failed to send error: {}", e);
            }
        }

        let tx_clone = Arc::clone(&tx);
        let recv_stream = CRecvStream {
            onResponse: Some(response_callback),
            onError: Some(error_callback),
            responseContext: Arc::into_raw(tx_clone) as *mut c_void,
            errorContext: Arc::into_raw(tx) as *mut c_void,
        };

        let request = lnrpc::PeerEventSubscription {};
        let encoded = request.encode_to_vec();
        let c_args = CString::new(encoded).unwrap();

        unsafe {
            let c_args_len = c_args.as_bytes().len() as c_int;
            let c_args_ptr = c_args.into_raw();
            subscribePeerEvents(c_args_ptr, c_args_len, recv_stream);
            let _ = CString::from_raw(c_args_ptr);
        }

        Ok(rx)
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

        std::thread::sleep(std::time::Duration::from_secs(5));

        // Test getInfo function
        match client.get_info(lnrpc::GetInfoRequest {}) {
            Ok(info) => println!("Node info: {:?}", info),
            Err(e) => eprintln!("GetInfo error: {}", e),
        }

        // Test addInvoice function
        let invoice = lnrpc::Invoice {
            memo: "test invoice".to_string(),
            value: 1000,
            ..Default::default()
        };
        match client.add_invoice(invoice) {
            Ok(response) => println!("Invoice added: {:?}", response),
            Err(e) => eprintln!("AddInvoice error: {}", e),
        }

        match client.connect_peer(lnrpc::ConnectPeerRequest {
            addr: Some(lnrpc::LightningAddress {
                pubkey: "02546bfe3778d7f8aea43224337d082bcc4521150569c94c9052413ae5b6599c2d"
                    .to_string(),
                host: "192.168.10.120:9735".to_string(),
                ..Default::default()
            }),
            perm: true,
            ..Default::default()
        }) {
            Ok(response) => println!("Peer connected: {:?}", response),
            Err(e) => eprintln!("ConnectPeer error: {}", e),
        }
    }
}
