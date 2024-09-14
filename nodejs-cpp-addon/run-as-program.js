const addon = require('./build/Release/addon');
const readline = require('readline');

console.log("Loaded addon:", addon);
console.log(addon.start.name);

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

addon.start(startArgs)
  .then((result) => {
    console.log("LND started successfully:", result);

    // Set up readline interface
    readline.emitKeypressEvents(process.stdin);
    process.stdin.setRawMode(true);

    process.stdin.on('keypress', (str, key) => {
      if (key.ctrl && key.name === 'c') {
        console.log('Ctrl+C pressed. Quitting...');
        gracefulShutdown();
      } else {
        switch (key.name) {
          case 'q':
            console.log('Quitting...');
            gracefulShutdown();
            break;
          case 'g':
            console.log("Requesting getInfo...");
            addon.getInfo("")
              .then((getInfoResult) => {
                console.log("getInfo result:", getInfoResult);
              })
              .catch((error) => {
                console.error("Error getting info:", error);
              });
            break;
          default:
            break;
        }
      }
    });

    console.log("Subscribing to state changes");
    const unsubscribe = addon.subscribeState(
      "",
      (d) => {
        console.log("!!!!!!!Received state update", d);
      },
      (error) => {
        console.error("!!!!!!Error subscribing to state:", error);
      }
    );
    console.log("Subscribe function returned:", unsubscribe);
  })
  .catch((error) => {
    console.error("Error starting LND:", error);
  });

let isShuttingDown = false;

function gracefulShutdown() {
  if (isShuttingDown) return;
  isShuttingDown = true;

  // Restore the default behavior of stdin before exiting
  process.stdin.setRawMode(false);
  process.stdin.pause();

  process.exit(0);
}

console.log("Process running. Press 'q' to quit, 'g' for getInfo, or use Ctrl+C.");

// Keep the process alive without using setInterval
process.stdin.resume();
