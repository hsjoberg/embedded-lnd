use lnd_rust_wrapper::LndLibrary;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let lnd_lib = Arc::new(LndLibrary::new("./liblnd.so")?);
    println!("LndLibrary created successfully");

    let start_args = "--lnddir=./lnd --noseedbackup --nolisten --bitcoin.active --bitcoin.regtest --bitcoin.node=neutrino --feeurl=\"https://nodes.lightning.computer/fees/v1/btc-fee-estimates.json\" --routing.assumechanvalid --tlsdisableautofill --db.bolt.auto-compact --db.bolt.auto-compact-min-age=0 --neutrino.connect=192.168.10.120:19444";

    let lnd_lib_clone = Arc::clone(&lnd_lib);
    tokio::spawn(async move {
        match lnd_lib_clone.start(start_args).await {
            Ok(result) => println!("LND started successfully: {}", result),
            Err(error) => eprintln!("Error starting LND: {}", error),
        }
    });

    // Wait for LND to start
    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

    // Call GetInfo
    match lnd_lib.get_info().await {
        Ok(result) => println!("GetInfo result: {}", result),
        Err(e) => println!("GetInfo error: {}", e),
    }

    // Keep the process alive
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        println!("Still alive...");
    }
}
