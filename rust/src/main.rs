use lnd_rust_wrapper::LndLibrary;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let lnd_lib = LndLibrary::new("./liblnd.so")?;

    let start_args = "--lnddir=./lnd --noseedbackup --nolisten --bitcoin.active --bitcoin.regtest --bitcoin.node=neutrino --feeurl=\"https://nodes.lightning.computer/fees/v1/btc-fee-estimates.json\" --routing.assumechanvalid --tlsdisableautofill --db.bolt.auto-compact --db.bolt.auto-compact-min-age=0 --neutrino.connect=192.168.10.120:19444";

    match lnd_lib.start(start_args).await {
        Ok(result) => println!("LND started successfully: {}", result),
        Err(error) => eprintln!("Error starting LND: {}", error),
    }

    // Keep the process alive
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        println!("Still alive...");
    }
}