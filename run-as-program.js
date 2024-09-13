const addon = require('./build/Release/addon');

console.log("Loaded addon:", addon);
console.log(addon.start.name)

const startArgs = `--lnddir=./lnd
--noseedbackup
--nolisten
--bitcoin.active
--bitcoin.regtest
--bitcoin.node=neutrino
--feeurl="https://nodes.lightning.computer/fees/v1/btc-fee-estimates.json"
--routing.assumechanvalid
--tlsdisableautofill
--db.bolt.auto-compact
--db.bolt.auto-compact-min-age=0
--neutrino.connect=192.168.10.120:19444`;

console.log("Calling start with args:", startArgs);

const startResult = addon.start(startArgs);
console.log("Start function returned:", startResult);

if (startResult && typeof startResult.then === 'function') {
  startResult
    .then((result) => {
      console.log("LND started successfully:", result);

      setTimeout(async () => {
        console.log("getInfo");
        const getInfoResult = await addon.getInfo("");
        console.log("getInfo result:", getInfoResult);
      }, 3000);

      setTimeout(() => {
        console.log("Subscribing to state changes");
        const unsubscribe = addon.subscribeState(
          "",
          (d) => {
            console.log("!!!!!!!Received state update",d,d.length)
            unsubscribe();

          },
          (error) => {
            console.error("!!!!!!Error subscribing to state:", error)

          }
        );
        console.log("Subscribe function returned:", unsubscribe);


        // setTimeout(() => {
        //   unsubscribe();
        // }, 2000);

      }, 1000);
    })
    .catch((error) => {
      console.error("Error starting LND:", error);
    });
} else {
  console.error("start function did not return a Promise");
}


let isShuttingDown = false;

function gracefulShutdown() {
  if (isShuttingDown) return;
  isShuttingDown = true;

  process.exit(0);
}

process.on('SIGINT', gracefulShutdown);
process.on('SIGTERM', gracefulShutdown);

console.log("Process running. Press Ctrl+C to exit.");

// Keep the process alive
setInterval(() => {
  if (!isShuttingDown) {
      console.log("Still alive...");
  }
}, 5000);
