const os = require('os');
const platform = os.platform();
const arch = os.arch();

if (platform === 'win32' && arch === 'x64') {
  module.exports = require('./actualised_sdk.win32-x64-msvc.node');
} else if (platform === 'linux' && arch === 'x64') {
  module.exports = require('./actualised_sdk.linux-x64-gnu.node');
} else {
  throw new Error(`Unsupported platform: ${platform} ${arch}`);
}