import { writable, get } from 'svelte/store';
import type { ITheme } from '@xterm/xterm';

export type ThemeMode = 'dark' | 'light';

// 主题 store
function createThemeStore() {
  // 从 localStorage 恢复，默认 dark
  const stored = typeof localStorage !== 'undefined'
    ? localStorage.getItem('theme') as ThemeMode | null
    : null;
  const initial: ThemeMode = stored || 'dark';

  const { subscribe, set } = writable<ThemeMode>(initial);

  // 初始化时设置 data-theme 属性
  if (typeof document !== 'undefined') {
    document.documentElement.setAttribute('data-theme', initial);
  }

  return {
    subscribe,
    toggle: () => {
      const current = get({ subscribe });
      const next: ThemeMode = current === 'dark' ? 'light' : 'dark';
      set(next);
      if (typeof localStorage !== 'undefined') {
        localStorage.setItem('theme', next);
      }
      if (typeof document !== 'undefined') {
        document.documentElement.setAttribute('data-theme', next);
      }
    },
    setTheme: (theme: ThemeMode) => {
      set(theme);
      if (typeof localStorage !== 'undefined') {
        localStorage.setItem('theme', theme);
      }
      if (typeof document !== 'undefined') {
        document.documentElement.setAttribute('data-theme', theme);
      }
    },
  };
}

export const theme = createThemeStore();

// xterm.js 主题配置
export const terminalThemes: Record<ThemeMode, ITheme> = {
  dark: {
    background: '#1e1e1e',
    foreground: '#d4d4d4',
    cursor: '#d4d4d4',
    cursorAccent: '#1e1e1e',
    selectionBackground: '#264f78',
    selectionForeground: '#ffffff',
    black: '#1e1e1e',
    red: '#f44747',
    green: '#6a9955',
    yellow: '#d7ba7d',
    blue: '#569cd6',
    magenta: '#c586c0',
    cyan: '#4ec9b0',
    white: '#d4d4d4',
    brightBlack: '#808080',
    brightRed: '#f44747',
    brightGreen: '#6a9955',
    brightYellow: '#d7ba7d',
    brightBlue: '#569cd6',
    brightMagenta: '#c586c0',
    brightCyan: '#4ec9b0',
    brightWhite: '#ffffff',
  },
  light: {
    background: '#ffffff',
    foreground: '#24292e',
    cursor: '#24292e',
    cursorAccent: '#ffffff',
    selectionBackground: '#0366d6',
    selectionForeground: '#ffffff',
    black: '#24292e',
    red: '#d73a49',
    green: '#22863a',
    yellow: '#b08800',
    blue: '#0366d6',
    magenta: '#6f42c1',
    cyan: '#1b7c83',
    white: '#6a737d',
    brightBlack: '#959da5',
    brightRed: '#cb2431',
    brightGreen: '#28a745',
    brightYellow: '#dbab09',
    brightBlue: '#2188ff',
    brightMagenta: '#8a63d2',
    brightCyan: '#3192aa',
    brightWhite: '#fafbfc',
  },
};
