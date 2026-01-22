import { mount } from 'svelte'
import './app.css'
import App from './App.svelte'
import { setupViewportHandler } from './utils/viewport'

// Setup viewport handler for mobile virtual keyboard support
setupViewportHandler()

const app = mount(App, {
  target: document.getElementById('app')!,
})

export default app
