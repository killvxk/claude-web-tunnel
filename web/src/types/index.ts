// 用户角色 - 与后端 snake_case 对应
export type Role = 'super_admin' | 'admin' | 'user';

// 认证状态
export interface AuthState {
  isAuthenticated: boolean;
  token: string | null;
  role: Role | null;
  agentName: string | null;
  agentId: string | null;
}

// 实例状态 - 与后端对应
export type InstanceStatus = 'running' | 'suspended' | 'stopped';

// Agent 实例信息 - 与后端 Instance 结构对应
export interface AgentInstance {
  id: string;
  agent_id: string;
  cwd: string;
  status: InstanceStatus;
  created_at: string;
  attached_users: number;
}

// WebSocket 消息类型 - 与后端 snake_case 对应
export type WsMessageType =
  // User -> Server
  | 'auth'
  | 'create_instance'
  | 'close_instance'
  | 'attach'
  | 'detach'
  | 'pty_input'
  | 'resize'
  | 'list_instances'
  | 'heartbeat'
  // Admin commands (SuperAdmin only)
  | 'get_admin_stats'
  | 'force_disconnect_agent'
  | 'force_close_instance'
  | 'delete_agent'
  // Tag commands (Admin only)
  | 'get_all_tags'
  | 'get_agent_tags'
  | 'add_agent_tag'
  | 'remove_agent_tag'
  // Audit log commands (SuperAdmin only)
  | 'get_audit_logs'
  // Working Agent commands (SuperAdmin only)
  | 'select_working_agent'
  | 'clear_working_agent'
  | 'list_agent_instances'
  // Server -> User
  | 'auth_result'
  | 'instance_list'
  | 'instance_created'
  | 'instance_closed'
  | 'pty_output'
  | 'user_joined'
  | 'user_left'
  | 'agent_status_changed'
  | 'error'
  | 'pong'
  // Admin responses (SuperAdmin only)
  | 'admin_stats'
  | 'agent_disconnected'
  | 'agent_deleted'
  // Tag responses
  | 'tag_list'
  | 'agent_tags'
  | 'tag_added'
  | 'tag_removed'
  // Audit log responses (SuperAdmin only)
  | 'audit_log_list'
  // Working Agent responses (SuperAdmin only)
  | 'working_agent_selected'
  | 'working_agent_cleared';

// 基础 WebSocket 消息
export interface WsMessage {
  type: WsMessageType;
  [key: string]: unknown;
}

// ============================================================================
// User -> Server 消息
// ============================================================================

// 认证消息
export interface AuthMessage extends WsMessage {
  type: 'auth';
  token: string;
}

// 创建实例请求 (admin only)
export interface CreateInstanceMessage extends WsMessage {
  type: 'create_instance';
  cwd: string;
}

// 关闭实例请求 (admin only)
export interface CloseInstanceMessage extends WsMessage {
  type: 'close_instance';
  instance_id: string;
}

// 连接到实例
export interface AttachMessage extends WsMessage {
  type: 'attach';
  instance_id: string;
}

// 断开实例连接
export interface DetachMessage extends WsMessage {
  type: 'detach';
  instance_id: string;
}

// 终端输入
export interface PtyInputMessage extends WsMessage {
  type: 'pty_input';
  instance_id: string;
  data: string;  // base64 encoded
}

// 终端调整大小
export interface ResizeMessage extends WsMessage {
  type: 'resize';
  instance_id: string;
  cols: number;
  rows: number;
}

// 请求实例列表
export interface ListInstancesMessage extends WsMessage {
  type: 'list_instances';
}

// 心跳
export interface HeartbeatMessage extends WsMessage {
  type: 'heartbeat';
}

// ============================================================================
// Server -> User 消息
// ============================================================================

// 认证响应
export interface AuthResultMessage extends WsMessage {
  type: 'auth_result';
  success: boolean;
  role?: Role;
  agent_name?: string;
  agent_id?: string;
  error?: string;
}

// 实例列表
export interface InstanceListMessage extends WsMessage {
  type: 'instance_list';
  instances: AgentInstance[];
}

// 实例创建通知
export interface InstanceCreatedMessage extends WsMessage {
  type: 'instance_created';
  instance: AgentInstance;
}

// 实例关闭通知
export interface InstanceClosedMessage extends WsMessage {
  type: 'instance_closed';
  instance_id: string;
}

// 终端输出
export interface PtyOutputMessage extends WsMessage {
  type: 'pty_output';
  instance_id: string;
  data: string;  // base64 encoded
}

// 用户加入通知
export interface UserJoinedMessage extends WsMessage {
  type: 'user_joined';
  instance_id: string;
  user_count: number;
}

// 用户离开通知
export interface UserLeftMessage extends WsMessage {
  type: 'user_left';
  instance_id: string;
  user_count: number;
}

// Agent 状态变更
export interface AgentStatusChangedMessage extends WsMessage {
  type: 'agent_status_changed';
  agent_id: string;
  online: boolean;
}

// 错误消息
export interface ErrorMessage extends WsMessage {
  type: 'error';
  message: string;
}

// 心跳响应
export interface PongMessage extends WsMessage {
  type: 'pong';
}

// ============================================================================
// Admin Messages (SuperAdmin only)
// ============================================================================

// Agent 状态
export type AgentStatus = 'online' | 'offline';

// Agent 信息 (管理面板用)
export interface AgentInfo {
  id: string;
  name: string;
  status: AgentStatus;
  connected_at: string | null;
  instance_count: number;
  user_count: number;
}

// 全局统计
export interface GlobalStats {
  total_agents: number;
  online_agents: number;
  total_instances: number;
  running_instances: number;
  total_users: number;
}

// 请求管理统计
export interface GetAdminStatsMessage extends WsMessage {
  type: 'get_admin_stats';
}

// 强制断开 Agent
export interface ForceDisconnectAgentMessage extends WsMessage {
  type: 'force_disconnect_agent';
  agent_id: string;
}

// 强制关闭实例
export interface ForceCloseInstanceMessage extends WsMessage {
  type: 'force_close_instance';
  instance_id: string;
}

// 删除 Agent
export interface DeleteAgentMessage extends WsMessage {
  type: 'delete_agent';
  agent_id: string;
}

// 管理统计响应
export interface AdminStatsMessage extends WsMessage {
  type: 'admin_stats';
  agents: AgentInfo[];
  stats: GlobalStats;
}

// Agent 断开通知
export interface AgentDisconnectedMessage extends WsMessage {
  type: 'agent_disconnected';
  agent_id: string;
}

// Agent 删除通知
export interface AgentDeletedMessage extends WsMessage {
  type: 'agent_deleted';
  agent_id: string;
}

// ============================================================================
// Tag Messages
// ============================================================================

// 获取所有标签请求
export interface GetAllTagsMessage extends WsMessage {
  type: 'get_all_tags';
}

// 获取 Agent 标签请求
export interface GetAgentTagsMessage extends WsMessage {
  type: 'get_agent_tags';
  agent_id: string;
}

// 添加 Agent 标签请求
export interface AddAgentTagMessage extends WsMessage {
  type: 'add_agent_tag';
  agent_id: string;
  tag: string;
}

// 移除 Agent 标签请求
export interface RemoveAgentTagMessage extends WsMessage {
  type: 'remove_agent_tag';
  agent_id: string;
  tag: string;
}

// 标签列表响应
export interface TagListMessage extends WsMessage {
  type: 'tag_list';
  tags: string[];
}

// Agent 标签响应
export interface AgentTagsMessage extends WsMessage {
  type: 'agent_tags';
  agent_id: string;
  tags: string[];
}

// 标签添加通知
export interface TagAddedMessage extends WsMessage {
  type: 'tag_added';
  agent_id: string;
  tag: string;
}

// 标签移除通知
export interface TagRemovedMessage extends WsMessage {
  type: 'tag_removed';
  agent_id: string;
  tag: string;
}

// ============================================================================
// Audit Log Messages (SuperAdmin only)
// ============================================================================

// 审计日志条目
export interface AuditLogEntry {
  id: number;
  timestamp: string;
  event_type: string;
  user_role: string;
  agent_id: string | null;
  instance_id: string | null;
  target_id: string | null;
  client_ip: string;
  success: boolean;
  details: string | null;
}

// 获取审计日志请求
export interface GetAuditLogsMessage extends WsMessage {
  type: 'get_audit_logs';
  limit?: number;
  offset?: number;
  event_type?: string;
}

// 审计日志列表响应
export interface AuditLogListMessage extends WsMessage {
  type: 'audit_log_list';
  logs: AuditLogEntry[];
  total: number;
}

// ============================================================================
// Working Agent Messages (SuperAdmin only)
// ============================================================================

// 选择工作 Agent 请求
export interface SelectWorkingAgentMessage extends WsMessage {
  type: 'select_working_agent';
  agent_id: string;
}

// 清除工作 Agent 请求
export interface ClearWorkingAgentMessage extends WsMessage {
  type: 'clear_working_agent';
}

// 列出指定 Agent 的实例请求
export interface ListAgentInstancesMessage extends WsMessage {
  type: 'list_agent_instances';
  agent_id: string;
}

// 工作 Agent 选择成功响应
export interface WorkingAgentSelectedMessage extends WsMessage {
  type: 'working_agent_selected';
  agent_id: string;
  agent_name: string;
  success: boolean;
  error?: string;
}

// 工作 Agent 清除通知
export interface WorkingAgentClearedMessage extends WsMessage {
  type: 'working_agent_cleared';
}

// ============================================================================
// 应用状态
// ============================================================================

export type AppView = 'login' | 'instances' | 'terminal' | 'admin';

export interface AppState {
  view: AppView;
  connectedInstanceId: string | null;
  instances: AgentInstance[];
  error: string | null;
}
