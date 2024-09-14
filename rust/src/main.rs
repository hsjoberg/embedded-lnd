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
        --neutrino.connect=localhost:19444";

    match client.start(start_args) {
        Ok(_) => println!("LND started successfully"),
        Err(e) => eprintln!("Error starting LND: {}", e),
    }

    println!("...........................................");
    std::thread::sleep(std::time::Duration::from_secs(5));

    match client.get_info() {
        Ok(info) => println!("LND Info: {}", info),
        Err(e) => eprintln!("Error getting LND info: {}", e),
    }

    // // Call walletBalance
    // match client.wallet_balance() {
    //     Ok(balance) => println!("Wallet Balance: {}", balance),
    //     Err(e) => eprintln!("Error getting wallet balance: {}", e),
    // }

    // // Call listChannels
    // match client.list_channels() {
    //     Ok(channels) => println!("Channels: {}", channels),
    //     Err(e) => eprintln!("Error listing channels: {}", e),
    // }

    // Keep the main thread alive
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
