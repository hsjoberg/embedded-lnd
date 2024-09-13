const addon = require('./build/Release/addon');

console.log("Loaded addon:", addon);
console.log(addon.start.name)

module.exports = addon;
