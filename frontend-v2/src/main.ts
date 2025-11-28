import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import router from './router'
import './style.css'

const app = createApp(App)

app.use(createPinia())
app.use(router)

app.config.errorHandler = (err, _instance, info) => {
    console.error('Global Error:', err)
    const div = document.createElement('div')
    div.style.position = 'fixed'
    div.style.top = '0'
    div.style.left = '0'
    div.style.width = '100%'
    div.style.height = '100%'
    div.style.backgroundColor = 'rgba(0,0,0,0.9)'
    div.style.color = '#ff5555'
    div.style.zIndex = '9999'
    div.style.padding = '20px'
    div.style.fontFamily = 'monospace'
    div.style.whiteSpace = 'pre-wrap'
    div.style.overflow = 'auto'
    div.innerHTML = `<h1>Runtime Error</h1><p>${String(err)}</p><pre>${info}</pre>`
    document.body.appendChild(div)
}

app.mount('#app')
