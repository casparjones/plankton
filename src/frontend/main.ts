// Plankton Frontend – Entry Point (Vue.js 3 + TypeScript)
// CSS-Import für Webpack-Bundling.
import './styles/globals.css'
import 'vue-toastification/dist/index.css'

import { createApp } from 'vue'
import Toast from 'vue-toastification'
import App from './App.vue'

const app = createApp(App)
app.use(Toast, {
  position: 'bottom-right',
  timeout: 3000,
  closeOnClick: true,
  pauseOnHover: true,
  draggable: true,
  hideProgressBar: false,
  toastClassName: 'plankton-toast',
})
app.mount('#app')
