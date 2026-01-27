<script lang="ts">
  import { wsService } from '../services/websocket';
  import type { AuditLogEntry } from '../types';
  import { onMount, onDestroy } from 'svelte';

  interface Props {
    onClose: () => void;
  }

  let { onClose }: Props = $props();

  // State
  let logs = $state<AuditLogEntry[]>([]);
  let total = $state(0);
  let loading = $state(false);
  let currentPage = $state(1);
  let pageSize = $state(20);
  let eventTypeFilter = $state('');

  // Event types for filter
  const eventTypes = [
    { value: '', label: '全部事件' },
    { value: 'auth_success', label: '认证成功' },
    { value: 'auth_failure', label: '认证失败' },
    { value: 'create_instance', label: '创建实例' },
    { value: 'close_instance', label: '关闭实例' },
    { value: 'attach', label: '连接终端' },
    { value: 'detach', label: '断开终端' },
    { value: 'force_disconnect_agent', label: '强制断开代理' },
    { value: 'force_close_instance', label: '强制关闭实例' },
    { value: 'delete_agent', label: '删除代理' },
    { value: 'add_agent_tag', label: '添加标签' },
    { value: 'remove_agent_tag', label: '移除标签' },
    { value: 'select_working_agent', label: '选择工作代理' },
    { value: 'clear_working_agent', label: '清除工作代理' },
  ];

  // Derived
  let totalPages = $derived(Math.ceil(total / pageSize) || 1);

  // Load audit logs
  function loadLogs() {
    loading = true;
    const offset = (currentPage - 1) * pageSize;
    wsService.requestAuditLogs(
      pageSize,
      offset,
      eventTypeFilter || undefined
    );
  }

  // Handle audit log response
  function handleAuditLogResponse(receivedLogs: AuditLogEntry[], receivedTotal: number) {
    logs = receivedLogs;
    total = receivedTotal;
    loading = false;
  }

  // Pagination
  function goToPage(page: number) {
    if (page >= 1 && page <= totalPages) {
      currentPage = page;
      loadLogs();
    }
  }

  // Filter change
  function handleFilterChange() {
    currentPage = 1;
    loadLogs();
  }

  // Format timestamp
  function formatTime(timestamp: string): string {
    const date = new Date(timestamp);
    return date.toLocaleString('zh-CN', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    });
  }

  // Format event type for display
  function formatEventType(eventType: string): string {
    return eventType.replace(/_/g, ' ').replace(/\b\w/g, l => l.toUpperCase());
  }

  // Get event type color
  function getEventTypeColor(eventType: string): string {
    if (eventType.includes('failure') || eventType.includes('delete')) {
      return 'text-red-400 bg-red-900/30';
    }
    if (eventType.includes('success') || eventType.includes('create')) {
      return 'text-green-400 bg-green-900/30';
    }
    if (eventType.includes('force')) {
      return 'text-yellow-400 bg-yellow-900/30';
    }
    return 'text-blue-400 bg-blue-900/30';
  }

  // Truncate UUID for display
  function truncateUuid(uuid: string | null): string {
    if (!uuid) return '-';
    return uuid.substring(0, 8) + '...';
  }

  onMount(() => {
    wsService.setAuditLogHandler(handleAuditLogResponse);
    loadLogs();
  });

  onDestroy(() => {
    wsService.setAuditLogHandler(null);
  });
</script>

<div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
  <div class="bg-theme-secondary rounded-lg border border-theme w-full max-w-5xl max-h-[90vh] flex flex-col">
    <!-- Header -->
    <div class="flex items-center justify-between p-4 border-b border-theme flex-shrink-0">
      <div>
        <h2 class="text-lg font-semibold text-theme-primary">审计日志</h2>
        <p class="text-xs text-theme-secondary">共 {total} 条记录</p>
      </div>
      <button
        onclick={onClose}
        class="p-2 text-theme-secondary hover:text-theme-primary hover:bg-theme-hover rounded-lg transition-colors"
        title="关闭"
      >
        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>
    </div>

    <!-- Filter -->
    <div class="p-4 border-b border-theme flex-shrink-0">
      <div class="flex items-center gap-4">
        <label class="flex items-center gap-2">
          <span class="text-sm text-theme-secondary">事件类型：</span>
          <select
            bind:value={eventTypeFilter}
            onchange={handleFilterChange}
            class="bg-theme-primary border border-theme rounded-lg px-3 py-1.5 text-sm text-theme-primary focus:outline-none focus:ring-2 focus:ring-blue-500"
          >
            {#each eventTypes as type}
              <option value={type.value}>{type.label}</option>
            {/each}
          </select>
        </label>
        <button
          onclick={loadLogs}
          class="px-3 py-1.5 text-sm bg-blue-600 hover:bg-blue-500 text-white rounded-lg transition-colors flex items-center gap-1"
          disabled={loading}
        >
          <svg class="w-4 h-4 {loading ? 'animate-spin' : ''}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
          </svg>
          刷新
        </button>
      </div>
    </div>

    <!-- Table -->
    <div class="flex-1 overflow-auto p-4">
      {#if loading && logs.length === 0}
        <div class="flex items-center justify-center h-full">
          <div class="text-center">
            <svg class="w-12 h-12 mx-auto mb-4 text-theme-secondary animate-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
            </svg>
            <p class="text-theme-secondary">加载中...</p>
          </div>
        </div>
      {:else if logs.length === 0}
        <div class="flex items-center justify-center h-full">
          <div class="text-center">
            <svg class="w-16 h-16 mx-auto mb-4 text-theme-secondary opacity-50" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
            </svg>
            <p class="text-theme-secondary text-lg">暂无审计日志</p>
          </div>
        </div>
      {:else}
        <table class="w-full text-sm">
          <thead class="text-left text-theme-secondary border-b border-theme">
            <tr>
              <th class="pb-2 pr-4">时间</th>
              <th class="pb-2 pr-4">事件</th>
              <th class="pb-2 pr-4">角色</th>
              <th class="pb-2 pr-4">代理 ID</th>
              <th class="pb-2 pr-4">实例 ID</th>
              <th class="pb-2 pr-4">客户端 IP</th>
              <th class="pb-2 pr-4">状态</th>
              <th class="pb-2">详情</th>
            </tr>
          </thead>
          <tbody class="text-theme-primary">
            {#each logs as log (log.id)}
              <tr class="border-b border-theme/50 hover:bg-theme-hover transition-colors">
                <td class="py-2 pr-4 whitespace-nowrap text-xs font-mono">{formatTime(log.timestamp)}</td>
                <td class="py-2 pr-4">
                  <span class="px-2 py-0.5 rounded text-xs font-medium {getEventTypeColor(log.event_type)}">
                    {formatEventType(log.event_type)}
                  </span>
                </td>
                <td class="py-2 pr-4 text-xs">{log.user_role}</td>
                <td class="py-2 pr-4 text-xs font-mono" title={log.agent_id || undefined}>{truncateUuid(log.agent_id)}</td>
                <td class="py-2 pr-4 text-xs font-mono" title={log.instance_id || undefined}>{truncateUuid(log.instance_id)}</td>
                <td class="py-2 pr-4 text-xs font-mono">{log.client_ip}</td>
                <td class="py-2 pr-4">
                  {#if log.success}
                    <span class="text-green-400">成功</span>
                  {:else}
                    <span class="text-red-400">失败</span>
                  {/if}
                </td>
                <td class="py-2 text-xs text-theme-secondary max-w-xs truncate" title={log.details || undefined}>
                  {log.details || '-'}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </div>

    <!-- Pagination -->
    <div class="flex items-center justify-between p-4 border-t border-theme flex-shrink-0">
      <div class="text-sm text-theme-secondary">
        显示 {(currentPage - 1) * pageSize + 1} - {Math.min(currentPage * pageSize, total)} 条，共 {total} 条
      </div>
      <div class="flex items-center gap-2">
        <button
          onclick={() => goToPage(1)}
          disabled={currentPage === 1 || loading}
          class="px-2 py-1 text-sm bg-theme-primary border border-theme rounded text-theme-secondary hover:bg-theme-hover disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        >
          首页
        </button>
        <button
          onclick={() => goToPage(currentPage - 1)}
          disabled={currentPage === 1 || loading}
          class="px-2 py-1 text-sm bg-theme-primary border border-theme rounded text-theme-secondary hover:bg-theme-hover disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        >
          上一页
        </button>
        <span class="text-sm text-theme-primary px-2">
          第 {currentPage} / {totalPages} 页
        </span>
        <button
          onclick={() => goToPage(currentPage + 1)}
          disabled={currentPage === totalPages || loading}
          class="px-2 py-1 text-sm bg-theme-primary border border-theme rounded text-theme-secondary hover:bg-theme-hover disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        >
          下一页
        </button>
        <button
          onclick={() => goToPage(totalPages)}
          disabled={currentPage === totalPages || loading}
          class="px-2 py-1 text-sm bg-theme-primary border border-theme rounded text-theme-secondary hover:bg-theme-hover disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        >
          末页
        </button>
      </div>
    </div>
  </div>
</div>
