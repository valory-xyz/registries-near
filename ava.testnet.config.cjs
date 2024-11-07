module.exports = {
  ...require('./ava.testnet.config.cjs'),
  ...require('./ava.config.cjs'),
};
module.exports.environmentVariables = {
     TESTNET_MASTER_ACCOUNT_ID: 'olas_000.testnet',
};