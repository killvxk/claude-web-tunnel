<script lang="ts">
  import { onMount } from 'svelte';
  import { theme } from '../stores/theme';

  interface BeforeInstallPromptEvent extends Event {
    prompt(): Promise<void>;
    userChoice: Promise<{ outcome: 'accepted' | 'dismissed' }>;
  }

  let deferredPrompt: BeforeInstallPromptEvent | null = null;
  let showPrompt = $state(false);
  let dismissed = $state(false);

  onMount(() => {
    // 检查是否已经安装
    if (window.matchMedia('(display-mode: standalone)').matches) {
      return;
    }

    // 检查是否已经dismissed
    const dismissedAt = localStorage.getItem('pwa_dismissed_at');
    if (dismissedAt) {
      const dismissedTime = parseInt(dismissedAt, 10);
      const oneWeek = 7 * 24 * 60 * 60 * 1000;
      if (Date.now() - dismissedTime < oneWeek) {
        dismissed = true;
        return;
      }
    }

    window.addEventListener('beforeinstallprompt', (e) => {
      e.preventDefault();
      deferredPrompt = e as BeforeInstallPromptEvent;
      if (!dismissed) {
        showPrompt = true;
      }
    });

    window.addEventListener('appinstalled', () => {
      showPrompt = false;
      deferredPrompt = null;
    });
  });

  async function install() {
    if (!deferredPrompt) return;

    await deferredPrompt.prompt();
    const choice = await deferredPrompt.userChoice;

    if (choice.outcome === 'accepted') {
      showPrompt = false;
    }

    deferredPrompt = null;
  }

  function dismiss() {
    showPrompt = false;
    dismissed = true;
    localStorage.setItem('pwa_dismissed_at', Date.now().toString());
  }
</script>

{#if showPrompt}
  <div class="fixed bottom-4 left-4 right-4 sm:left-auto sm:right-4 sm:w-80 z-50 animate-slide-up">
    <div class="bg-theme-secondary border border-theme rounded-lg shadow-xl p-4">
      <div class="flex items-start gap-3">
        <div class="flex-shrink-0 w-10 h-10 bg-blue-600 rounded-lg flex items-center justify-center">
          <svg class="w-6 h-6 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                  d="M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z" />
          </svg>
        </div>
        <div class="flex-1 min-w-0">
          <h3 class="text-theme-primary font-medium text-sm">安装 Claude Web Tunnel</h3>
          <p class="text-theme-secondary text-xs mt-1">
            安装到桌面，获得更好的体验
          </p>
        </div>
        <button
          onclick={dismiss}
          class="flex-shrink-0 text-theme-secondary hover:text-theme-primary transition-colors"
          aria-label="关闭"
        >
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>

      <div class="flex gap-2 mt-4">
        <button
          onclick={dismiss}
          class="flex-1 px-3 py-2 text-sm text-theme-secondary hover:text-theme-primary
                 border border-theme rounded-lg hover:bg-theme-hover transition-colors"
        >
          稍后
        </button>
        <button
          onclick={install}
          class="flex-1 px-3 py-2 text-sm text-white bg-blue-600 hover:bg-blue-700
                 rounded-lg transition-colors"
        >
          安装
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  @keyframes slide-up {
    from {
      opacity: 0;
      transform: translateY(1rem);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .animate-slide-up {
    animation: slide-up 0.3s ease-out;
  }
</style>
