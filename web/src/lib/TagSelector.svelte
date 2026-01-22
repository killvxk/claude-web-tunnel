<script lang="ts">
  import { allTags, selectedTags, toggleTagFilter, clearTagFilters } from '../stores/tags';
  import { wsService } from '../services/websocket';
  import { onMount } from 'svelte';

  onMount(() => {
    // 加载所有标签
    wsService.requestAllTags();
  });
</script>

{#if $allTags.length > 0}
  <div class="tag-selector">
    <div class="tag-header">
      <span class="tag-label">标签过滤:</span>
      {#if $selectedTags.length > 0}
        <button
          class="clear-tags"
          onclick={() => clearTagFilters()}
          title="清除所有过滤"
        >
          清除 ({$selectedTags.length})
        </button>
      {/if}
    </div>
    <div class="tag-list">
      {#each $allTags as tag}
        <button
          class="tag-chip"
          class:selected={$selectedTags.includes(tag)}
          onclick={() => toggleTagFilter(tag)}
        >
          {tag}
        </button>
      {/each}
    </div>
  </div>
{/if}

<style>
  .tag-selector {
    padding: 0.75rem;
    background: var(--bg-secondary, #1f2937);
    border-radius: 0.5rem;
    margin-bottom: 1rem;
  }

  .tag-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.5rem;
  }

  .tag-label {
    font-size: 0.875rem;
    color: var(--text-secondary, #9ca3af);
  }

  .clear-tags {
    font-size: 0.75rem;
    padding: 0.25rem 0.5rem;
    background: transparent;
    border: 1px solid var(--border-color, #374151);
    border-radius: 0.25rem;
    color: var(--text-secondary, #9ca3af);
    cursor: pointer;
    transition: all 0.2s;
  }

  .clear-tags:hover {
    background: var(--bg-tertiary, #374151);
    color: var(--text-primary, #f3f4f6);
  }

  .tag-list {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
  }

  .tag-chip {
    display: inline-flex;
    align-items: center;
    padding: 0.25rem 0.75rem;
    font-size: 0.875rem;
    background: var(--bg-tertiary, #374151);
    border: 1px solid var(--border-color, #4b5563);
    border-radius: 9999px;
    color: var(--text-primary, #f3f4f6);
    cursor: pointer;
    transition: all 0.2s;
  }

  .tag-chip:hover {
    background: var(--bg-hover, #4b5563);
  }

  .tag-chip.selected {
    background: var(--accent-color, #3b82f6);
    border-color: var(--accent-color, #3b82f6);
    color: white;
  }

  .tag-chip.selected:hover {
    background: var(--accent-hover, #2563eb);
    border-color: var(--accent-hover, #2563eb);
  }
</style>
