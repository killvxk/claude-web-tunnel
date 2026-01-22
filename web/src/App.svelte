<script lang="ts">
  import { onMount } from 'svelte';
  import { currentView, auth } from './stores';
  import { wsService } from './services/websocket';
  import Login from './lib/Login.svelte';
  import Instances from './lib/Instances.svelte';
  import Terminal from './lib/Terminal.svelte';
  import AdminPanel from './lib/AdminPanel.svelte';
  import ReconnectToast from './lib/ReconnectToast.svelte';
  import InstallPrompt from './lib/InstallPrompt.svelte';

  // 尝试恢复会话
  onMount(async () => {
    const savedToken = auth.restore();
    if (savedToken) {
      try {
        await wsService.connect(savedToken);
      } catch (error) {
        console.error('Failed to restore session:', error);
        auth.logout();
      }
    }
  });
</script>

{#if $currentView === 'login'}
  <Login />
{:else if $currentView === 'instances'}
  <Instances />
{:else if $currentView === 'terminal'}
  <Terminal />
{:else if $currentView === 'admin'}
  <AdminPanel />
{/if}

<!-- Global reconnection toast -->
<ReconnectToast />

<!-- PWA install prompt -->
<InstallPrompt />
