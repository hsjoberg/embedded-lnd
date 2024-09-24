use crate::CRecvStream;
use crate::LndClient;
use anyhow::Result;
use lnd_grpc_rust::prost::Message;
use std::marker::PhantomData;

pub type GetResponse<Req, Resp> = Box<dyn Fn(Option<Req>) -> Option<Resp> + Send + Sync + 'static>;
pub type OnRequest<Req> = Box<dyn Fn(Result<Req, String>) + Send + Sync + 'static>;

/// Builder for setting up a bidirectional stream with the LND node.
pub struct BidiStreamBuilder<'a, Req, Resp> {
    client: &'a LndClient,
    stream_func: unsafe extern "C" fn(CRecvStream) -> usize,
    on_request: Option<OnRequest<Req>>,
    get_response: Option<GetResponse<Req, Resp>>,
    _phantom: PhantomData<(Req, Resp)>,
}

impl<'a, Req, Resp> BidiStreamBuilder<'a, Req, Resp>
where
    Req: Message + Default + Clone + 'static,
    Resp: Message + Default + 'static,
{
    pub(crate) fn new(
        client: &'a LndClient,
        stream_func: unsafe extern "C" fn(CRecvStream) -> usize,
    ) -> Self {
        Self {
            client,
            stream_func,
            on_request: None,
            get_response: None,
            _phantom: PhantomData,
        }
    }

    /// Sets the callback for handling incoming requests.
    pub fn on_request<F>(mut self, f: F) -> Self
    where
        F: Fn(Result<Req, String>) + Send + Sync + 'static,
    {
        self.on_request = Some(Box::new(f));
        self
    }

    /// Sets the callback for generating responses to requests.
    pub fn get_response<F>(mut self, f: F) -> Self
    where
        F: Fn(Option<Req>) -> Option<Resp> + Send + Sync + 'static,
    {
        self.get_response = Some(Box::new(f));
        self
    }

    /// Builds and starts the bidirectional stream.
    ///
    /// # Returns
    ///
    /// A `Result` containing the stream pointer or an error.
    pub fn build(self) -> Result<usize> {
        let on_request = self
            .on_request
            .ok_or_else(|| anyhow::anyhow!("on_request callback not set"))?;
        let get_response = self
            .get_response
            .ok_or_else(|| anyhow::anyhow!("get_response callback not set"))?;

        self.client
            .setup_bidirectional_stream(self.stream_func, on_request, get_response)
    }
}
