mod lnd_types;

use libloading::{Library, Symbol};
use std::ffi::{CString, c_void};
use std::os::raw::{c_char, c_int};
use tokio::sync::oneshot;
use lnd_types::{CCallback, LndFuncPtr};

pub struct LndLibrary {
    lib: Library,
}

impl LndLibrary {
    pub fn new(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let lib = unsafe { Library::new(path)? };
        Ok(Self { lib })
    }

    fn get_function(&self, name: &str) -> Result<Symbol<LndFuncPtr>, Box<dyn std::error::Error>> {
        unsafe { Ok(self.lib.get(name.as_bytes())?) }
    }

    pub async fn call_lnd_function(&self, func_name: &str, args: &str) -> Result<String, String> {
        let func = self.get_function(func_name).map_err(|e| e.to_string())?;
        let (tx, rx) = oneshot::channel();

        let args_cstring = CString::new(args).map_err(|e| e.to_string())?;

        extern "C" fn response_callback(context: *mut c_void, data: *const c_char, length: c_int) {
            let tx = unsafe { Box::from_raw(context as *mut oneshot::Sender<Result<String, String>>) };
            let result = unsafe { std::slice::from_raw_parts(data as *const u8, length as usize) };
            let result_str = String::from_utf8_lossy(result).to_string();
            let _ = tx.send(Ok(result_str));
        }

        extern "C" fn error_callback(context: *mut c_void, error: *const c_char) {
            let tx = unsafe { Box::from_raw(context as *mut oneshot::Sender<Result<String, String>>) };
            let error_str = unsafe { CString::from_raw(error as *mut c_char).to_string_lossy().to_string() };
            let _ = tx.send(Err(error_str));
        }

        let tx_box = Box::new(tx);
        let tx_ptr = Box::into_raw(tx_box);

        let callback = CCallback {
            response_callback,
            error_callback,
            response_context: tx_ptr as *mut c_void,
            error_context: tx_ptr as *mut c_void,
        };

        unsafe {
            func(args_cstring.into_raw(), callback);
        }

        rx.await.map_err(|e| e.to_string())?
    }
}

// Convenience functions for common LND operations
impl LndLibrary {
    pub async fn start(&self, args: &str) -> Result<String, String> {
        self.call_lnd_function("start", args).await
    }

    // Add more convenience functions for other LND operations as needed
}
