use std::os::raw::{c_char, c_int};

#[repr(C)]
pub struct CCallback {
    pub response_callback: extern "C" fn(*mut std::ffi::c_void, *const c_char, c_int),
    pub error_callback: extern "C" fn(*mut std::ffi::c_void, *const c_char),
    pub response_context: *mut std::ffi::c_void,
    pub error_context: *mut std::ffi::c_void,
}

pub type LndFuncPtr = unsafe extern "C" fn(*mut c_char, CCallback) -> ();
