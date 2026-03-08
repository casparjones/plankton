// Plankton Frontend – Entry Point
// CSS-Import für Webpack-Bundling.
import '../../static/styles.css';

// Anwendungslogik importieren und starten.
import { init } from './app.js';

document.addEventListener('DOMContentLoaded', init);
