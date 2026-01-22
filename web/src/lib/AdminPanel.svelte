<script lang="ts">
  import { app, auth, admin } from '../stores';
  import { theme } from '../stores/theme';
  import { wsService } from '../services/websocket';
  import type { AgentInfo } from '../types';
  import { onMount } from 'svelte';
  import AuditLogViewer from './AuditLogViewer.svelte';

  let searchQuery = $state('');
  let confirmAction: { type: 'disconnect' | 'delete'; agent: AgentInfo } | null = $state(null);
  let showAuditLogs = $state(false);

  // 加载管理统计
  onMount(() => {
    wsService.requestAdminStats();
  });

  // 过滤 agents
  let filteredAgents = $derived($admin.agents.filter(agent =>
    agent.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
    agent.id.toLowerCase().includes(searchQuery.toLowerCase())
  ));

  // 刷新数据
  function refreshStats() {
    wsService.requestAdminStats();
  }

  // 返回实例列表
  function goBack() {
    app.setView('instances');
  }

  // 退出登录
  function handleLogout() {
    wsService.disconnect();
    auth.logout();
    admin.reset();
    app.reset();
  }

  // 格式化时间
  function formatTime(dateString: string | null): string {
    if (!dateString) return '-';
    const date = new Date(dateString);
    return date.toLocaleString('zh-CN', {
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
    });
  }

  // 打开确认对话框
  function openConfirmDialog(type: 'disconnect' | 'delete', agent: AgentInfo) {
    confirmAction = { type, agent };
  }

  // 关闭确认对话框
  function closeConfirmDialog() {
    confirmAction = null;
  }

  // 执行确认操作
  function executeAction() {
    if (!confirmAction) return;

    const { type, agent } = confirmAction;
    if (type === 'disconnect') {
      wsService.forceDisconnectAgent(agent.id);
    } else if (type === 'delete') {
      wsService.deleteAgent(agent.id);
    }
    closeConfirmDialog();
  }

  // 强制断开 Agent
  function forceDisconnectAgent(agent: AgentInfo) {
    if (agent.status !== 'online') return;
    openConfirmDialog('disconnect', agent);
  }

  // 删除 Agent
  function deleteAgent(agent: AgentInfo) {
    openConfirmDialog('delete', agent);
  }
</script>

<div class="h-viewport flex flex-col bg-theme-primary">
  <!-- Header -->
  <header class="flex-shrink-0 bg-theme-secondary border-b border-theme px-4 py-3">
    <div class="flex items-center justify-between">
      <div class="flex items-center gap-3">
        <!-- Back button -->
        <button
          onclick={goBack}
          class="p-2 text-theme-secondary hover:text-theme-primary hover:bg-theme-hover rounded-lg transition-colors"
          title="返回实例列表"
        >
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
          </svg>
        </button>
        <div>
          <h1 class="text-lg font-semibold text-theme-primary">Super Admin Panel</h1>
          <p class="text-xs text-theme-secondary">系统管理控制台</p>
        </div>
      </div>

      <div class="flex items-center gap-2">
        <!-- Audit Logs -->
        <button
          onclick={() => showAuditLogs = true}
          class="p-2 text-theme-secondary hover:text-theme-primary hover:bg-theme-hover rounded-lg transition-colors"
          title="Audit Logs"
        >
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
          </svg>
        </button>

        <!-- Refresh -->
        <button
          onclick={refreshStats}
          class="p-2 text-theme-secondary hover:text-theme-primary hover:bg-theme-hover rounded-lg transition-colors"
          disabled={$admin.loading}
          title="刷新"
        >
          <svg class="w-5 h-5 {$admin.loading ? 'animate-spin' : ''}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
          </svg>
        </button>

        <!-- Theme toggle -->
        <button
          onclick={() => theme.toggle()}
          class="p-2 text-theme-secondary hover:text-theme-primary hover:bg-theme-hover rounded-lg transition-colors"
          title={$theme === 'dark' ? '切换到亮色主题' : '切换到暗色主题'}
        >
          {#if $theme === 'dark'}
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z" />
            </svg>
          {:else}
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z" />
            </svg>
          {/if}
        </button>

        <!-- Logout -->
        <button
          onclick={handleLogout}
          class="p-2 text-red-400 hover:text-red-300 hover:bg-red-900/30 rounded-lg transition-colors"
          title="退出登录"
        >
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1" />
          </svg>
        </button>
      </div>
    </div>
  </header>

  <!-- Stats Cards -->
  {#if $admin.stats}
    <div class="flex-shrink-0 px-4 py-4 border-b border-theme">
      <div class="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-5 gap-3">
        <!-- Total Agents -->
        <div class="bg-theme-secondary rounded-lg p-3 border border-theme">
          <div class="text-xs text-theme-secondary mb-1">Agents</div>
          <div class="text-2xl font-bold text-theme-primary">{$admin.stats.total_agents}</div>
          <div class="text-xs text-green-400">{$admin.stats.online_agents} online</div>
        </div>

        <!-- Total Instances -->
        <div class="bg-theme-secondary rounded-lg p-3 border border-theme">
          <div class="text-xs text-theme-secondary mb-1">Instances</div>
          <div class="text-2xl font-bold text-theme-primary">{$admin.stats.total_instances}</div>
          <div class="text-xs text-green-400">{$admin.stats.running_instances} running</div>
        </div>

        <!-- Total Users -->
        <div class="bg-theme-secondary rounded-lg p-3 border border-theme">
          <div class="text-xs text-theme-secondary mb-1">Users</div>
          <div class="text-2xl font-bold text-theme-primary">{$admin.stats.total_users}</div>
          <div class="text-xs text-theme-secondary">connected</div>
        </div>

        <!-- Online Rate -->
        <div class="bg-theme-secondary rounded-lg p-3 border border-theme hidden sm:block">
          <div class="text-xs text-theme-secondary mb-1">Online Rate</div>
          <div class="text-2xl font-bold text-theme-primary">
            {$admin.stats.total_agents > 0
              ? Math.round(($admin.stats.online_agents / $admin.stats.total_agents) * 100)
              : 0}%
          </div>
          <div class="text-xs text-theme-secondary">agents online</div>
        </div>

        <!-- Instance Rate -->
        <div class="bg-theme-secondary rounded-lg p-3 border border-theme hidden lg:block">
          <div class="text-xs text-theme-secondary mb-1">Running Rate</div>
          <div class="text-2xl font-bold text-theme-primary">
            {$admin.stats.total_instances > 0
              ? Math.round(($admin.stats.running_instances / $admin.stats.total_instances) * 100)
              : 0}%
          </div>
          <div class="text-xs text-theme-secondary">instances running</div>
        </div>
      </div>
    </div>
  {/if}

  <!-- Search Bar -->
  <div class="flex-shrink-0 px-4 py-3 border-b border-theme">
    <div class="relative">
      <input
        type="text"
        bind:value={searchQuery}
        placeholder="Search agents..."
        class="w-full pl-10 pr-4 py-2 bg-theme-secondary border border-theme rounded-lg text-sm text-theme-primary placeholder-theme-secondary focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
      />
      <svg class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-theme-secondary" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
      </svg>
    </div>
  </div>

  <!-- Agent List -->
  <div class="flex-1 overflow-y-auto p-4">
    {#if $admin.loading && $admin.agents.length === 0}
      <div class="flex items-center justify-center h-full">
        <div class="text-center">
          <svg class="w-12 h-12 mx-auto mb-4 text-theme-secondary animate-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
          </svg>
          <p class="text-theme-secondary">Loading...</p>
        </div>
      </div>
    {:else if filteredAgents.length === 0}
      <div class="flex items-center justify-center h-full">
        <div class="text-center">
          <svg class="w-16 h-16 mx-auto mb-4 text-theme-secondary opacity-50" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
          </svg>
          <p class="text-theme-secondary text-lg mb-2">
            {searchQuery ? 'No matching agents' : 'No agents registered'}
          </p>
          <p class="text-theme-secondary text-sm">
            {searchQuery ? 'Try a different search term' : 'Agents will appear here when they connect'}
          </p>
        </div>
      </div>
    {:else}
      <div class="space-y-3">
        {#each filteredAgents as agent (agent.id)}
          <div class="bg-theme-secondary rounded-lg border border-theme p-4 hover:border-blue-500/50 transition-colors">
            <div class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3">
              <!-- Agent Info -->
              <div class="flex-1 min-w-0">
                <div class="flex items-center gap-2 mb-1">
                  <!-- Status indicator -->
                  <span class="w-2 h-2 rounded-full {agent.status === 'online' ? 'bg-green-500' : 'bg-gray-500'}"></span>
                  <h3 class="font-medium text-theme-primary truncate">{agent.name}</h3>
                  <span class="text-xs px-2 py-0.5 rounded-full {agent.status === 'online' ? 'bg-green-900/50 text-green-400' : 'bg-gray-700 text-gray-400'}">
                    {agent.status}
                  </span>
                </div>
                <div class="text-xs text-theme-secondary font-mono truncate mb-2">{agent.id}</div>
                <div class="flex flex-wrap gap-4 text-xs text-theme-secondary">
                  <span class="flex items-center gap-1">
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z" />
                    </svg>
                    {agent.instance_count} instances
                  </span>
                  <span class="flex items-center gap-1">
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4.354a4 4 0 110 5.292M15 21H3v-1a6 6 0 0112 0v1zm0 0h6v-1a6 6 0 00-9-5.197M13 7a4 4 0 11-8 0 4 4 0 018 0z" />
                    </svg>
                    {agent.user_count} users
                  </span>
                  <span class="flex items-center gap-1">
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
                    </svg>
                    {formatTime(agent.connected_at)}
                  </span>
                </div>
              </div>

              <!-- Actions -->
              <div class="flex items-center gap-2 sm:flex-shrink-0">
                {#if agent.status === 'online'}
                  <button
                    onclick={() => forceDisconnectAgent(agent)}
                    class="px-3 py-1.5 text-xs bg-yellow-600/20 text-yellow-400 hover:bg-yellow-600/30 rounded-lg transition-colors flex items-center gap-1"
                    title="强制断开连接"
                  >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728A9 9 0 015.636 5.636m12.728 12.728L5.636 5.636" />
                    </svg>
                    <span class="hidden sm:inline">Disconnect</span>
                  </button>
                {/if}
                <button
                  onclick={() => deleteAgent(agent)}
                  class="px-3 py-1.5 text-xs bg-red-600/20 text-red-400 hover:bg-red-600/30 rounded-lg transition-colors flex items-center gap-1"
                  title="删除 Agent"
                >
                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                  </svg>
                  <span class="hidden sm:inline">Delete</span>
                </button>
              </div>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>

<!-- Confirm Dialog -->
{#if confirmAction}
  <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
    <div class="bg-theme-secondary rounded-lg border border-theme p-6 max-w-md w-full">
      <h3 class="text-lg font-semibold text-theme-primary mb-2">
        {confirmAction.type === 'disconnect' ? 'Force Disconnect Agent' : 'Delete Agent'}
      </h3>
      <p class="text-theme-secondary mb-4">
        {#if confirmAction.type === 'disconnect'}
          确定要强制断开 Agent <span class="font-mono text-yellow-400">{confirmAction.agent.name}</span> 的连接吗？
          <br /><span class="text-sm text-yellow-500">该操作会立即断开 Agent 连接，但不会删除其记录。</span>
        {:else}
          确定要删除 Agent <span class="font-mono text-red-400">{confirmAction.agent.name}</span> 吗？
          <br /><span class="text-sm text-red-500">该操作不可恢复，Agent 的所有数据将被永久删除！</span>
        {/if}
      </p>
      <div class="flex justify-end gap-3">
        <button
          onclick={closeConfirmDialog}
          class="px-4 py-2 text-sm bg-theme-primary border border-theme rounded-lg text-theme-primary hover:bg-theme-hover transition-colors"
        >
          取消
        </button>
        <button
          onclick={executeAction}
          class="px-4 py-2 text-sm rounded-lg transition-colors {confirmAction.type === 'disconnect' ? 'bg-yellow-600 hover:bg-yellow-500 text-white' : 'bg-red-600 hover:bg-red-500 text-white'}"
        >
          {confirmAction.type === 'disconnect' ? '断开连接' : '确认删除'}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Audit Log Viewer -->
{#if showAuditLogs}
  <AuditLogViewer onClose={() => showAuditLogs = false} />
{/if}
