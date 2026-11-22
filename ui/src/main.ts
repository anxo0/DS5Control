import { createApp, watchEffect } from 'vue'
import '@fontsource-variable/figtree/wght.css'
import './estilos/base.css'
import App from './App.vue'
import { idioma } from './i18n'

// El idioma de la página, para el lector de pantalla y la separación de palabras.
watchEffect(() => {
  document.documentElement.lang = idioma.value
})

createApp(App).mount('#app')
