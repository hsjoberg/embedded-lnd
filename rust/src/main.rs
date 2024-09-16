use lnd_grpc_rust::lnrpc;
use lnd_rust_wrapper::LndClient;
use std::sync::Arc;
use std::thread;

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
    match client.start(start_args) {
        Ok(()) => println!("LND started successfully"),
        Err(e) => {
            eprintln!("Error starting LND: {}", e);
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)));
        }
    }

    println!("...........................................");
    std::thread::sleep(std::time::Duration::from_secs(8));

    match client.get_info(lnrpc::GetInfoRequest {}) {
        Ok(info) => {
            println!("LND Info: {:?}", info);
        }
        Err(e) => {
            eprintln!("Error getting LND info: {}", e);

            std::thread::sleep(std::time::Duration::from_secs(5));
        }
    }

    // Test addInvoice function
    let invoice = lnrpc::Invoice {
        memo: "test invoice".to_string(),
        value: 1000,
        ..Default::default()
    };
    match client.add_invoice(invoice) {
        Ok(response) => println!("Invoice added: {:?}", response.payment_addr),
        Err(e) => eprintln!("AddInvoice error: {}", e),
    }

    let client_clone = Arc::clone(&client);
    let _handle = thread::spawn(move || match client_clone.subscribe_peer_events() {
        Ok(rx) => {
            println!("Subscribed to peer events");
            for event in rx.iter() {
                match event {
                    Ok(update) => println!("Received peer event: {:?}", update),
                    Err(e) => eprintln!("Peer event error: {}", e),
                }
            }
        }
        Err(e) => eprintln!("Failed to subscribe to peer events: {}", e),
    });

    println!("reaching here");

    loop {
        match client.connect_peer(lnrpc::ConnectPeerRequest {
            addr: Some(lnrpc::LightningAddress {
                pubkey: "02546bfe3778d7f8aea43224337d082bcc4521150569c94c9052413ae5b6599c2d"
                    .to_string(),
                host: "localhost:9735".to_string(),
                ..Default::default()
            }),
            perm: true,
            ..Default::default()
        }) {
            Ok(response) => println!("Peer connected: {:?}", response),
            Err(e) => eprintln!("ConnectPeer error: {}", e),
        }

        // Sleep for 3 seconds before the next iteration
        std::thread::sleep(std::time::Duration::from_secs(3));
    }

    // Keep the main thread alive
    // loop {
    //     std::thread::sleep(std::time::Duration::from_secs(1));
    // }
}
