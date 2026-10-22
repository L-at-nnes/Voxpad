module.exports = {
  transpileDependencies: ['vuetify'],
  publicPath: '',
  productionSourceMap: false,

  // Terser's dead-code elimination mis-compiles @tauri-apps/api's TypeScript-emitted private
  // class fields (a `class Channel` whose private-field WeakMaps are initialized via a
  // computed-method-key side effect): it strips the class as "unused" but leaves the WeakMap
  // assignment statements behind without their `var` declaration, which throws
  // "ReferenceError: _Channel_onmessage is not defined" under "use strict". This is a portable
  // desktop app (not served over a network), so trading a larger bundle for correctness by
  // skipping minification is a fine tradeoff.
  configureWebpack: {
    optimization: {
      minimize: false,
    },
  },

  pluginOptions: {
    i18n: {
      locale: 'en',
      fallbackLocale: 'en',
      localeDir: 'locales',
    },
  },
};
