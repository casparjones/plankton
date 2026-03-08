const path = require('path');
const MiniCssExtractPlugin = require('mini-css-extract-plugin');

module.exports = {
  entry: './src/frontend/main.js',
  output: {
    filename: 'bundle.js',
    path: path.resolve(__dirname, 'static'),
    clean: false,
  },
  plugins: [
    new MiniCssExtractPlugin({ filename: 'bundle.css' }),
  ],
  module: {
    rules: [
      {
        test: /\.css$/i,
        use: [MiniCssExtractPlugin.loader, 'css-loader'],
      },
      {
        // jKanban Fix: Die IIFE `(function() { this.jKanban = ... })()`
        // hat in Webpack strict-mode `this === undefined`.
        // Ersetze den self-invoking Call `})()` durch `}).call(window)`,
        // damit `this` auf `window` zeigt und `window.jKanban` gesetzt wird.
        test: /jkanban[/\\]jkanban\.js$/,
        use: [
          {
            loader: 'exports-loader',
            options: { type: 'commonjs', exports: 'single window.jKanban' },
          },
          {
            loader: 'string-replace-loader',
            options: {
              search: '})()',
              replace: '}).call(window)',
            },
          },
        ],
      },
    ],
  },
};
