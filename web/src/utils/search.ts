/**
 * Search utilities for instance filtering and highlighting
 */

import type { AgentInstance } from '../types';

export interface SearchOptions {
  query: string;
  fields?: ('cwd' | 'id')[];
}

/**
 * Debounce function to limit the rate of function calls
 */
export function debounce<T extends (...args: never[]) => void>(
  fn: T,
  delay: number
): (...args: Parameters<T>) => void {
  let timeoutId: ReturnType<typeof setTimeout> | null = null;

  return (...args: Parameters<T>) => {
    if (timeoutId) {
      clearTimeout(timeoutId);
    }
    timeoutId = setTimeout(() => {
      fn(...args);
      timeoutId = null;
    }, delay);
  };
}

/**
 * Escape special regex characters in a string
 */
function escapeRegex(str: string): string {
  return str.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

/**
 * Split query into tokens for fuzzy matching
 */
function tokenizeQuery(query: string): string[] {
  return query
    .toLowerCase()
    .split(/\s+/)
    .filter(token => token.length > 0);
}

/**
 * Check if text matches all query tokens
 */
function matchesAllTokens(text: string, tokens: string[]): boolean {
  const lowerText = text.toLowerCase();
  return tokens.every(token => lowerText.includes(token));
}

/**
 * Search instances with multiple field support
 */
export function searchInstances(
  instances: AgentInstance[],
  options: SearchOptions
): AgentInstance[] {
  const { query, fields = ['cwd', 'id'] } = options;

  if (!query.trim()) {
    return instances;
  }

  const tokens = tokenizeQuery(query);

  return instances.filter(instance => {
    // Check each field
    for (const field of fields) {
      let value = '';
      switch (field) {
        case 'cwd':
          value = instance.cwd;
          break;
        case 'id':
          value = instance.id;
          break;
      }

      if (matchesAllTokens(value, tokens)) {
        return true;
      }
    }

    return false;
  });
}

/**
 * Highlight matching text with mark tags
 * Returns HTML string with <mark> tags around matches
 */
export function highlightMatch(text: string, query: string): string {
  if (!query.trim()) {
    return escapeHtml(text);
  }

  const tokens = tokenizeQuery(query);
  let result = escapeHtml(text);

  // Sort tokens by length (longest first) to avoid partial replacements
  const sortedTokens = [...tokens].sort((a, b) => b.length - a.length);

  for (const token of sortedTokens) {
    const regex = new RegExp(`(${escapeRegex(token)})`, 'gi');
    result = result.replace(regex, '<mark class="bg-yellow-500/30 text-inherit rounded px-0.5">$1</mark>');
  }

  return result;
}

/**
 * Escape HTML special characters
 */
function escapeHtml(text: string): string {
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}

/**
 * Get display name from cwd (last directory)
 */
export function getDisplayName(cwd: string): string {
  const parts = cwd.replace(/\\/g, '/').split('/').filter(Boolean);
  return parts[parts.length - 1] || cwd;
}

/**
 * Get parent directory as group key
 */
export function getGroupKey(cwd: string): string {
  const normalized = cwd.replace(/\\/g, '/');
  const parts = normalized.split('/').filter(Boolean);
  if (parts.length <= 1) return '/';
  return '/' + parts.slice(0, -1).join('/');
}
