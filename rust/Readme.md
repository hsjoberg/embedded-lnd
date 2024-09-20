# embedded-lnd

`embedded-lnd` is a Rust library that provides a high-level, safe interface for interacting with an embedded LND (Lightning Network Daemon) node. It allows developers to easily integrate Lightning Network functionality into their Rust applications.


## Installation

```
cargo add embedded-lnd
```

## Usage

Here's a basic example of how to use `embedded-lnd`:

```rust
use embedded_lnd::{LndClient, lnrpc, getInfo};
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Arc::new(LndClient::new());

    // Start LND
    client.start("--bitcoin.active --bitcoin.regtest")?;

    // Get node info
    let info: lnrpc::GetInfoResponse = client.call_lnd_method(lnrpc::GetInfoRequest {}, getInfo)?;
    println!("Node info: {:?}", info);

    Ok(())
}
```

## Examples

### Creating an Invoice

```rust
use embedded_lnd::{LndClient, lnrpc, addInvoice};

let client = Arc::new(LndClient::new());

let invoice = lnrpc::Invoice {
    memo: "Test invoice".to_string(),
    value: 1000,
    ..Default::default()
};

let response: lnrpc::AddInvoiceResponse = client.call_lnd_method(invoice, addInvoice)?;
println!("Invoice created: {:?}", response);
```

### Subscribing to Events

```rust
use embedded_lnd::{LndClient, lnrpc, subscribePeerEvents};

let client = Arc::new(LndClient::new());

client.subscribe_events::<lnrpc::PeerEvent, lnrpc::PeerEventSubscription>(subscribePeerEvents)
    .on_event(|event_result| match event_result {
        Ok(event) => println!("Received peer event: {:?}", event.pub_key),
        Err(e) => eprintln!("Peer event error: {}", e),
    })
    .with_request(lnrpc::PeerEventSubscription::default())
    .subscribe()?;
```

### Setting up a Bidirectional Stream

```rust
use embedded_lnd::{LndClient, lnrpc, channelAcceptor};

let client = Arc::new(LndClient::new());

let acceptor = client
    .bidi_stream::<lnrpc::ChannelAcceptRequest, lnrpc::ChannelAcceptResponse>(channelAcceptor)
    .on_request(|request_result| {
        match request_result {
            Ok(request) => println!("Received channel request: {:?}", request),
            Err(e) => println!("Error: {}", e),
        }
    })
    .get_response(|request| {
        request.map(|req| {
            lnrpc::ChannelAcceptResponse {
                accept: false,
                pending_chan_id: req.pending_chan_id,
                error: "Channel not accepted".to_string(),
                ..Default::default()
            }
        })
    })
    .build()?;
```

## API Documentation

For detailed API documentation, run `cargo doc --open` in your project directory.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the [MIT License](LICENSE).

## Disclaimer

This software is in beta and should not be used in production environments without proper review and testing.

## Contact

If you have any questions or feedback, please open an issue on the GitHub repository.
