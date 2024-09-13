use libloading::{Library, Symbol};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::ffi::{c_void, CString};
use std::os::raw::{c_char, c_int};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::oneshot;

mod lnd_types;
use lnd_types::{CCallback, LndFuncPtr};

pub struct LndLibrary {
    lib: Arc<Library>,
    functions: Arc<RwLock<HashMap<String, Symbol<'static, LndFuncPtr>>>>,
    is_valid: Arc<AtomicBool>,
}

impl LndLibrary {
    pub fn new(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let lib = Arc::new(unsafe { Library::new(path)? });
        Ok(Self {
            lib: Arc::clone(&lib),
            functions: Arc::new(RwLock::new(HashMap::new())),
            is_valid: Arc::new(AtomicBool::new(true)),
        })
    }

    fn get_function(&self, name: &str) -> Result<Symbol<LndFuncPtr>, Box<dyn std::error::Error>> {
        if !self.is_valid.load(Ordering::SeqCst) {
            return Err("LndLibrary is no longer valid".into());
        }

        let mut functions = self.functions.write();
        if let Some(func) = functions.get(name) {
            Ok(func.clone())
        } else {
            let func: Symbol<LndFuncPtr> = unsafe { self.lib.get(name.as_bytes())? };
            let func: Symbol<'static, LndFuncPtr> = unsafe { std::mem::transmute(func) };
            functions.insert(name.to_string(), func.clone());
            Ok(func)
        }
    }

    pub async fn call_lnd_function(&self, func_name: &str, args: &str) -> Result<String, String> {
        if !self.is_valid.load(Ordering::SeqCst) {
            return Err("LndLibrary is no longer valid".to_string());
        }

        let func = self.get_function(func_name).map_err(|e| e.to_string())?;
        let (tx, rx) = oneshot::channel();
        let args_cstring = CString::new(args).map_err(|e| e.to_string())?;

        let tx = Arc::new(parking_lot::Mutex::new(Some(tx)));
        let tx_ptr = Arc::into_raw(tx) as *mut c_void;

        let callback = CCallback {
            response_callback: Self::response_callback,
            error_callback: Self::error_callback,
            response_context: tx_ptr,
            error_context: tx_ptr,
        };

        unsafe {
            func(args_cstring.into_raw(), callback);
        }

        rx.await.map_err(|e| e.to_string())?
    }

    extern "C" fn response_callback(context: *mut c_void, data: *const c_char, length: c_int) {
        let tx = unsafe {
            Arc::from_raw(
                context
                    as *const parking_lot::Mutex<Option<oneshot::Sender<Result<String, String>>>>,
            )
        };
        let result = unsafe { std::slice::from_raw_parts(data as *const u8, length as usize) };
        let result_str = String::from_utf8_lossy(result).to_string();
        if let Some(sender) = tx.lock().take() {
            let _ = sender.send(Ok(result_str));
        }
        // Prevent the Arc from being dropped
        let _ = Arc::into_raw(tx);
    }

    extern "C" fn error_callback(context: *mut c_void, error: *const c_char) {
        let tx = unsafe {
            Arc::from_raw(
                context
                    as *const parking_lot::Mutex<Option<oneshot::Sender<Result<String, String>>>>,
            )
        };
        let error_str = unsafe {
            CString::from_raw(error as *mut c_char)
                .to_string_lossy()
                .to_string()
        };
        if let Some(sender) = tx.lock().take() {
            let _ = sender.send(Err(error_str));
        }
        // Prevent the Arc from being dropped
        let _ = Arc::into_raw(tx);
    }

    pub async fn start(&self, args: &str) -> Result<String, String> {
        self.call_lnd_function("start", args).await
    }

    pub async fn get_info(&self) -> Result<String, String> {
        self.call_lnd_function("getInfo", "").await
    }
}

impl Drop for LndLibrary {
    fn drop(&mut self) {
        self.is_valid.store(false, Ordering::SeqCst);
    }
}

// Implement Send and Sync for LndLibrary
unsafe impl Send for LndLibrary {}
unsafe impl Sync for LndLibrary {}
