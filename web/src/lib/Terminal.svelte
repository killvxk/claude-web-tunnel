<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { Terminal } from '@xterm/xterm';
  import { FitAddon } from '@xterm/addon-fit';
  import { WebLinksAddon } from '@xterm/addon-web-links';
  import '@xterm/xterm/css/xterm.css';
  import { app, auth, connectedInstance } from '../stores';
  import { theme, terminalThemes } from '../stores/theme';
  import { wsService } from '../services/websocket';

  let terminalContainer: HTMLDivElement;
  let terminal: Terminal | null = null;
  let fitAddon: FitAddon | null = null;
  let resizeObserver: ResizeObserver | null = null;

  // Mobile menu state
  let showMobileMenu = false;

  // 获取工作目录显示名称
  function getDisplayName(cwd: string): string {
    const parts = cwd.replace(/\\/g, '/').split('/').filter(Boolean);
    return parts[parts.length - 1] || cwd;
  }

  onMount(() => {
    initTerminal();

    // 监听窗口大小变化
    resizeObserver = new ResizeObserver(() => {
      if (fitAddon && terminal) {
        fitAddon.fit();
        // 发送新的终端大小
        wsService.sendTerminalResize(terminal.cols, terminal.rows);
      }
    });

    if (terminalContainer) {
      resizeObserver.observe(terminalContainer);
    }

    return () => {
      cleanup();
    };
  });

  onDestroy(() => {
    cleanup();
  });

  function initTerminal() {
    terminal = new Terminal({
      cursorBlink: true,
      cursorStyle: 'block',
      fontSize: 14,
      fontFamily: 'Consolas, "Courier New", monospace',
      theme: terminalThemes[$theme],
      allowProposedApi: true,
    });

    // 添加插件
    fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);
    terminal.loadAddon(new WebLinksAddon());

    // 挂载到 DOM
    terminal.open(terminalContainer);
    fitAddon.fit();

    // 发送初始终端大小
    wsService.sendTerminalResize(terminal.cols, terminal.rows);

    // 处理用户输入
    terminal.onData((data) => {
      wsService.sendTerminalInput(data);
    });

    // 注册终端输出处理器
    wsService.setTerminalOutputHandler((data) => {
      if (terminal) {
        terminal.write(data);
      }
    });

    // 显示欢迎消息
    terminal.writeln('\x1b[1;34m═══════════════════════════════════════════════════════════════\x1b[0m');
    terminal.writeln('\x1b[1;37m                   Claude Web Tunnel                            \x1b[0m');
    terminal.writeln('\x1b[1;34m═══════════════════════════════════════════════════════════════\x1b[0m');
    terminal.writeln('');
    if ($connectedInstance) {
      terminal.writeln(`\x1b[32m已连接到: ${getDisplayName($connectedInstance.cwd)}\x1b[0m`);
    }
    terminal.writeln('\x1b[33m等待 Claude Code 输出...\x1b[0m');
    terminal.writeln('');
  }

  function cleanup() {
    wsService.setTerminalOutputHandler(null);

    if (resizeObserver) {
      resizeObserver.disconnect();
      resizeObserver = null;
    }

    if (terminal) {
      terminal.dispose();
      terminal = null;
    }

    fitAddon = null;
  }

  function handleDisconnect() {
    wsService.detachFromInstance();
    cleanup();
  }

  function handleLogout() {
    wsService.disconnect();
    auth.logout();
    app.reset();
    cleanup();
  }

  // 处理复制
  function handleCopy() {
    if (terminal) {
      const selection = terminal.getSelection();
      if (selection) {
        navigator.clipboard.writeText(selection);
      }
    }
  }

  // 处理粘贴
  async function handlePaste() {
    try {
      const text = await navigator.clipboard.readText();
      if (text && terminal) {
        wsService.sendTerminalInput(text);
      }
    } catch (error) {
      console.error('Paste failed:', error);
    }
  }

  // 清屏
  function handleClear() {
    if (terminal) {
      terminal.clear();
    }
  }

  // 切换主题
  function handleToggleTheme() {
    theme.toggle();
  }

  // 响应主题变化 - 更新终端主题
  $: if (terminal && $theme) {
    terminal.options.theme = terminalThemes[$theme];
  }
</script>

<div class="flex flex-col h-viewport bg-theme-primary">
  <!-- Header -->
  <header class="flex-shrink-0 bg-theme-secondary border-b border-theme px-2 sm:px-4 py-2">
    <div class="flex items-center justify-between">
      <div class="flex items-center space-x-2 sm:space-x-4 min-w-0">
        <h1 class="text-base sm:text-lg font-semibold text-theme-primary truncate">Claude Web Tunnel</h1>
        {#if $connectedInstance}
          <div class="hidden sm:flex items-center text-sm text-theme-secondary">
            <span class="w-2 h-2 bg-green-500 rounded-full mr-2"></span>
            <span class="truncate max-w-[150px]">{getDisplayName($connectedInstance.cwd)}</span>
          </div>
        {/if}
      </div>

      <!-- Desktop Toolbar -->
      <div class="hidden md:flex items-center space-x-2">
        <button
          on:click={handleCopy}
          class="p-2 text-theme-secondary hover:text-theme-primary hover:bg-theme-tertiary rounded transition-colors"
          title="复制选中内容"
        >
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                  d="M8 5H6a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2v-1M8 5a2 2 0 002 2h2a2 2 0 002-2M8 5a2 2 0 012-2h2a2 2 0 012 2m0 0h2a2 2 0 012 2v3m2 4H10m0 0l3-3m-3 3l3 3" />
          </svg>
        </button>

        <button
          on:click={handlePaste}
          class="p-2 text-theme-secondary hover:text-theme-primary hover:bg-theme-tertiary rounded transition-colors"
          title="粘贴"
        >
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                  d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2" />
          </svg>
        </button>

        <button
          on:click={handleClear}
          class="p-2 text-theme-secondary hover:text-theme-primary hover:bg-theme-tertiary rounded transition-colors"
          title="清屏"
        >
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                  d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
          </svg>
        </button>

        <button
          on:click={handleToggleTheme}
          class="p-2 text-theme-secondary hover:text-theme-primary hover:bg-theme-tertiary rounded transition-colors"
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

        <div class="w-px h-6 bg-theme-tertiary mx-2"></div>

        <button
          on:click={handleDisconnect}
          class="px-3 py-1.5 text-sm bg-theme-tertiary hover:opacity-80 text-theme-primary rounded transition-colors"
        >
          断开连接
        </button>

        <button
          on:click={handleLogout}
          class="px-3 py-1.5 text-sm bg-red-600 hover:bg-red-700 text-white rounded transition-colors"
        >
          退出登录
        </button>
      </div>

      <!-- Mobile Menu Button -->
      <div class="md:hidden flex items-center space-x-2">
        <button
          on:click={handleToggleTheme}
          class="p-2 text-theme-secondary hover:text-theme-primary hover:bg-theme-tertiary rounded transition-colors touch-target"
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

        <button
          on:click={() => showMobileMenu = !showMobileMenu}
          class="p-2 text-theme-secondary hover:text-theme-primary hover:bg-theme-tertiary rounded transition-colors touch-target"
        >
          <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            {#if showMobileMenu}
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
            {:else}
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16M4 18h16" />
            {/if}
          </svg>
        </button>
      </div>
    </div>

    <!-- Mobile Dropdown Menu -->
    {#if showMobileMenu}
      <div class="md:hidden mt-2 pt-2 border-t border-theme">
        {#if $connectedInstance}
          <div class="flex items-center text-sm text-theme-secondary mb-2 px-2">
            <span class="w-2 h-2 bg-green-500 rounded-full mr-2"></span>
            <span class="truncate">{getDisplayName($connectedInstance.cwd)}</span>
          </div>
        {/if}

        <div class="grid grid-cols-3 gap-2 mb-2">
          <button
            on:click={() => { handleCopy(); showMobileMenu = false; }}
            class="flex flex-col items-center p-3 text-theme-secondary hover:text-theme-primary hover:bg-theme-tertiary rounded transition-colors"
          >
            <svg class="w-5 h-5 mb-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                    d="M8 5H6a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2v-1M8 5a2 2 0 002 2h2a2 2 0 002-2M8 5a2 2 0 012-2h2a2 2 0 012 2m0 0h2a2 2 0 012 2v3m2 4H10m0 0l3-3m-3 3l3 3" />
            </svg>
            <span class="text-xs">复制</span>
          </button>

          <button
            on:click={() => { handlePaste(); showMobileMenu = false; }}
            class="flex flex-col items-center p-3 text-theme-secondary hover:text-theme-primary hover:bg-theme-tertiary rounded transition-colors"
          >
            <svg class="w-5 h-5 mb-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                    d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2" />
            </svg>
            <span class="text-xs">粘贴</span>
          </button>

          <button
            on:click={() => { handleClear(); showMobileMenu = false; }}
            class="flex flex-col items-center p-3 text-theme-secondary hover:text-theme-primary hover:bg-theme-tertiary rounded transition-colors"
          >
            <svg class="w-5 h-5 mb-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                    d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
            </svg>
            <span class="text-xs">清屏</span>
          </button>
        </div>

        <div class="flex space-x-2">
          <button
            on:click={() => { handleDisconnect(); showMobileMenu = false; }}
            class="flex-1 px-3 py-2 text-sm bg-theme-tertiary hover:opacity-80 text-theme-primary rounded transition-colors"
          >
            断开连接
          </button>

          <button
            on:click={() => { handleLogout(); showMobileMenu = false; }}
            class="flex-1 px-3 py-2 text-sm bg-red-600 hover:bg-red-700 text-white rounded transition-colors"
          >
            退出登录
          </button>
        </div>
      </div>
    {/if}
  </header>

  <!-- Terminal Container -->
  <div class="flex-1 overflow-hidden p-1 sm:p-2" bind:this={terminalContainer}></div>

  <!-- Status Bar - Hidden on mobile -->
  <footer class="hidden sm:block flex-shrink-0 bg-theme-secondary border-t border-theme px-4 py-1">
    <div class="flex items-center justify-between text-xs text-theme-secondary">
      <div class="flex items-center space-x-4">
        <span>用户: {$auth.agentName || '-'}</span>
        <span>角色: {$auth.role || '-'}</span>
      </div>
      <div class="flex items-center space-x-4">
        {#if terminal}
          <span>终端: {terminal.cols}x{terminal.rows}</span>
        {/if}
        <span>Ctrl+Shift+C 复制 | Ctrl+Shift+V 粘贴</span>
      </div>
    </div>
  </footer>
</div>

<style>
  :global(.xterm) {
    padding: 8px;
  }
</style>
