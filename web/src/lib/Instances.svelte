<script lang="ts">
  import { app, auth, instances, wsStatus } from '../stores';
  import { theme } from '../stores/theme';
  import { searchHistory } from '../stores/searchHistory';
  import { allTags, selectedTags, groupMode, agentTags, setGroupMode, clearTagFilters } from '../stores/tags';
  import { wsService } from '../services/websocket';
  import { searchInstances, highlightMatch, getGroupKey, getDisplayName, debounce } from '../utils/search';
  import TagSelector from './TagSelector.svelte';
  import TagEditor from './TagEditor.svelte';
  import type { AgentInstance, InstanceStatus } from '../types';

  let selectedInstance: AgentInstance | null = $state(null);
  let isConnecting = $state(false);
  let searchQuery = $state('');
  let debouncedQuery = $state('');
  let showSearchHistory = $state(false);
  let searchInputEl: HTMLInputElement;

  // Admin 创建实例相关
  let showCreateModal = $state(false);
  let newInstanceCwd = $state('');
  let isCreating = $state(false);

  // 标签编辑相关
  let showTagEditor = $state(false);
  let tagEditingAgentId = $state('');
  let tagEditingAgentName = $state('');

  // 分组视图相关
  type ViewMode = 'list' | 'grouped' | 'tag';
  let viewMode: ViewMode = $state((localStorage.getItem('instances_view_mode') as ViewMode) || 'list');
  let collapsedGroups: Set<string> = $state(new Set());

  // Debounced search update
  const updateDebouncedQuery = debounce((query: string) => {
    debouncedQuery = query;
  }, 150);

  // Watch searchQuery changes
  $effect(() => {
    updateDebouncedQuery(searchQuery);
  });

  // 保存视图模式到 localStorage
  function setViewMode(mode: ViewMode) {
    viewMode = mode;
    localStorage.setItem('instances_view_mode', mode);
  }

  // 检查是否有 Admin 权限
  let canManageInstances = $derived($auth.role === 'admin' || $auth.role === 'super_admin');

  // 过滤实例 - 使用增强搜索
  let filteredInstances = $derived(
    searchInstances($instances, { query: debouncedQuery, fields: ['cwd', 'id'] })
  );

  // 按父目录分组实例
  let groupedInstances = $derived((() => {
    const groups = new Map<string, AgentInstance[]>();
    for (const instance of filteredInstances) {
      const key = getGroupKey(instance.cwd);
      if (!groups.has(key)) {
        groups.set(key, []);
      }
      groups.get(key)!.push(instance);
    }
    // 转换为数组并排序
    return Array.from(groups.entries())
      .map(([path, items]) => ({ path, instances: items }))
      .sort((a, b) => a.path.localeCompare(b.path));
  })());

  // 按标签分组实例 (基于 agent_id)
  let tagGroupedInstances = $derived((() => {
    const groups = new Map<string, AgentInstance[]>();
    const noTagGroup: AgentInstance[] = [];

    for (const instance of filteredInstances) {
      const tags = $agentTags.get(instance.agent_id) || [];
      if (tags.length === 0) {
        noTagGroup.push(instance);
      } else {
        for (const tag of tags) {
          if (!groups.has(tag)) {
            groups.set(tag, []);
          }
          groups.get(tag)!.push(instance);
        }
      }
    }

    // 转换为数组并排序
    const result = Array.from(groups.entries())
      .map(([tag, items]) => ({ tag, instances: items }))
      .sort((a, b) => a.tag.localeCompare(b.tag));

    // 将无标签组放在最后
    if (noTagGroup.length > 0) {
      result.push({ tag: '未分类', instances: noTagGroup });
    }

    return result;
  })());

  // 切换分组折叠状态
  function toggleGroup(path: string) {
    if (collapsedGroups.has(path)) {
      collapsedGroups.delete(path);
    } else {
      collapsedGroups.add(path);
    }
    collapsedGroups = new Set(collapsedGroups); // 触发响应式更新
  }

  // 检查分组是否折叠
  function isGroupCollapsed(path: string): boolean {
    return collapsedGroups.has(path);
  }

  // 刷新实例列表
  function refreshInstances() {
    wsService.requestInstances();
  }

  // 连接到实例 - 只有 running 状态才能连接
  async function connectToInstance(instance: AgentInstance) {
    if (instance.status !== 'running') {
      return;
    }

    selectedInstance = instance;
    isConnecting = true;

    try {
      wsService.attachToInstance(instance.id);
    } catch (error) {
      console.error('Failed to connect:', error);
      app.setError('连接实例失败');
    } finally {
      isConnecting = false;
    }
  }

  // 搜索提交 - 保存到历史
  function handleSearchSubmit() {
    if (searchQuery.trim()) {
      searchHistory.add(searchQuery.trim());
      showSearchHistory = false;
    }
  }

  // 选择历史记录
  function selectHistoryItem(query: string) {
    searchQuery = query;
    debouncedQuery = query;
    showSearchHistory = false;
    searchInputEl?.focus();
  }

  // 删除历史记录
  function removeHistoryItem(e: Event, query: string) {
    e.stopPropagation();
    searchHistory.remove(query);
  }

  // 清除搜索
  function clearSearch() {
    searchQuery = '';
    debouncedQuery = '';
    searchInputEl?.focus();
  }

  // 创建新实例 (Admin only)
  function openCreateModal() {
    newInstanceCwd = '';
    showCreateModal = true;
  }

  function closeCreateModal() {
    showCreateModal = false;
    newInstanceCwd = '';
    isCreating = false;
  }

  async function createInstance() {
    if (!newInstanceCwd.trim()) {
      app.setError('请输入工作目录');
      return;
    }

    isCreating = true;
    try {
      wsService.createInstance(newInstanceCwd.trim());
      closeCreateModal();
    } catch (error) {
      console.error('Failed to create instance:', error);
      app.setError('创建实例失败');
    } finally {
      isCreating = false;
    }
  }

  // 关闭实例 (Admin only)
  function closeInstance(event: Event, instance: AgentInstance) {
    event.stopPropagation(); // 阻止点击事件冒泡到父元素
    if (confirm(`确定要关闭实例 "${getDisplayName(instance.cwd)}" 吗？`)) {
      wsService.closeInstance(instance.id);
    }
  }

  // 打开标签编辑器 (Admin only)
  function openTagEditor(event: Event, instance: AgentInstance) {
    event.stopPropagation(); // 阻止点击事件冒泡到父元素
    tagEditingAgentId = instance.agent_id;
    // 使用 cwd 的项目名称作为显示名，或者 agent_id 的前8位
    tagEditingAgentName = getDisplayName(instance.cwd) || instance.agent_id.substring(0, 8);
    showTagEditor = true;
  }

  // 关闭标签编辑器
  function closeTagEditor() {
    showTagEditor = false;
    tagEditingAgentId = '';
    tagEditingAgentName = '';
  }

  // 退出登录
  function handleLogout() {
    wsService.disconnect();
    auth.logout();
    app.reset();
  }

  // 格式化时间 - 使用 created_at 字段
  function formatTime(dateString: string): string {
    const date = new Date(dateString);
    return date.toLocaleString('zh-CN', {
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
    });
  }

  // 获取状态颜色 - 适配后端的 InstanceStatus
  function getStatusColor(status: InstanceStatus): string {
    switch (status) {
      case 'running':
        return 'bg-green-500';
      case 'suspended':
        return 'bg-yellow-500';
      case 'stopped':
        return 'bg-gray-500';
      default:
        return 'bg-gray-500';
    }
  }

  // 获取状态文本
  function getStatusText(status: InstanceStatus): string {
    switch (status) {
      case 'running':
        return '运行中';
      case 'suspended':
        return '已暂停';
      case 'stopped':
        return '已停止';
      default:
        return '未知';
    }
  }

  // 格式化角色显示
  function formatRole(role: string | null): string {
    if (!role) return '-';
    switch (role) {
      case 'super_admin':
        return 'SuperAdmin';
      case 'admin':
        return 'Admin';
      case 'user':
        return 'User';
      default:
        return role;
    }
  }
</script>

<div class="min-h-screen bg-theme-primary">
  <!-- Header -->
  <header class="bg-theme-secondary border-b border-theme">
    <div class="max-w-7xl mx-auto px-4 py-4">
      <div class="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div class="flex items-center gap-3 sm:gap-4">
          <div class="inline-flex items-center justify-center w-10 h-10 bg-blue-600 rounded-lg">
            <svg class="w-6 h-6 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                    d="M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z" />
            </svg>
          </div>
          <div>
            <h1 class="text-xl font-bold text-theme-primary">Claude Web Tunnel</h1>
            <p class="text-sm text-theme-secondary">选择一个实例开始工作</p>
          </div>
        </div>

        <div class="flex flex-wrap items-center justify-start gap-2 sm:flex-nowrap sm:justify-end sm:gap-4">
          <!-- User Info -->
          <div class="flex items-center gap-2 sm:gap-3 text-sm">
            <div class="flex items-center">
              <span class="w-2 h-2 rounded-full mr-2 {
                $wsStatus.status === 'connected' ? 'bg-green-500' :
                $wsStatus.status === 'connecting' ? 'bg-yellow-500 animate-pulse' :
                $wsStatus.status === 'reconnecting' ? 'bg-yellow-500 animate-pulse' :
                'bg-gray-500'
              }"></span>
              <span class="text-gray-400">
                {$auth.agentName || 'Server'}
              </span>
            </div>
            <span class="px-2 py-0.5 bg-gray-700 rounded text-gray-300 text-xs">
              {formatRole($auth.role)}
            </span>
          </div>

          <!-- Admin Panel Button (SuperAdmin only) -->
          {#if $auth.role === 'super_admin'}
            <button
              onclick={() => app.setView('admin')}
              class="p-2 text-purple-400 hover:text-purple-300 hover:bg-purple-900/30 rounded-lg transition-colors"
              title="管理面板"
            >
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
              </svg>
            </button>
          {/if}

          <!-- Theme Toggle -->
          <button
            onclick={() => theme.toggle()}
            class="p-2 text-gray-400 hover:text-white hover:bg-gray-700 rounded-lg transition-colors"
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

          <button
            onclick={handleLogout}
            class="px-4 py-2 text-sm bg-gray-700 hover:bg-gray-600 text-white rounded-lg transition-colors"
          >
            退出登录
          </button>
        </div>
      </div>
    </div>
  </header>

  <!-- Main Content -->
  <main class="max-w-7xl mx-auto px-2 sm:px-4 py-4 sm:py-8">
    <!-- Toolbar -->
    <div class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3 mb-4 sm:mb-6">
      <div class="relative flex-1 sm:max-w-md">
        <input
          type="text"
          bind:this={searchInputEl}
          bind:value={searchQuery}
          placeholder="搜索实例 (支持多关键词)..."
          class="w-full pl-10 pr-10 py-2 bg-theme-secondary border border-theme rounded-lg text-theme-primary
                 placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-blue-500
                 focus:border-transparent"
          onfocus={() => showSearchHistory = true}
          onblur={() => setTimeout(() => showSearchHistory = false, 200)}
          onkeydown={(e) => e.key === 'Enter' && handleSearchSubmit()}
        />
        <svg class="absolute left-3 top-2.5 w-5 h-5 text-gray-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
        </svg>
        {#if searchQuery}
          <button
            onclick={clearSearch}
            class="absolute right-3 top-2.5 text-gray-500 hover:text-theme-primary transition-colors"
            aria-label="清除搜索"
          >
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        {/if}

        <!-- Search History Dropdown -->
        {#if showSearchHistory && $searchHistory.length > 0 && !searchQuery}
          <div class="absolute top-full left-0 right-0 mt-1 bg-theme-secondary border border-theme rounded-lg shadow-lg z-20 overflow-hidden">
            <div class="px-3 py-2 text-xs text-theme-secondary border-b border-theme flex items-center justify-between">
              <span>搜索历史</span>
              <button
                onclick={() => searchHistory.clear()}
                class="text-red-400 hover:text-red-300 transition-colors"
              >
                清除
              </button>
            </div>
            {#each $searchHistory as query}
              <div
                onclick={() => selectHistoryItem(query)}
                onkeydown={(e) => e.key === 'Enter' && selectHistoryItem(query)}
                role="button"
                tabindex="0"
                class="w-full px-3 py-2 text-left text-sm text-theme-primary hover:bg-theme-hover
                       flex items-center justify-between group transition-colors cursor-pointer"
              >
                <div class="flex items-center gap-2 min-w-0">
                  <svg class="w-4 h-4 text-theme-secondary flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
                  </svg>
                  <span class="truncate">{query}</span>
                </div>
                <button
                  onclick={(e) => removeHistoryItem(e, query)}
                  class="opacity-0 group-hover:opacity-100 text-theme-secondary hover:text-red-400 transition-all"
                  aria-label="删除"
                >
                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                  </svg>
                </button>
              </div>
            {/each}
          </div>
        {/if}
      </div>

      <div class="flex items-center space-x-2">
        <!-- View Mode Toggle -->
        <div class="hidden sm:flex items-center bg-theme-secondary rounded-lg border border-theme p-1">
          <button
            onclick={() => setViewMode('list')}
            class="p-1.5 rounded {viewMode === 'list' ? 'bg-blue-600 text-white' : 'text-theme-secondary hover:text-theme-primary'}"
            title="列表视图"
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 10h16M4 14h16M4 18h16" />
            </svg>
          </button>
          <button
            onclick={() => setViewMode('grouped')}
            class="p-1.5 rounded {viewMode === 'grouped' ? 'bg-blue-600 text-white' : 'text-theme-secondary hover:text-theme-primary'}"
            title="路径分组视图"
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
            </svg>
          </button>
          {#if canManageInstances}
            <button
              onclick={() => setViewMode('tag')}
              class="p-1.5 rounded {viewMode === 'tag' ? 'bg-blue-600 text-white' : 'text-theme-secondary hover:text-theme-primary'}"
              title="标签分组视图"
            >
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 7h.01M7 3h5c.512 0 1.024.195 1.414.586l7 7a2 2 0 010 2.828l-7 7a2 2 0 01-2.828 0l-7-7A1.994 1.994 0 013 12V7a4 4 0 014-4z" />
              </svg>
            </button>
          {/if}
        </div>

        <button
          onclick={refreshInstances}
          class="flex-1 sm:flex-none px-3 sm:px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg
                 transition-colors flex items-center justify-center touch-target"
        >
          <svg class="w-5 h-5 sm:mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                  d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
          </svg>
          <span class="hidden sm:inline">刷新</span>
        </button>

        {#if canManageInstances}
          <button
            onclick={openCreateModal}
            class="flex-1 sm:flex-none px-3 sm:px-4 py-2 bg-green-600 hover:bg-green-700 text-white rounded-lg
                   transition-colors flex items-center justify-center touch-target"
          >
            <svg class="w-5 h-5 sm:mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
            </svg>
            <span class="hidden sm:inline">创建实例</span>
          </button>
        {/if}
      </div>
    </div>

    <!-- Error Message -->
    {#if $app.error}
      <div class="mb-6 p-4 bg-red-900/50 border border-red-700 rounded-lg">
        <div class="flex items-center">
          <svg class="w-5 h-5 text-red-400 mr-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                  d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
          <p class="text-red-400">{$app.error}</p>
          <button
            onclick={() => app.setError(null)}
            class="ml-auto text-red-400 hover:text-red-300"
            aria-label="关闭错误提示"
          >
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>
      </div>
    {/if}

    <!-- Tag Selector (Admin only, shown when in tag view) -->
    {#if canManageInstances && viewMode === 'tag'}
      <div class="mb-4">
        <TagSelector />
      </div>
    {/if}

    <!-- Instances Grid -->
    {#if filteredInstances.length === 0}
      <div class="text-center py-12 sm:py-16">
        <svg class="w-12 sm:w-16 h-12 sm:h-16 mx-auto text-theme-secondary mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                d="M9.172 16.172a4 4 0 015.656 0M9 10h.01M15 10h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
        <h3 class="text-base sm:text-lg font-medium text-theme-secondary mb-2">没有可用的实例</h3>
        <p class="text-sm text-theme-secondary">请等待 Agent 创建实例或刷新列表</p>
      </div>
    {:else if viewMode === 'grouped'}
      <!-- Grouped View -->
      <div class="space-y-4">
        {#each groupedInstances as group (group.path)}
          <div class="bg-theme-secondary/50 rounded-lg border border-theme overflow-hidden">
            <!-- Group Header -->
            <button
              onclick={() => toggleGroup(group.path)}
              class="w-full flex items-center justify-between p-3 hover:bg-theme-hover transition-colors"
            >
              <div class="flex items-center gap-2">
                <svg
                  class="w-4 h-4 text-theme-secondary transition-transform {isGroupCollapsed(group.path) ? '' : 'rotate-90'}"
                  fill="none" stroke="currentColor" viewBox="0 0 24 24"
                >
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
                </svg>
                <svg class="w-5 h-5 text-yellow-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
                </svg>
                <span class="text-theme-primary font-medium text-sm truncate max-w-[300px]" title={group.path}>
                  {group.path}
                </span>
              </div>
              <span class="text-xs text-theme-secondary bg-theme-tertiary px-2 py-0.5 rounded-full">
                {group.instances.length} 实例
              </span>
            </button>

            <!-- Group Content -->
            {#if !isGroupCollapsed(group.path)}
              <div class="p-3 pt-0">
                <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
                  {#each group.instances as instance (instance.id)}
                    {@render instanceCard(instance)}
                  {/each}
                </div>
              </div>
            {/if}
          </div>
        {/each}
      </div>
    {:else if viewMode === 'tag'}
      <!-- Tag Grouped View -->
      <div class="space-y-4">
        {#each tagGroupedInstances as group (group.tag)}
          <div class="bg-theme-secondary/50 rounded-lg border border-theme overflow-hidden">
            <!-- Group Header -->
            <button
              onclick={() => toggleGroup(group.tag)}
              class="w-full flex items-center justify-between p-3 hover:bg-theme-hover transition-colors"
            >
              <div class="flex items-center gap-2">
                <svg
                  class="w-4 h-4 text-theme-secondary transition-transform {isGroupCollapsed(group.tag) ? '' : 'rotate-90'}"
                  fill="none" stroke="currentColor" viewBox="0 0 24 24"
                >
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
                </svg>
                <svg class="w-5 h-5 {group.tag === '未分类' ? 'text-gray-500' : 'text-blue-500'}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 7h.01M7 3h5c.512 0 1.024.195 1.414.586l7 7a2 2 0 010 2.828l-7 7a2 2 0 01-2.828 0l-7-7A1.994 1.994 0 013 12V7a4 4 0 014-4z" />
                </svg>
                <span class="text-theme-primary font-medium text-sm">
                  {group.tag}
                </span>
              </div>
              <span class="text-xs text-theme-secondary bg-theme-tertiary px-2 py-0.5 rounded-full">
                {group.instances.length} 实例
              </span>
            </button>

            <!-- Group Content -->
            {#if !isGroupCollapsed(group.tag)}
              <div class="p-3 pt-0">
                <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
                  {#each group.instances as instance (instance.id)}
                    {@render instanceCard(instance)}
                  {/each}
                </div>
              </div>
            {/if}
          </div>
        {/each}
      </div>
    {:else}
      <!-- List View (default) -->
      <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3 sm:gap-4">
        {#each filteredInstances as instance (instance.id)}
          {@render instanceCard(instance)}
        {/each}
      </div>
    {/if}
  </main>

  <!-- Create Instance Modal -->
  {#if showCreateModal}
    <div class="fixed inset-0 bg-black/50 flex items-end sm:items-center justify-center z-50">
      <div class="bg-theme-secondary rounded-t-lg sm:rounded-lg p-4 sm:p-6 w-full sm:max-w-md sm:mx-4 border-t sm:border border-theme max-h-viewport overflow-y-auto">
        <h2 class="text-lg sm:text-xl font-bold text-theme-primary mb-4">创建新实例</h2>

        <div class="mb-4">
          <label for="cwd" class="block text-sm font-medium text-theme-secondary mb-2">
            工作目录
          </label>
          <input
            id="cwd"
            type="text"
            bind:value={newInstanceCwd}
            placeholder="/path/to/project"
            class="w-full px-4 py-3 sm:py-2 bg-theme-tertiary border border-theme rounded-lg text-theme-primary
                   placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-blue-500
                   focus:border-transparent"
            onkeydown={(e) => e.key === 'Enter' && createInstance()}
          />
          <p class="mt-2 text-xs text-theme-secondary">
            输入 Agent 机器上的项目路径，将在该目录启动 Claude Code
          </p>
        </div>

        <div class="flex flex-col-reverse sm:flex-row sm:justify-end gap-2 sm:space-x-3">
          <button
            onclick={closeCreateModal}
            class="w-full sm:w-auto px-4 py-3 sm:py-2 text-theme-secondary hover:text-theme-primary transition-colors touch-target"
            disabled={isCreating}
          >
            取消
          </button>
          <button
            onclick={createInstance}
            disabled={isCreating || !newInstanceCwd.trim()}
            class="w-full sm:w-auto px-4 py-3 sm:py-2 bg-green-600 hover:bg-green-700 disabled:bg-gray-600
                   disabled:cursor-not-allowed text-white rounded-lg transition-colors
                   flex items-center justify-center touch-target"
          >
            {#if isCreating}
              <svg class="animate-spin w-4 h-4 mr-2" fill="none" viewBox="0 0 24 24">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                <path class="opacity-75" fill="currentColor"
                      d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z">
                </path>
              </svg>
              创建中...
            {:else}
              创建
            {/if}
          </button>
        </div>
      </div>
    </div>
  {/if}
</div>

{#snippet instanceCard(instance: AgentInstance)}
  <div
    onclick={() => connectToInstance(instance)}
    onkeydown={(e) => e.key === 'Enter' && connectToInstance(instance)}
    role="button"
    tabindex="0"
    class="p-4 sm:p-6 bg-theme-secondary border border-theme rounded-lg text-left cursor-pointer
           hover:border-blue-500 transition-all
           {instance.status !== 'running' || isConnecting ? 'opacity-50 cursor-not-allowed hover:border-theme' : ''}
           {selectedInstance?.id === instance.id && isConnecting ? 'border-blue-500 ring-2 ring-blue-500/50' : ''}"
  >
    <div class="flex items-start justify-between mb-3 sm:mb-4">
      <div class="flex items-center min-w-0">
        <div class="w-8 sm:w-10 h-8 sm:h-10 bg-theme-tertiary rounded-lg flex items-center justify-center mr-2 sm:mr-3 flex-shrink-0">
          <svg class="w-5 sm:w-6 h-5 sm:h-6 text-theme-secondary" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                  d="M5 12h14M5 12a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v4a2 2 0 01-2 2M5 12a2 2 0 00-2 2v4a2 2 0 002 2h14a2 2 0 002-2v-4a2 2 0 00-2-2m-2-4h.01M17 16h.01" />
          </svg>
        </div>
        <div class="min-w-0">
          <h3 class="text-theme-primary font-medium truncate">{getDisplayName(instance.cwd)}</h3>
          <p class="text-xs text-theme-secondary font-mono">{instance.id.slice(0, 8)}...</p>
        </div>
      </div>

      <div class="flex items-center flex-shrink-0 ml-2">
        <span class="w-2 h-2 rounded-full mr-1 sm:mr-2 {getStatusColor(instance.status)}"></span>
        <span class="text-xs sm:text-sm text-theme-secondary">{getStatusText(instance.status)}</span>
      </div>
    </div>

    <div class="space-y-1.5 sm:space-y-2 text-xs sm:text-sm">
      <div class="flex items-center justify-between">
        <span class="text-theme-secondary">工作目录</span>
        <span class="text-theme-primary text-xs font-mono truncate max-w-[120px] sm:max-w-[200px]" title={instance.cwd}>
          {instance.cwd}
        </span>
      </div>
      <div class="flex items-center justify-between">
        <span class="text-theme-secondary">创建时间</span>
        <span class="text-theme-primary">{formatTime(instance.created_at)}</span>
      </div>
      <div class="flex items-center justify-between">
        <span class="text-theme-secondary">当前用户</span>
        <span class="text-theme-primary">{instance.attached_users}</span>
      </div>
    </div>

    {#if instance.status === 'running'}
      <div class="mt-3 sm:mt-4 pt-3 sm:pt-4 border-t border-theme flex items-center justify-between">
        <span class="text-blue-400 text-xs sm:text-sm flex items-center">
          {#if selectedInstance?.id === instance.id && isConnecting}
            <svg class="animate-spin w-4 h-4 mr-2" fill="none" viewBox="0 0 24 24">
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
              <path class="opacity-75" fill="currentColor"
                    d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z">
              </path>
            </svg>
            连接中...
          {:else}
            点击连接
            <svg class="w-4 h-4 ml-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
            </svg>
          {/if}
        </span>
        {#if canManageInstances}
          <div class="flex items-center gap-1">
            <button
              onclick={(e) => openTagEditor(e, instance)}
              class="px-2 py-1 text-xs bg-blue-600 hover:bg-blue-700 text-white rounded transition-colors touch-target"
              title="编辑标签"
            >
              标签
            </button>
            <button
              onclick={(e) => closeInstance(e, instance)}
              class="px-2 py-1 text-xs bg-red-600 hover:bg-red-700 text-white rounded transition-colors touch-target"
              title="关闭实例"
            >
              关闭
            </button>
          </div>
        {/if}
      </div>
    {:else if instance.status === 'suspended'}
      <div class="mt-3 sm:mt-4 pt-3 sm:pt-4 border-t border-theme">
        <span class="text-yellow-400 text-xs sm:text-sm">Agent 离线，等待重连</span>
      </div>
    {:else}
      <div class="mt-3 sm:mt-4 pt-3 sm:pt-4 border-t border-theme">
        <span class="text-theme-secondary text-xs sm:text-sm">实例已停止</span>
      </div>
    {/if}
  </div>
{/snippet}

<!-- Tag Editor Modal -->
{#if showTagEditor}
  <TagEditor
    agentId={tagEditingAgentId}
    agentName={tagEditingAgentName}
    onClose={closeTagEditor}
  />
{/if}
