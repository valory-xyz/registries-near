require('util').inspect.defaultOptions.depth = 5; // Increase AVA's printing depth

module.exports = {
  timeout: '300000',
  files: ['test/*.ts'],
  failWithoutAssertions: false,
  extensions: [
    'ts',
    'js',
  ],
  require: [
    'ts-node/register',
  ],
};