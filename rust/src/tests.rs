// tests.rs

use crate::{CCallback, CRecvStream, LndClient};
use lnd_grpc_rust::lnrpc;
use lnd_grpc_rust::prost::Message;
use std::ffi::CString;
use std::os::raw::{c_char, c_int};
use std::sync::mpsc;
use std::sync::{Arc, LazyLock, Mutex};

// Mock LND struct
struct MockLnd {
    get_info_response: Mutex<Option<lnrpc::GetInfoResponse>>,
    add_invoice_response: Mutex<Option<lnrpc::AddInvoiceResponse>>,
    connect_peer_response: Mutex<Option<Result<lnrpc::ConnectPeerResponse, String>>>,
    start_response: Mutex<Option<Result<(), String>>>,
    peer_event: Mutex<Option<lnrpc::PeerEvent>>,
}

static MOCK_LND: LazyLock<Arc<MockLnd>> = LazyLock::new(|| Arc::new(MockLnd::new()));

impl MockLnd {
    fn new() -> Self {
        MockLnd {
            get_info_response: Mutex::new(None),
            add_invoice_response: Mutex::new(None),
            connect_peer_response: Mutex::new(None),
            start_response: Mutex::new(None),
            peer_event: Mutex::new(None),
        }
    }

    fn set_get_info_response(&self, response: lnrpc::GetInfoResponse) {
        *self.get_info_response.lock().unwrap() = Some(response);
    }

    fn set_add_invoice_response(&self, response: lnrpc::AddInvoiceResponse) {
        *self.add_invoice_response.lock().unwrap() = Some(response);
    }

    fn set_connect_peer_response(&self, response: Result<lnrpc::ConnectPeerResponse, String>) {
        *self.connect_peer_response.lock().unwrap() = Some(response);
    }

    fn set_start_response(&self, response: Result<(), String>) {
        *self.start_response.lock().unwrap() = Some(response);
    }

    fn set_peer_event(&self, event: lnrpc::PeerEvent) {
        *self.peer_event.lock().unwrap() = Some(event);
    }
}

#[no_mangle]
pub unsafe extern "C" fn mock_start(_args: *mut c_char, callback: CCallback) -> c_int {
    if let Some(response) = MOCK_LND.start_response.lock().unwrap().clone() {
        match response {
            Ok(()) => {
                if let Some(on_response) = callback.onResponse {
                    on_response(callback.responseContext, std::ptr::null(), 0);
                }
                0
            }
            Err(err) => {
                let c_err = CString::new(err).unwrap();
                if let Some(on_error) = callback.onError {
                    on_error(callback.errorContext, c_err.as_ptr());
                }
                1
            }
        }
    } else {
        1
    }
}

// Mock event subscription function
unsafe extern "C" fn mock_subscribe_peer_events(
    _request: *mut c_char,
    _length: c_int,
    recv_stream: CRecvStream,
) {
    if let Some(event) = MOCK_LND.peer_event.lock().unwrap().clone() {
        let encoded = event.encode_to_vec();
        if let Some(on_response) = recv_stream.onResponse {
            on_response(
                recv_stream.responseContext,
                encoded.as_ptr() as *const c_char,
                encoded.len() as c_int,
            );
        }
    }
}

// Mock LND methods
unsafe extern "C" fn mock_get_info(
    _data: *mut std::os::raw::c_char,
    _length: std::os::raw::c_int,
    callback: CCallback,
) {
    if let Some(response) = MOCK_LND.get_info_response.lock().unwrap().clone() {
        let encoded = response.encode_to_vec();
        if let Some(on_response) = callback.onResponse {
            on_response(
                callback.responseContext,
                encoded.as_ptr() as *const std::os::raw::c_char,
                encoded.len() as std::os::raw::c_int,
            );
        }
    }
}

unsafe extern "C" fn mock_add_invoice(
    _data: *mut std::os::raw::c_char,
    _length: std::os::raw::c_int,
    callback: CCallback,
) {
    if let Some(response) = MOCK_LND.add_invoice_response.lock().unwrap().clone() {
        let encoded = response.encode_to_vec();
        if let Some(on_response) = callback.onResponse {
            on_response(
                callback.responseContext,
                encoded.as_ptr() as *const std::os::raw::c_char,
                encoded.len() as std::os::raw::c_int,
            );
        }
    }
}

unsafe extern "C" fn mock_connect_peer(
    _data: *mut std::os::raw::c_char,
    _length: std::os::raw::c_int,
    callback: CCallback,
) {
    if let Some(response) = MOCK_LND.connect_peer_response.lock().unwrap().clone() {
        match response {
            Ok(resp) => {
                let encoded = resp.encode_to_vec();
                if let Some(on_response) = callback.onResponse {
                    on_response(
                        callback.responseContext,
                        encoded.as_ptr() as *const std::os::raw::c_char,
                        encoded.len() as std::os::raw::c_int,
                    );
                }
            }
            Err(err) => {
                let c_err = CString::new(err).unwrap();
                if let Some(on_error) = callback.onError {
                    on_error(callback.errorContext, c_err.as_ptr());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_start() {
        let client = Arc::new(LndClient::new());
        MOCK_LND.set_start_response(Ok(()));

        let start_args = "--lnddir=./lnd --noseedbackup";
        let result = client.start(start_args);
        assert!(result.is_ok(), "Expected Ok, got Err: {:?}", result.err());
    }

    #[test]
    fn test_event_subscription() {
        let client = Arc::new(LndClient::new());

        let expected_event = lnrpc::PeerEvent {
            pub_key: "02546bfe3778d7f8aea43224337d082bcc4521150569c94c9052413ae5b6599c2d"
                .to_string(),
            r#type: 1, // Connected
            ..Default::default()
        };
        MOCK_LND.set_peer_event(expected_event.clone());

        let (event_sender, event_receiver) = mpsc::channel();
        let event_sender = Arc::new(Mutex::new(event_sender));

        let subscription_result = client
            .subscribe_events::<lnrpc::PeerEvent, lnrpc::PeerEventSubscription>(
                mock_subscribe_peer_events,
            )
            .on_event(move |event_result| {
                let sender = event_sender.lock().unwrap();
                if let Ok(event) = event_result {
                    sender.send(event).unwrap();
                }
            })
            .with_request(lnrpc::PeerEventSubscription::default())
            .subscribe();

        assert!(
            subscription_result.is_ok(),
            "Failed to subscribe: {:?}",
            subscription_result.err()
        );

        // Wait for the event to be received
        let received_event = event_receiver
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("Timed out waiting for event");

        assert_eq!(received_event.pub_key, expected_event.pub_key);
        assert_eq!(received_event.r#type, expected_event.r#type);
    }

    #[test]
    fn test_get_info() {
        let client = Arc::new(LndClient::new());
        let expected_response = lnrpc::GetInfoResponse {
            version: "0.18.3-beta".to_string(),
            identity_pubkey: "02546bfe3778d7f8aea43224337d082bcc4521150569c94c9052413ae5b6599c2d"
                .to_string(),
            ..Default::default()
        };
        MOCK_LND.set_get_info_response(expected_response.clone());

        let result: lnrpc::GetInfoResponse = client
            .call_lnd_method(lnrpc::GetInfoRequest {}, mock_get_info)
            .unwrap();
        assert_eq!(result, expected_response);
    }

    #[test]
    fn test_add_invoice() {
        let client = Arc::new(LndClient::new());
        let expected_response = lnrpc::AddInvoiceResponse {
            r_hash: vec![1, 2, 3],
            payment_request: "test_payment_request".to_string(),
            ..Default::default()
        };
        MOCK_LND.set_add_invoice_response(expected_response.clone());

        let invoice = lnrpc::Invoice {
            memo: "test invoice".to_string(),
            value: 1000,
            ..Default::default()
        };

        let result: lnrpc::AddInvoiceResponse =
            client.call_lnd_method(invoice, mock_add_invoice).unwrap();
        assert_eq!(result, expected_response);
    }

    #[test]
    fn test_connect_peer_success() {
        let client = Arc::new(LndClient::new());
        let expected_response = lnrpc::ConnectPeerResponse {};
        MOCK_LND.set_connect_peer_response(Ok(expected_response.clone()));

        let connect_request = lnrpc::ConnectPeerRequest {
            addr: Some(lnrpc::LightningAddress {
                pubkey: "02546bfe3778d7f8aea43224337d082bcc4521150569c94c9052413ae5b6599c2d"
                    .to_string(),
                host: "localhost:9735".to_string(),
            }),
            perm: true,
            timeout: 60,
        };

        let result: anyhow::Result<lnrpc::ConnectPeerResponse> =
            client.call_lnd_method(connect_request, mock_connect_peer);
        assert!(result.is_ok(), "Expected Ok, got Err: {:?}", result.err());
        assert_eq!(result.unwrap(), expected_response);
    }
}
