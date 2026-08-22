import { createApp } from 'vue'
import { createPinia } from 'pinia'
import 'bootstrap/dist/css/bootstrap.min.css'
import 'bootstrap-icons/font/bootstrap-icons.css'
import './style.css'
import App from './App.vue'
import router from './router'
import { useAuthStore } from './stores/auth'
import { can } from './directives/can'
import { i18n } from './i18n'
import { useToast } from './composables/useToast'
import { loadBaseCurrencySymbol } from './lib/currency'

const app = createApp(App)
const pinia = createPinia()
app.use(pinia)
app.use(router)
app.use(i18n)
app.directive('can', can)
useAuthStore(pinia).hydrate()
void loadBaseCurrencySymbol()

// F9.3 global error boundary: surface unhandled errors as toasts.
const toast = useToast()
app.config.errorHandler = (err, _instance, info) => {
  console.error('[global]', err, info)
  toast.error(err instanceof Error ? err.message : String(err))
}
window.addEventListener('unhandledrejection', (event) => {
  const reason: unknown = event.reason
  console.error('[unhandled rejection]', reason)
  toast.error(reason instanceof Error ? reason.message : String(reason))
})

app.mount('#app')

// F9.1 splash screen: remove once the real app is mounted.
document.getElementById('splash')?.remove()
