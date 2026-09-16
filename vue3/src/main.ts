import { createApp } from 'vue'
import App from './App.vue'
import './style.css'
import './themes.css'
import { initAppearance } from './theme'
initAppearance()
createApp(App).mount('#app')
