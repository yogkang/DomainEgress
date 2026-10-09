import { createApp } from 'vue';
import App from './App.vue';
import './styles/base.css';
import './styles/network-tools.css';
import './styles/themes.css';
import { initAppearance } from './theme';
initAppearance();
createApp(App).mount('#app');
import './styles/workspace.css';
