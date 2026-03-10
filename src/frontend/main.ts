// Plankton Frontend – Entry Point (Vue.js 3 + TypeScript)
// CSS-Import für Webpack-Bundling.
import '../../static/styles.css'

import { createApp } from 'vue'
import App from './App.vue'

const app = createApp(App)
app.mount('#app')
