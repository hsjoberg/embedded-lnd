# embedded-lnd 🦀


- `embedded-lnd` is a Rust library that provides a high-level, safe interface for interacting with an embedded LND node.
- You can compile LND using CGO for Linux, MacOS and Windows and embed it into your application and interact with it.
- At compile time, `build.rs` takes the `liblnd.h` file and generates a `bindings.rs` file for Rust <-> C FFI.
- You can then import protobuf types and LND grpc methods from the library and just call them.
- Refer to [LND API](https://lightning.engineering/api-docs/api/lnd/) docs for methods and types.


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

    // Start LND
    client.start(start_args)?;

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

This project is licensed under the [MIT License](https://github.com/hsjoberg/embedded-lnd/blob/master/LICENSE).

## Disclaimer

This software is in beta and should not be used in production environments without proper review and testing.

## Contact

If you have any questions or feedback, please open an issue on the GitHub repository.
