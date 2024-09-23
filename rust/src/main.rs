use anyhow::{anyhow, Result};
use embedded_lnd::{
    addInvoice, channelAcceptor, connectPeer, getInfo, invoicesSubscribeSingleInvoice,
    subscribePeerEvents, LndClient,
};
use lnd_grpc_rust::{invoicesrpc, lnrpc};
use std::sync::Arc;

fn main() -> Result<()> {
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
    match client.start(start_args) {
        Ok(()) => println!("LND started successfully"),
        Err(e) => {
            eprintln!("Error starting LND: {}", e);
            return Err(anyhow!("Failed to start lnd {:?}", e));
        }
    }

    println!("...........................................");
    std::thread::sleep(std::time::Duration::from_secs(4));

    let info: lnrpc::GetInfoResponse = client.call_lnd_method(lnrpc::GetInfoRequest {}, getInfo)?;

    println!("Getinfo response {:?}", info);

    let invoice = lnrpc::Invoice {
        memo: "test invoice".to_string(),
        value: 1000,
        ..Default::default()
    };

    let invoice_response: lnrpc::AddInvoiceResponse =
        client.call_lnd_method(invoice, addInvoice)?;
    println!("Invoice created: {:?}", invoice_response);

    // Subscribe to single invoice
    let single_invoice_request = invoicesrpc::SubscribeSingleInvoiceRequest {
        r_hash: invoice_response.r_hash,
        ..Default::default()
    };
    client
        .subscribe_events::<lnrpc::Invoice, invoicesrpc::SubscribeSingleInvoiceRequest>(
            invoicesSubscribeSingleInvoice,
        )
        .on_event(|event_result| match event_result {
            Ok(invoice) => println!("Received invoice update: {:?}", invoice),
            Err(e) => eprintln!("Invoice subscription error: {}", e),
        })
        .with_request(single_invoice_request)
        .subscribe()?;

    // Subscribe to peer events
    client
        .subscribe_events::<lnrpc::PeerEvent, lnrpc::PeerEventSubscription>(subscribePeerEvents)
        .on_event(|event_result| match event_result {
            Ok(event) => println!("Received peer event: {:?}", event.pub_key),
            Err(e) => eprintln!("Peer event error: {}", e),
        })
        .with_request(lnrpc::PeerEventSubscription::default())
        .subscribe()?;

    // Setup channel acceptor
    let acceptor = client
        .bidi_stream::<lnrpc::ChannelAcceptRequest, lnrpc::ChannelAcceptResponse>(channelAcceptor)
        .on_request(|request_result| {
            match request_result {
                Ok(request) => {
                    println!("Received channel request: {:?}", request);
                    // Your logic here
                }
                Err(e) => println!("Error: {}", e),
            }
        })
        .get_response(|request| {
            request.map(|req| {
                lnrpc::ChannelAcceptResponse {
                    accept: false,
                    pending_chan_id: req.pending_chan_id,
                    error: "i won't accept your channel".to_string(),
                    // Set other fields as needed
                    ..Default::default()
                }
            })
        })
        .build()?;

    let mut i = 0;

    loop {
        let connect_request = lnrpc::ConnectPeerRequest {
            addr: Some(lnrpc::LightningAddress {
                pubkey: "02546bfe3778d7f8aea43224337d082bcc4521150569c94c9052413ae5b6599c2d"
                    .to_string(),
                host: "localhost:9735".to_string(),
            }),
            perm: true,
            timeout: 60,
        };
        let connect_response: Result<lnrpc::ConnectPeerResponse> =
            client.call_lnd_method(connect_request, connectPeer);
        println!("Peer connection result: {:?}", connect_response);

        i = i + 1;
        // Sleep for 3 seconds before the next iteration
        std::thread::sleep(std::time::Duration::from_secs(3));

        if i == 100 {
            break;
        }
    }

    client.stop_stream(acceptor)?;

    Ok(())
}
