<script lang="ts">
  import { wsStatus } from '../stores';

  $: isReconnecting = $wsStatus.status === 'reconnecting';
  $: attempt = $wsStatus.reconnectAttempt;
  $: maxAttempts = $wsStatus.maxReconnectAttempts;
</script>

{#if isReconnecting}
  <div class="fixed bottom-4 right-4 z-50 animate-slide-up">
    <div class="bg-yellow-900/90 border border-yellow-700 rounded-lg shadow-lg p-4 flex items-center space-x-3">
      <div class="flex-shrink-0">
        <svg class="animate-spin w-5 h-5 text-yellow-400" fill="none" viewBox="0 0 24 24">
          <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
          <path class="opacity-75" fill="currentColor"
                d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z">
          </path>
        </svg>
      </div>
      <div>
        <p class="text-yellow-200 font-medium">
          Reconnecting...
        </p>
        <p class="text-yellow-400/80 text-sm">
          {attempt} / {maxAttempts}
        </p>
      </div>
    </div>
  </div>
{/if}

<style>
  @keyframes slide-up {
    from {
      opacity: 0;
      transform: translateY(20px);
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
