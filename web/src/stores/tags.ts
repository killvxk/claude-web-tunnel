/**
 * Tags store - 管理 Agent 标签状态
 */
import { writable, derived, get } from 'svelte/store';

// 所有可用标签
export const allTags = writable<string[]>([]);

// Agent -> 标签映射
export const agentTags = writable<Map<string, string[]>>(new Map());

// 当前选中的过滤标签
export const selectedTags = writable<string[]>([]);

// 分组模式：'none' | 'path' | 'tag'
export const groupMode = writable<'none' | 'path' | 'tag'>('path');

/**
 * 更新所有标签列表
 */
export function setAllTags(tags: string[]): void {
  allTags.set(tags);
}

/**
 * 设置指定 Agent 的标签
 */
export function setAgentTags(agentId: string, tags: string[]): void {
  agentTags.update(map => {
    const newMap = new Map(map);
    newMap.set(agentId, tags);
    return newMap;
  });
}

/**
 * 添加标签到 Agent（本地更新）
 */
export function addTagToAgent(agentId: string, tag: string): void {
  agentTags.update(map => {
    const newMap = new Map(map);
    const current = newMap.get(agentId) || [];
    if (!current.includes(tag)) {
      newMap.set(agentId, [...current, tag]);
    }
    return newMap;
  });

  // 同时更新 allTags
  allTags.update(tags => {
    if (!tags.includes(tag)) {
      return [...tags, tag].sort();
    }
    return tags;
  });
}

/**
 * 从 Agent 移除标签（本地更新）
 */
export function removeTagFromAgent(agentId: string, tag: string): void {
  agentTags.update(map => {
    const newMap = new Map(map);
    const current = newMap.get(agentId) || [];
    newMap.set(agentId, current.filter(t => t !== tag));
    return newMap;
  });
}

/**
 * 获取指定 Agent 的标签
 */
export function getAgentTagsSync(agentId: string): string[] {
  return get(agentTags).get(agentId) || [];
}

/**
 * 切换标签过滤
 */
export function toggleTagFilter(tag: string): void {
  selectedTags.update(tags => {
    if (tags.includes(tag)) {
      return tags.filter(t => t !== tag);
    }
    return [...tags, tag];
  });
}

/**
 * 清除所有标签过滤
 */
export function clearTagFilters(): void {
  selectedTags.set([]);
}

/**
 * 设置分组模式
 */
export function setGroupMode(mode: 'none' | 'path' | 'tag'): void {
  groupMode.set(mode);
}

/**
 * 清空所有标签数据（登出时调用）
 */
export function clearTags(): void {
  allTags.set([]);
  agentTags.set(new Map());
  selectedTags.set([]);
}

// 派生 store：按标签分组的 Agent ID 列表
export const agentsByTag = derived(agentTags, ($agentTags) => {
  const result = new Map<string, string[]>();

  for (const [agentId, tags] of $agentTags) {
    for (const tag of tags) {
      const agents = result.get(tag) || [];
      agents.push(agentId);
      result.set(tag, agents);
    }
  }

  return result;
});
