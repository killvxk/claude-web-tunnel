<script lang="ts">
  import { allTags, agentTags, getAgentTagsSync } from '../stores/tags';
  import { wsService } from '../services/websocket';

  interface Props {
    agentId: string;
    agentName: string;
    onClose: () => void;
  }

  let { agentId, agentName, onClose }: Props = $props();

  let newTag = $state('');
  let currentTags = $derived(getAgentTagsSync(agentId));

  // 加载该 Agent 的标签
  $effect(() => {
    wsService.requestAgentTags(agentId);
  });

  function handleAddTag(): void {
    const trimmedTag = newTag.trim();
    if (trimmedTag && !currentTags.includes(trimmedTag)) {
      wsService.addAgentTag(agentId, trimmedTag);
      newTag = '';
    }
  }

  function handleRemoveTag(tag: string): void {
    wsService.removeAgentTag(agentId, tag);
  }

  function handleKeyDown(event: KeyboardEvent): void {
    if (event.key === 'Enter') {
      handleAddTag();
    }
  }

  function handleSelectExisting(tag: string): void {
    if (!currentTags.includes(tag)) {
      wsService.addAgentTag(agentId, tag);
    }
  }
</script>

<div class="tag-editor-overlay" onclick={onClose} role="presentation">
  <div class="tag-editor" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true">
    <div class="editor-header">
      <h3>编辑标签</h3>
      <span class="agent-name">{agentName}</span>
      <button class="close-btn" onclick={onClose} aria-label="关闭">×</button>
    </div>

    <div class="current-tags">
      <label>当前标签:</label>
      <div class="tags-container">
        {#if currentTags.length === 0}
          <span class="no-tags">暂无标签</span>
        {:else}
          {#each currentTags as tag}
            <span class="tag-badge">
              {tag}
              <button class="remove-tag" onclick={() => handleRemoveTag(tag)} aria-label="移除标签">×</button>
            </span>
          {/each}
        {/if}
      </div>
    </div>

    <div class="add-tag-section">
      <label>添加新标签:</label>
      <div class="input-row">
        <input
          type="text"
          bind:value={newTag}
          onkeydown={handleKeyDown}
          placeholder="输入标签名..."
          class="tag-input"
        />
        <button class="add-btn" onclick={handleAddTag} disabled={!newTag.trim()}>
          添加
        </button>
      </div>
    </div>

    {#if $allTags.filter(t => !currentTags.includes(t)).length > 0}
      <div class="existing-tags">
        <label>快速选择现有标签:</label>
        <div class="tags-container">
          {#each $allTags.filter(t => !currentTags.includes(t)) as tag}
            <button class="existing-tag" onclick={() => handleSelectExisting(tag)}>
              + {tag}
            </button>
          {/each}
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .tag-editor-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .tag-editor {
    background: var(--bg-secondary, #1f2937);
    border-radius: 0.75rem;
    padding: 1.5rem;
    width: 90%;
    max-width: 480px;
    max-height: 80vh;
    overflow-y: auto;
    box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.5);
  }

  .editor-header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 1.5rem;
    position: relative;
  }

  .editor-header h3 {
    margin: 0;
    font-size: 1.125rem;
    color: var(--text-primary, #f3f4f6);
  }

  .agent-name {
    font-size: 0.875rem;
    color: var(--text-secondary, #9ca3af);
    padding: 0.125rem 0.5rem;
    background: var(--bg-tertiary, #374151);
    border-radius: 0.25rem;
  }

  .close-btn {
    position: absolute;
    right: 0;
    top: -0.5rem;
    background: transparent;
    border: none;
    font-size: 1.5rem;
    color: var(--text-secondary, #9ca3af);
    cursor: pointer;
    padding: 0.25rem 0.5rem;
    line-height: 1;
  }

  .close-btn:hover {
    color: var(--text-primary, #f3f4f6);
  }

  .current-tags,
  .add-tag-section,
  .existing-tags {
    margin-bottom: 1.25rem;
  }

  label {
    display: block;
    font-size: 0.875rem;
    color: var(--text-secondary, #9ca3af);
    margin-bottom: 0.5rem;
  }

  .tags-container {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    min-height: 2rem;
  }

  .no-tags {
    font-size: 0.875rem;
    color: var(--text-muted, #6b7280);
    font-style: italic;
  }

  .tag-badge {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.25rem 0.5rem;
    background: var(--accent-color, #3b82f6);
    border-radius: 0.25rem;
    font-size: 0.875rem;
    color: white;
  }

  .remove-tag {
    background: transparent;
    border: none;
    color: rgba(255, 255, 255, 0.7);
    font-size: 1rem;
    cursor: pointer;
    padding: 0;
    line-height: 1;
    margin-left: 0.25rem;
  }

  .remove-tag:hover {
    color: white;
  }

  .input-row {
    display: flex;
    gap: 0.5rem;
  }

  .tag-input {
    flex: 1;
    padding: 0.5rem 0.75rem;
    font-size: 0.875rem;
    background: var(--bg-primary, #111827);
    border: 1px solid var(--border-color, #374151);
    border-radius: 0.375rem;
    color: var(--text-primary, #f3f4f6);
    outline: none;
  }

  .tag-input:focus {
    border-color: var(--accent-color, #3b82f6);
  }

  .add-btn {
    padding: 0.5rem 1rem;
    font-size: 0.875rem;
    background: var(--accent-color, #3b82f6);
    border: none;
    border-radius: 0.375rem;
    color: white;
    cursor: pointer;
    transition: background 0.2s;
  }

  .add-btn:hover:not(:disabled) {
    background: var(--accent-hover, #2563eb);
  }

  .add-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .existing-tag {
    padding: 0.25rem 0.5rem;
    font-size: 0.875rem;
    background: var(--bg-tertiary, #374151);
    border: 1px dashed var(--border-color, #4b5563);
    border-radius: 0.25rem;
    color: var(--text-secondary, #9ca3af);
    cursor: pointer;
    transition: all 0.2s;
  }

  .existing-tag:hover {
    background: var(--bg-hover, #4b5563);
    color: var(--text-primary, #f3f4f6);
    border-style: solid;
  }
</style>
