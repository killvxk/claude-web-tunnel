/**
 * Search history store with localStorage persistence
 */

import { writable } from 'svelte/store';

const STORAGE_KEY = 'search_history';
const MAX_HISTORY = 10;

function loadHistory(): string[] {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      const parsed = JSON.parse(stored);
      if (Array.isArray(parsed)) {
        return parsed.slice(0, MAX_HISTORY);
      }
    }
  } catch (e) {
    console.warn('Failed to load search history:', e);
  }
  return [];
}

function saveHistory(history: string[]): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(history));
  } catch (e) {
    console.warn('Failed to save search history:', e);
  }
}

function createSearchHistoryStore() {
  const initial = loadHistory();
  const { subscribe, set, update } = writable<string[]>(initial);

  return {
    subscribe,

    /**
     * Add a query to history (moves to front if already exists)
     */
    add: (query: string) => {
      const trimmed = query.trim();
      if (!trimmed) return;

      update(history => {
        // Remove if already exists
        const filtered = history.filter(q => q !== trimmed);
        // Add to front
        const newHistory = [trimmed, ...filtered].slice(0, MAX_HISTORY);
        saveHistory(newHistory);
        return newHistory;
      });
    },

    /**
     * Remove a specific query from history
     */
    remove: (query: string) => {
      update(history => {
        const newHistory = history.filter(q => q !== query);
        saveHistory(newHistory);
        return newHistory;
      });
    },

    /**
     * Clear all history
     */
    clear: () => {
      set([]);
      localStorage.removeItem(STORAGE_KEY);
    },
  };
}

export const searchHistory = createSearchHistoryStore();
