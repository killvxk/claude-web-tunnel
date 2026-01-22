import { writable, derived } from 'svelte/store';
import type { AuthState, AppState, AgentInstance, Role, AppView, InstanceStatus, AgentInfo, GlobalStats } from '../types';

// ============================================================================
// Admin Store (SuperAdmin only)
// ============================================================================

export interface AdminState {
  agents: AgentInfo[];
  stats: GlobalStats | null;
  loading: boolean;
  selectedAgentId: string | null;
}

function createAdminStore() {
  const initial: AdminState = {
    agents: [],
    stats: null,
    loading: false,
    selectedAgentId: null,
  };

  const { subscribe, set, update } = writable<AdminState>(initial);

  return {
    subscribe,
    setLoading: (loading: boolean) => update(state => ({ ...state, loading })),
    setStats: (agents: AgentInfo[], stats: GlobalStats) => update(state => ({
      ...state,
      agents,
      stats,
      loading: false,
    })),
    selectAgent: (agentId: string | null) => update(state => ({
      ...state,
      selectedAgentId: agentId,
    })),
    removeAgent: (agentId: string) => update(state => ({
      ...state,
      agents: state.agents.filter(a => a.id !== agentId),
      selectedAgentId: state.selectedAgentId === agentId ? null : state.selectedAgentId,
      stats: state.stats ? {
        ...state.stats,
        total_agents: state.stats.total_agents - 1,
        online_agents: state.agents.find(a => a.id === agentId)?.status === 'online'
          ? state.stats.online_agents - 1
          : state.stats.online_agents,
      } : null,
    })),
    updateAgentStatus: (agentId: string, online: boolean) => update(state => ({
      ...state,
      agents: state.agents.map(a =>
        a.id === agentId ? { ...a, status: online ? 'online' : 'offline' } : a
      ),
      stats: state.stats ? {
        ...state.stats,
        online_agents: online
          ? state.stats.online_agents + 1
          : state.stats.online_agents - 1,
      } : null,
    })),
    reset: () => set(initial),
  };
}

export const admin = createAdminStore();

// 认证状态 store
function createAuthStore() {
  const initial: AuthState = {
    isAuthenticated: false,
    token: null,
    role: null,
    agentName: null,
    agentId: null,
  };

  const { subscribe, set, update } = writable<AuthState>(initial);

  return {
    subscribe,
    login: (token: string, role: Role, agentName: string | null, agentId: string | null) => {
      set({
        isAuthenticated: true,
        token,
        role,
        agentName,
        agentId,
      });
      // 保存到 localStorage
      localStorage.setItem('auth_token', token);
    },
    logout: () => {
      set(initial);
      localStorage.removeItem('auth_token');
    },
    restore: () => {
      const token = localStorage.getItem('auth_token');
      if (token) {
        update(state => ({ ...state, token }));
        return token;
      }
      return null;
    },
  };
}

// 应用状态 store
function createAppStore() {
  const initial: AppState = {
    view: 'login',
    connectedInstanceId: null,
    instances: [],
    error: null,
  };

  const { subscribe, set, update } = writable<AppState>(initial);

  return {
    subscribe,
    setView: (view: AppView) => {
      update(state => ({ ...state, view, error: null }));
    },
    setInstances: (instances: AgentInstance[]) => {
      update(state => ({ ...state, instances }));
    },
    addInstance: (instance: AgentInstance) => {
      update(state => ({
        ...state,
        instances: [...state.instances, instance],
      }));
    },
    removeInstance: (instanceId: string) => {
      update(state => ({
        ...state,
        instances: state.instances.filter(i => i.id !== instanceId),
      }));
    },
    updateInstanceUsers: (instanceId: string, userCount: number) => {
      update(state => ({
        ...state,
        instances: state.instances.map(i =>
          i.id === instanceId ? { ...i, attached_users: userCount } : i
        ),
      }));
    },
    updateInstancesStatusByAgent: (agentId: string, status: InstanceStatus) => {
      update(state => ({
        ...state,
        instances: state.instances.map(i =>
          i.agent_id === agentId ? { ...i, status } : i
        ),
      }));
    },
    connectToInstance: (instanceId: string) => {
      update(state => ({
        ...state,
        connectedInstanceId: instanceId,
        view: 'terminal',
      }));
    },
    disconnectFromInstance: () => {
      update(state => ({
        ...state,
        connectedInstanceId: null,
        view: 'instances',
      }));
    },
    setError: (error: string | null) => {
      update(state => ({ ...state, error }));
    },
    reset: () => {
      set(initial);
    },
  };
}

// WebSocket 连接状态
export type WsStatus = 'disconnected' | 'connecting' | 'connected' | 'reconnecting' | 'error';

export interface WsState {
  status: WsStatus;
  reconnectAttempt: number;
  maxReconnectAttempts: number;
}

function createWsStore() {
  const initial: WsState = {
    status: 'disconnected',
    reconnectAttempt: 0,
    maxReconnectAttempts: 5,
  };

  const { subscribe, set, update } = writable<WsState>(initial);

  return {
    subscribe,
    setStatus: (status: WsStatus) => update(state => ({ ...state, status })),
    setReconnecting: (attempt: number, max: number) => set({
      status: 'reconnecting',
      reconnectAttempt: attempt,
      maxReconnectAttempts: max,
    }),
    reset: () => set(initial),
  };
}

export const auth = createAuthStore();
export const app = createAppStore();
export const wsStatus = createWsStore();

// 派生状态
export const isLoggedIn = derived(auth, $auth => $auth.isAuthenticated);
export const currentView = derived(app, $app => $app.view);
export const currentError = derived(app, $app => $app.error);
export const instances = derived(app, $app => $app.instances);
export const connectedInstance = derived(
  [app, instances],
  ([$app, $instances]) =>
    $instances.find(i => i.id === $app.connectedInstanceId) || null
);
