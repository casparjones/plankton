const path = require('path');
const MiniCssExtractPlugin = require('mini-css-extract-plugin');

module.exports = {
  // Einstiegspunkt: static/main.js (liegt neben index.html)
  entry: './static/main.js',
  output: {
    filename: 'bundle.js',
    path: path.resolve(__dirname, 'static'),
    clean: false, // index.html nicht löschen
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
    ],
  },
};