use lnd_rust_wrapper::LndClient;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = LndClient::new();

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
        --neutrino.connect=192.168.10.120:19444";

    match client.start(start_args) {
        Ok(_) => println!("LND started successfully"),
        Err(e) => eprintln!("Error starting LND: {}", e),
    }

    // Keep the main thread alive
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
