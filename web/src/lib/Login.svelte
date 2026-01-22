<script lang="ts">
  import { auth, app, wsStatus } from '../stores';
  import { theme } from '../stores/theme';
  import { wsService } from '../services/websocket';

  let token = '';
  let isLoading = false;
  let errorMessage = '';

  // 获取服务器地址配置
  let serverUrl = localStorage.getItem('server_url') || '';
  let showServerConfig = false;

  async function handleLogin() {
    if (!token.trim()) {
      errorMessage = '请输入访问令牌';
      return;
    }

    isLoading = true;
    errorMessage = '';

    try {
      // 保存令牌到 localStorage
      localStorage.setItem('auth_token', token);

      // 如果有自定义服务器地址，使用它
      const wsUrl = serverUrl.trim() ? serverUrl : undefined;

      await wsService.connect(token, wsUrl);
    } catch (error) {
      console.error('Login failed:', error);
      errorMessage = '连接服务器失败，请检查网络或服务器地址';
      localStorage.removeItem('auth_token');
    } finally {
      isLoading = false;
    }
  }

  function toggleServerConfig() {
    showServerConfig = !showServerConfig;
  }

  function saveServerUrl() {
    if (serverUrl.trim()) {
      localStorage.setItem('server_url', serverUrl);
    } else {
      localStorage.removeItem('server_url');
    }
    showServerConfig = false;
  }

  // 监听错误状态
  $: if ($app.error) {
    errorMessage = $app.error;
    isLoading = false;
  }
</script>

<div class="min-h-screen flex items-center justify-center bg-theme-primary relative">
  <!-- Theme Toggle Button - Top Right -->
  <button
    on:click={() => theme.toggle()}
    class="absolute top-4 right-4 p-2 text-theme-secondary hover:text-theme-primary hover:bg-theme-tertiary rounded-lg transition-colors"
    title={$theme === 'dark' ? '切换到亮色主题' : '切换到暗色主题'}
  >
    {#if $theme === 'dark'}
      <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
              d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z" />
      </svg>
    {:else}
      <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
              d="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z" />
      </svg>
    {/if}
  </button>

  <div class="max-w-md w-full mx-4">
    <div class="bg-theme-secondary rounded-lg shadow-xl p-8 border border-theme">
      <!-- Logo and Title -->
      <div class="text-center mb-8">
        <div class="inline-flex items-center justify-center w-16 h-16 bg-blue-600 rounded-full mb-4">
          <svg class="w-8 h-8 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                  d="M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z" />
          </svg>
        </div>
        <h1 class="text-2xl font-bold text-theme-primary">Claude Web Tunnel</h1>
        <p class="text-theme-secondary mt-2">远程 Claude Code 访问控制台</p>
      </div>

      <!-- Login Form -->
      <form on:submit|preventDefault={handleLogin} class="space-y-6">
        <div>
          <label for="token" class="block text-sm font-medium text-gray-300 mb-2">
            访问令牌
          </label>
          <input
            id="token"
            type="password"
            bind:value={token}
            placeholder="输入您的访问令牌"
            class="w-full px-4 py-3 bg-gray-700 border border-gray-600 rounded-lg text-white
                   placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-blue-500
                   focus:border-transparent transition-colors"
            disabled={isLoading}
          />
        </div>

        {#if errorMessage}
          <div class="p-3 bg-red-900/50 border border-red-700 rounded-lg">
            <p class="text-red-400 text-sm">{errorMessage}</p>
          </div>
        {/if}

        <button
          type="submit"
          class="w-full py-3 px-4 bg-blue-600 hover:bg-blue-700 disabled:bg-blue-800
                 disabled:cursor-not-allowed text-white font-medium rounded-lg
                 transition-colors flex items-center justify-center"
          disabled={isLoading}
        >
          {#if isLoading}
            <svg class="animate-spin -ml-1 mr-3 h-5 w-5 text-white" fill="none" viewBox="0 0 24 24">
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
              <path class="opacity-75" fill="currentColor"
                    d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z">
              </path>
            </svg>
            连接中...
          {:else}
            登录
          {/if}
        </button>
      </form>

      <!-- Server Configuration -->
      <div class="mt-6 pt-6 border-t border-gray-700">
        <button
          type="button"
          on:click={toggleServerConfig}
          class="text-sm text-gray-400 hover:text-gray-300 transition-colors flex items-center"
        >
          <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                  d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                  d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
          </svg>
          服务器配置
        </button>

        {#if showServerConfig}
          <div class="mt-4 space-y-3">
            <div>
              <label for="serverUrl" class="block text-sm font-medium text-gray-400 mb-1">
                WebSocket 服务器地址
              </label>
              <input
                id="serverUrl"
                type="text"
                bind:value={serverUrl}
                placeholder="wss://your-server.com/ws/user"
                class="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded-lg text-white
                       text-sm placeholder-gray-500 focus:outline-none focus:ring-2
                       focus:ring-blue-500 focus:border-transparent"
              />
              <p class="mt-1 text-xs text-gray-500">
                留空将使用当前页面的服务器地址
              </p>
            </div>
            <button
              type="button"
              on:click={saveServerUrl}
              class="px-4 py-2 bg-gray-700 hover:bg-gray-600 text-white text-sm
                     rounded-lg transition-colors"
            >
              保存配置
            </button>
          </div>
        {/if}
      </div>

      <!-- Connection Status -->
      <div class="mt-4 flex items-center justify-center">
        <div class="flex items-center text-sm">
          <span class="w-2 h-2 rounded-full mr-2 {
            $wsStatus.status === 'connected' ? 'bg-green-500' :
            $wsStatus.status === 'connecting' ? 'bg-yellow-500 animate-pulse' :
            $wsStatus.status === 'reconnecting' ? 'bg-yellow-500 animate-pulse' :
            $wsStatus.status === 'error' ? 'bg-red-500' :
            'bg-gray-500'
          }"></span>
          <span class="text-gray-400">
            {#if $wsStatus.status === 'connected'}
              已连接
            {:else if $wsStatus.status === 'connecting'}
              连接中...
            {:else if $wsStatus.status === 'reconnecting'}
              重连中 ({$wsStatus.reconnectAttempt}/{$wsStatus.maxReconnectAttempts})
            {:else if $wsStatus.status === 'error'}
              连接错误
            {:else}
              未连接
            {/if}
          </span>
        </div>
      </div>
    </div>

    <!-- Footer -->
    <p class="mt-6 text-center text-gray-500 text-sm">
      Claude Web Tunnel &copy; 2025
    </p>
  </div>
</div>
