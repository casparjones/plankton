const path = require('path');
const MiniCssExtractPlugin = require('mini-css-extract-plugin');
const HtmlWebpackPlugin = require('html-webpack-plugin');
const { VueLoaderPlugin } = require('vue-loader');

module.exports = {
  entry: './src/frontend/main.ts',
  output: {
    filename: 'bundle.[contenthash:8].js',
    path: path.resolve(__dirname, 'static'),
    publicPath: '/',
    clean: {
      keep: (filename) => {
        // Keep everything except old bundle.* and index.html (which we regenerate)
        if (/^bundle\./.test(filename)) return false;
        if (filename === 'index.html') return false;
        return true;
      },
    },
  },
  resolve: {
    extensions: ['.ts', '.js', '.vue', '.json'],
    alias: {
      '@': path.resolve(__dirname, 'src/frontend'),
    },
  },
  plugins: [
    new MiniCssExtractPlugin({ filename: 'bundle.[contenthash:8].css' }),
    new HtmlWebpackPlugin({
      template: './src/frontend/index.html',
      inject: true,
    }),
    new VueLoaderPlugin(),
  ],
  module: {
    rules: [
      {
        test: /\.vue$/,
        loader: 'vue-loader',
      },
      {
        test: /\.ts$/,
        loader: 'ts-loader',
        exclude: /node_modules/,
        options: {
          appendTsSuffixTo: [/\.vue$/],
          transpileOnly: true,
        },
      },
      {
        test: /\.css$/i,
        use: [
          MiniCssExtractPlugin.loader,
          'css-loader',
          {
            loader: 'postcss-loader',
            options: {
              postcssOptions: {
                plugins: [
                  '@tailwindcss/postcss',
                ],
              },
            },
          },
        ],
      },
    ],
  },
};
