import { createApp } from 'vue';

import App from './App.vue';
import { router } from './router';
import './styles/main.scss';

// Self-hosted (not Google Fonts CDN) so brand typography renders
// consistently regardless of third-party font blocking — see _tokens.scss
// --spontini-font-serif / --spontini-font-sans.
import '@fontsource/merriweather/400.css';
import '@fontsource/merriweather/700.css';
import '@fontsource/open-sans/400.css';
import '@fontsource/open-sans/600.css';
// bootstrap-italia declares --bs-font-sans: 'Titillium Web' (STACK.md §4.1)
// but its own scss source never bundles an @font-face for it — every DSI
// component (buttons, nav, badges, ...) silently falls back to the system
// font unless something on the page loads Titillium Web itself.
import '@fontsource/titillium-web/400.css';
import '@fontsource/titillium-web/600.css';
import '@fontsource/titillium-web/700.css';

createApp(App).use(router).mount('#app');
