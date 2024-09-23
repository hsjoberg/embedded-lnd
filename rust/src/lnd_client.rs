use crate::bidi_stream::BidiStreamBuilder;
use crate::event_subscription::EventSubscriptionBuilder;
use crate::{start, CCallback, CRecvStream, SendStreamC, StopStreamC};
use anyhow::{Context, Result};
use lnd_grpc_rust::prost::Message;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;
use std::sync::mpsc::{channel, Sender};
use std::sync::Once;
use std::sync::{Arc, Mutex};
use std::time::Duration;

static INIT: Once = Once::new();
static mut CALLBACK: Option<CCallback> = None;
type CallbackFn = Box<dyn Fn(Vec<u8>) + Send + Sync>;

static GLOBAL_CALLBACKS: Lazy<Mutex<HashMap<usize, CallbackFn>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static NEXT_ID: Lazy<Mutex<usize>> = Lazy::new(|| Mutex::new(0));

/// The main client for interacting with the LND node.
pub struct LndClient;

impl LndClient {
    /// Creates a new instance of the LndClient.
    pub fn new() -> Self {
        LndClient
    }

    /// Initiates a bidirectional stream with the LND node.
    ///
    /// # Arguments
    ///
    /// * `stream_func` - The FFI function to set up the stream.
    ///
    /// # Returns
    ///
    /// A `BidiStreamBuilder` to configure and build the stream.
    pub fn bidi_stream<Req, Resp>(
        &self,
        stream_func: unsafe extern "C" fn(CRecvStream) -> usize,
    ) -> BidiStreamBuilder<Req, Resp>
    where
        Req: Message + Default + Clone + 'static,
        Resp: Message + Default + 'static,
    {
        BidiStreamBuilder::new(self, stream_func)
    }

    /// Initiates an event subscription with the LND node.
    ///
    /// # Arguments
    ///
    /// * `subscribe_func` - The FFI function to set up the subscription.
    ///
    /// # Returns
    ///
    /// An `EventSubscriptionBuilder` to configure and build the subscription.
    pub fn subscribe_events<E, R>(
        &self,
        subscribe_func: unsafe extern "C" fn(*mut c_char, c_int, CRecvStream) -> (),
    ) -> EventSubscriptionBuilder<E, R>
    where
        E: Message + Default + 'static,
        R: Message,
    {
        EventSubscriptionBuilder::new(self, subscribe_func)
    }

    /// Starts the LND node with the given arguments.
    ///
    /// # Arguments
    ///
    /// * `args` - The command-line arguments to pass to the LND node.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure.
    pub fn start(&self, args: &str) -> Result<()> {
        let c_args = CString::new(args).context("Failed to create CString from args")?;

        extern "C" fn response_callback(
            _context: *mut c_void,
            data: *const c_char,
            _length: c_int,
        ) {
            unsafe {
                CStr::from_ptr(data).to_string_lossy().into_owned();
            }
        }

        extern "C" fn error_callback(_context: *mut c_void, error: *const c_char) {
            unsafe {
                CStr::from_ptr(error).to_string_lossy().into_owned();
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
    /// Stops a bidirectional stream.
    ///
    /// # Arguments
    ///
    /// * `stream_ptr` - The pointer to the stream to stop.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure.
    pub fn stop_stream(&self, stream_ptr: usize) -> Result<()> {
        let result = unsafe { StopStreamC(stream_ptr) };
        if result == 0 {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Failed to stop stream. Error code: {}",
                result
            ))
        }
    }

    /// Calls an LND method.
    ///
    /// # Arguments
    ///
    /// * `request` - The request message.
    /// * `lnd_func` - The FFI function to call.
    ///
    /// # Returns
    ///
    /// A `Result` containing the response or an error.
    pub fn call_lnd_method<Req, Resp>(
        &self,
        request: Req,
        lnd_func: unsafe extern "C" fn(*mut c_char, c_int, CCallback) -> (),
    ) -> Result<Resp>
    where
        Req: Message,
        Resp: Message + Default,
    {
        let encoded = request.encode_to_vec();
        let c_args =
            CString::new(encoded).context("Failed to create CString from encoded request")?;
        let (tx, rx) = channel::<Result<Vec<u8>>>();

        extern "C" fn response_callback(context: *mut c_void, data: *const c_char, length: c_int) {
            let tx = unsafe { &*(context as *const Sender<Result<Vec<u8>>>) };
            let response =
                unsafe { std::slice::from_raw_parts(data as *const u8, length as usize) };
            tx.send(Ok(response.to_vec())).unwrap();
        }

        extern "C" fn error_callback(context: *mut c_void, err: *const c_char) {
            let tx = unsafe { &*(context as *const Sender<Result<Vec<u8>>>) };
            let error = unsafe { CStr::from_ptr(err).to_str().unwrap_or("").to_string() };
            tx.send(Err(anyhow::anyhow!(error))).unwrap();
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

        rx.recv_timeout(Duration::from_secs(30))
            .context("Timeout waiting for response")?
            .and_then(|bytes| {
                Resp::decode(bytes.as_slice())
                    .map_err(|e| anyhow::anyhow!("Failed to decode response: {}", e))
            })
    }

    pub(crate) fn setup_bidirectional_stream<Req, Resp, F, G>(
        &self,
        stream_func: unsafe extern "C" fn(CRecvStream) -> usize,
        on_request: F,
        get_response: G,
    ) -> Result<usize>
    where
        Req: Message + Default + Clone + 'static,
        Resp: Message + Default + 'static,
        F: Fn(Result<Req, String>) + Send + Sync + 'static,
        G: Fn(Option<Req>) -> Option<Resp> + Send + Sync + 'static,
    {
        struct Context<Req, Resp> {
            on_request: Arc<Mutex<dyn Fn(Result<Req, String>) + Send + Sync>>,
            get_response: Arc<Mutex<dyn Fn(Option<Req>) -> Option<Resp> + Send + Sync>>,
            send_stream: Mutex<Option<usize>>,
            last_request: Mutex<Option<Req>>,
        }

        let context = Box::new(Context {
            on_request: Arc::new(Mutex::new(on_request)),
            get_response: Arc::new(Mutex::new(get_response)),
            send_stream: Mutex::new(None),
            last_request: Mutex::new(None),
        });
        let context_ptr = Box::into_raw(context);

        extern "C" fn request_callback<Req: Message + Default + Clone, Resp: Message + Default>(
            context: *mut c_void,
            data: *const c_char,
            length: c_int,
        ) {
            let context = unsafe { &*(context as *const Context<Req, Resp>) };
            let request_data =
                unsafe { std::slice::from_raw_parts(data as *const u8, length as usize) };

            match Req::decode(request_data) {
                Ok(request) => {
                    context.on_request.lock().unwrap()(Ok(request.clone()));
                    *context.last_request.lock().unwrap() = Some(request.clone());

                    if let Some(response) = context.get_response.lock().unwrap()(Some(request)) {
                        let encoded_response = response.encode_to_vec();
                        if let Some(send_stream) = *context.send_stream.lock().unwrap() {
                            let c_data =
                                CString::new(encoded_response).expect("CString::new failed");
                            unsafe {
                                SendStreamC(
                                    send_stream,
                                    c_data.clone().into_raw(),
                                    c_data.as_bytes().len() as c_int,
                                )
                            };
                        }
                    }
                }
                Err(e) => context.on_request.lock().unwrap()(Err(format!(
                    "Failed to decode request: {}",
                    e
                ))),
            }
        }

        extern "C" fn error_callback<Req: Message + Default + Clone, Resp: Message + Default>(
            context: *mut c_void,
            err: *const c_char,
        ) {
            let context = unsafe { &*(context as *const Context<Req, Resp>) };
            let error = unsafe {
                CStr::from_ptr(err)
                    .to_str()
                    .unwrap_or("Unknown error")
                    .to_string()
            };
            context.on_request.lock().unwrap()(Err(error));
        }

        let recv_stream = CRecvStream {
            onResponse: Some(request_callback::<Req, Resp>),
            onError: Some(error_callback::<Req, Resp>),
            responseContext: context_ptr as *mut c_void,
            errorContext: context_ptr as *mut c_void,
        };

        let send_stream = unsafe { stream_func(recv_stream) };

        if send_stream == 0 {
            unsafe {
                let _ = Box::from_raw(context_ptr);
            };
            Err(anyhow::anyhow!("Failed to create send stream"))
        } else {
            unsafe {
                (*context_ptr)
                    .send_stream
                    .lock()
                    .unwrap()
                    .replace(send_stream);
            }
            Ok(send_stream)
        }
    }

    pub fn subscribe_to_events<E, F, R>(
        &self,
        subscribe_func: unsafe extern "C" fn(*mut c_char, c_int, CRecvStream) -> (),
        callback: F,
        request: R,
    ) -> Result<()>
    where
        E: Message + Default + 'static,
        F: Fn(Result<E, String>) + Send + Sync + 'static,
        R: Message,
    {
        let id = {
            let mut id = NEXT_ID.lock().unwrap();
            *id += 1;
            *id
        };

        let callback_wrapper = Box::new(move |data: Vec<u8>| match E::decode(data.as_slice()) {
            Ok(event) => callback(Ok(event)),
            Err(e) => callback(Err(format!("Failed to decode event: {}", e))),
        });

        GLOBAL_CALLBACKS
            .lock()
            .unwrap()
            .insert(id, callback_wrapper);

        extern "C" fn response_callback(context: *mut c_void, data: *const c_char, length: c_int) {
            let id = context as usize;
            let response =
                unsafe { std::slice::from_raw_parts(data as *const u8, length as usize) };
            if let Some(callback) = GLOBAL_CALLBACKS.lock().unwrap().get(&id) {
                callback(response.to_vec());
            }
        }

        extern "C" fn error_callback(context: *mut c_void, err: *const c_char) {
            let id = context as usize;
            let error = unsafe {
                CStr::from_ptr(err)
                    .to_str()
                    .unwrap_or("Unknown error")
                    .to_string()
            };
            if let Some(callback) = GLOBAL_CALLBACKS.lock().unwrap().get(&id) {
                callback(error.into_bytes());
            }
        }

        let recv_stream = CRecvStream {
            onResponse: Some(response_callback),
            onError: Some(error_callback),
            responseContext: id as *mut c_void,
            errorContext: id as *mut c_void,
        };

        let encoded = request.encode_to_vec();
        let c_args =
            CString::new(encoded).context("Failed to create CString from encoded request")?;
        unsafe {
            let c_args_len = c_args.as_bytes().len() as c_int;
            let c_args_ptr = c_args.into_raw();
            subscribe_func(c_args_ptr, c_args_len, recv_stream);
            let _ = CString::from_raw(c_args_ptr);
        }
        Ok(())
    }
}
