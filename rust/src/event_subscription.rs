use crate::CRecvStream;
use crate::LndClient;
use anyhow::Result;
use lnd_grpc_rust::prost::Message;
use std::ffi::c_char;
use std::marker::PhantomData;
use std::os::raw::c_int;

/// Builder for setting up an event subscription with the LND node.
pub struct EventSubscriptionBuilder<'a, E, R> {
    client: &'a LndClient,
    subscribe_func: unsafe extern "C" fn(*mut c_char, c_int, CRecvStream) -> (),
    callback: Option<Box<dyn Fn(Result<E, String>) + Send + Sync + 'static>>,
    request: Option<R>,
    _phantom: PhantomData<E>,
}

impl<'a, E, R> EventSubscriptionBuilder<'a, E, R>
where
    E: Message + Default + 'static,
    R: Message,
{
    pub(crate) fn new(
        client: &'a LndClient,
        subscribe_func: unsafe extern "C" fn(*mut c_char, c_int, CRecvStream) -> (),
    ) -> Self {
        Self {
            client,
            subscribe_func,
            callback: None,
            request: None,
            _phantom: PhantomData,
        }
    }

    /// Sets the callback for handling incoming events.
    pub fn on_event<F>(mut self, f: F) -> Self
    where
        F: Fn(Result<E, String>) + Send + Sync + 'static,
    {
        self.callback = Some(Box::new(f));
        self
    }

    /// Sets the subscription request.
    pub fn with_request(mut self, request: R) -> Self {
        self.request = Some(request);
        self
    }

    /// Builds and starts the event subscription.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure.
    pub fn subscribe(self) -> Result<()> {
        let callback = self
            .callback
            .ok_or_else(|| anyhow::anyhow!("Event callback not set"))?;
        let request = self
            .request
            .ok_or_else(|| anyhow::anyhow!("Subscription request not set"))?;

        self.client
            .subscribe_to_events(self.subscribe_func, callback, request)
    }
}
