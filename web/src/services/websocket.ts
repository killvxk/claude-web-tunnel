import { auth, app, wsStatus, admin } from '../stores';
import {
  setAllTags,
  setAgentTags,
  addTagToAgent,
  removeTagFromAgent,
} from '../stores/tags';
import type {
  WsMessage,
  AuthResultMessage,
  InstanceListMessage,
  InstanceCreatedMessage,
  InstanceClosedMessage,
  PtyOutputMessage,
  UserJoinedMessage,
  UserLeftMessage,
  AgentStatusChangedMessage,
  ErrorMessage,
  AdminStatsMessage,
  AgentDisconnectedMessage,
  AgentDeletedMessage,
  TagListMessage,
  AgentTagsMessage,
  TagAddedMessage,
  TagRemovedMessage,
  AuditLogListMessage,
  AuditLogEntry,
  WorkingAgentSelectedMessage,
  WorkingAgentClearedMessage,
} from '../types';

type MessageHandler = (data: string) => void;
type AuditLogHandler = (logs: AuditLogEntry[], total: number) => void;

// UTF-8 安全的 Base64 编码/解码
function utf8ToBase64(str: string): string {
  const encoder = new TextEncoder();
  const bytes = encoder.encode(str);
  let binary = '';
  for (let i = 0; i < bytes.length; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary);
}

function base64ToUtf8(base64: string): string {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  const decoder = new TextDecoder('utf-8');
  return decoder.decode(bytes);
}

class WebSocketService {
  private ws: WebSocket | null = null;
  private url: string = '';
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;
  private reconnectDelay = 1000;
  private heartbeatInterval: ReturnType<typeof setInterval> | null = null;
  private terminalOutputHandler: MessageHandler | null = null;
  private auditLogHandler: AuditLogHandler | null = null;
  private currentInstanceId: string | null = null;
  private attachTimeout: ReturnType<typeof setTimeout> | null = null;
  private receivedOutput = false;

  constructor() {
    // 默认 URL，可通过 connect 方法覆盖
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    this.url = `${protocol}//${window.location.host}/ws/user`;
  }

  connect(token: string, customUrl?: string): Promise<void> {
    return new Promise((resolve, reject) => {
      if (customUrl) {
        this.url = customUrl;
      }

      wsStatus.setStatus('connecting');

      try {
        this.ws = new WebSocket(this.url);
      } catch (error) {
        wsStatus.setStatus('error');
        reject(error);
        return;
      }

      this.ws.onopen = () => {
        console.log('WebSocket connected');
        wsStatus.setStatus('connected');
        this.reconnectAttempts = 0;

        // 发送认证消息 - 与后端 UserMessage::Auth 对应
        this.send({
          type: 'auth',
          token: token,
        });

        this.startHeartbeat();
        resolve();
      };

      this.ws.onmessage = (event) => {
        this.handleMessage(event.data);
      };

      this.ws.onclose = (event) => {
        console.log('WebSocket closed:', event.code, event.reason);
        this.stopHeartbeat();
        this.currentInstanceId = null;

        // 尝试重连
        if (this.reconnectAttempts < this.maxReconnectAttempts) {
          this.reconnectAttempts++;
          wsStatus.setReconnecting(this.reconnectAttempts, this.maxReconnectAttempts);
          console.log(`Reconnecting... (${this.reconnectAttempts}/${this.maxReconnectAttempts})`);
          setTimeout(() => {
            const savedToken = localStorage.getItem('auth_token');
            if (savedToken) {
              this.connect(savedToken);
            }
          }, this.reconnectDelay * this.reconnectAttempts);
        } else {
          wsStatus.setStatus('disconnected');
        }
      };

      this.ws.onerror = (error) => {
        console.error('WebSocket error:', error);
        wsStatus.setStatus('error');
        reject(error);
      };
    });
  }

  disconnect(): void {
    this.stopHeartbeat();
    if (this.ws) {
      this.ws.close(1000, 'User disconnected');
      this.ws = null;
    }
    this.currentInstanceId = null;
    wsStatus.setStatus('disconnected');
  }

  send(message: WsMessage): void {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(message));
    } else {
      console.warn('WebSocket is not connected');
    }
  }

  // 请求实例列表 - 与后端 UserMessage::ListInstances 对应
  requestInstances(): void {
    this.send({ type: 'list_instances' });
  }

  // 连接到实例 - 与后端 UserMessage::Attach 对应
  attachToInstance(instanceId: string): void {
    this.currentInstanceId = instanceId;
    this.receivedOutput = false;
    this.clearAttachTimeout();

    this.send({
      type: 'attach',
      instance_id: instanceId,
    });

    // 设置超时检测（5秒无 pty_output 视为失败）
    this.attachTimeout = setTimeout(() => {
      if (!this.receivedOutput) {
        app.setError('连接实例超时，请重试');
        this.detachFromInstance();
      }
    }, 5000);

    // 直接跳转到终端视图（后端 Attach 没有响应消息）
    app.connectToInstance(instanceId);
  }

  // 清除 attach 超时定时器
  private clearAttachTimeout(): void {
    if (this.attachTimeout) {
      clearTimeout(this.attachTimeout);
      this.attachTimeout = null;
    }
  }

  // 断开实例连接 - 与后端 UserMessage::Detach 对应
  detachFromInstance(): void {
    this.clearAttachTimeout();
    if (this.currentInstanceId) {
      this.send({
        type: 'detach',
        instance_id: this.currentInstanceId,
      });
      this.currentInstanceId = null;
    }
    this.receivedOutput = false;
    app.disconnectFromInstance();
  }

  // 发送终端输入 - 与后端 UserMessage::PtyInput 对应
  sendTerminalInput(data: string): void {
    if (!this.currentInstanceId) {
      console.warn('No instance connected');
      return;
    }
    // 使用 UTF-8 安全的 base64 编码
    const base64Data = utf8ToBase64(data);
    this.send({
      type: 'pty_input',
      instance_id: this.currentInstanceId,
      data: base64Data,
    });
  }

  // 发送终端调整大小 - 与后端 UserMessage::Resize 对应
  sendTerminalResize(cols: number, rows: number): void {
    if (!this.currentInstanceId) {
      return;
    }
    this.send({
      type: 'resize',
      instance_id: this.currentInstanceId,
      cols: cols,
      rows: rows,
    });
  }

  // 创建实例 (admin only) - 与后端 UserMessage::CreateInstance 对应
  createInstance(cwd: string): void {
    this.send({
      type: 'create_instance',
      cwd: cwd,
    });
  }

  // 关闭实例 (admin only) - 与后端 UserMessage::CloseInstance 对应
  closeInstance(instanceId: string): void {
    this.send({
      type: 'close_instance',
      instance_id: instanceId,
    });
  }

  // ========================================================================
  // Admin methods (SuperAdmin only)
  // ========================================================================

  // 请求管理统计 - 与后端 UserMessage::GetAdminStats 对应
  requestAdminStats(): void {
    admin.setLoading(true);
    this.send({ type: 'get_admin_stats' });
  }

  // 强制断开 Agent - 与后端 UserMessage::ForceDisconnectAgent 对应
  forceDisconnectAgent(agentId: string): void {
    this.send({
      type: 'force_disconnect_agent',
      agent_id: agentId,
    });
  }

  // 强制关闭实例 - 与后端 UserMessage::ForceCloseInstance 对应
  forceCloseInstance(instanceId: string): void {
    this.send({
      type: 'force_close_instance',
      instance_id: instanceId,
    });
  }

  // 删除 Agent - 与后端 UserMessage::DeleteAgent 对应
  deleteAgent(agentId: string): void {
    this.send({
      type: 'delete_agent',
      agent_id: agentId,
    });
  }

  // ========================================================================
  // Tag methods (Admin only)
  // ========================================================================

  // 获取所有标签 - 与后端 UserMessage::GetAllTags 对应
  requestAllTags(): void {
    this.send({ type: 'get_all_tags' });
  }

  // 获取指定 Agent 的标签 - 与后端 UserMessage::GetAgentTags 对应
  requestAgentTags(agentId: string): void {
    this.send({
      type: 'get_agent_tags',
      agent_id: agentId,
    });
  }

  // 添加标签到 Agent - 与后端 UserMessage::AddAgentTag 对应
  addAgentTag(agentId: string, tag: string): void {
    this.send({
      type: 'add_agent_tag',
      agent_id: agentId,
      tag: tag,
    });
  }

  // 从 Agent 移除标签 - 与后端 UserMessage::RemoveAgentTag 对应
  removeAgentTag(agentId: string, tag: string): void {
    this.send({
      type: 'remove_agent_tag',
      agent_id: agentId,
      tag: tag,
    });
  }

  // ========================================================================
  // Audit log methods (SuperAdmin only)
  // ========================================================================

  // 请求审计日志 - 与后端 UserMessage::GetAuditLogs 对应
  requestAuditLogs(limit?: number, offset?: number, eventType?: string): void {
    this.send({
      type: 'get_audit_logs',
      limit: limit,
      offset: offset,
      event_type: eventType,
    });
  }

  // ========================================================================
  // Working Agent methods (SuperAdmin only)
  // ========================================================================

  // 选择工作 Agent - 与后端 UserMessage::SelectWorkingAgent 对应
  selectWorkingAgent(agentId: string): void {
    this.send({
      type: 'select_working_agent',
      agent_id: agentId,
    });
  }

  // 清除工作 Agent - 与后端 UserMessage::ClearWorkingAgent 对应
  clearWorkingAgent(): void {
    this.send({ type: 'clear_working_agent' });
  }

  // 列出指定 Agent 的实例 - 与后端 UserMessage::ListAgentInstances 对应
  listAgentInstances(agentId: string): void {
    this.send({
      type: 'list_agent_instances',
      agent_id: agentId,
    });
  }

  // 设置审计日志处理器
  setAuditLogHandler(handler: AuditLogHandler | null): void {
    this.auditLogHandler = handler;
  }

  // 设置终端输出处理器
  setTerminalOutputHandler(handler: MessageHandler | null): void {
    this.terminalOutputHandler = handler;
  }

  // 获取当前连接的实例 ID
  getCurrentInstanceId(): string | null {
    return this.currentInstanceId;
  }

  private handleMessage(data: string): void {
    try {
      const message = JSON.parse(data) as WsMessage;

      switch (message.type) {
        case 'auth_result':
          this.handleAuthResult(message as AuthResultMessage);
          break;
        case 'instance_list':
          this.handleInstanceList(message as InstanceListMessage);
          break;
        case 'instance_created':
          this.handleInstanceCreated(message as InstanceCreatedMessage);
          break;
        case 'instance_closed':
          this.handleInstanceClosed(message as InstanceClosedMessage);
          break;
        case 'pty_output':
          this.handlePtyOutput(message as PtyOutputMessage);
          break;
        case 'user_joined':
          this.handleUserJoined(message as UserJoinedMessage);
          break;
        case 'user_left':
          this.handleUserLeft(message as UserLeftMessage);
          break;
        case 'agent_status_changed':
          this.handleAgentStatusChanged(message as AgentStatusChangedMessage);
          break;
        case 'error':
          this.handleError(message as ErrorMessage);
          break;
        case 'pong':
          // 心跳确认，不需要处理
          break;
        // Admin responses
        case 'admin_stats':
          this.handleAdminStats(message as AdminStatsMessage);
          break;
        case 'agent_disconnected':
          this.handleAgentDisconnected(message as AgentDisconnectedMessage);
          break;
        case 'agent_deleted':
          this.handleAgentDeleted(message as AgentDeletedMessage);
          break;
        // Tag responses
        case 'tag_list':
          this.handleTagList(message as TagListMessage);
          break;
        case 'agent_tags':
          this.handleAgentTags(message as AgentTagsMessage);
          break;
        case 'tag_added':
          this.handleTagAdded(message as TagAddedMessage);
          break;
        case 'tag_removed':
          this.handleTagRemoved(message as TagRemovedMessage);
          break;
        // Audit log responses
        case 'audit_log_list':
          this.handleAuditLogList(message as AuditLogListMessage);
          break;
        // Working Agent responses
        case 'working_agent_selected':
          this.handleWorkingAgentSelected(message as WorkingAgentSelectedMessage);
          break;
        case 'working_agent_cleared':
          this.handleWorkingAgentCleared();
          break;
        default:
          console.warn('Unknown message type:', message.type);
      }
    } catch (error) {
      console.error('Failed to parse message:', error);
    }
  }

  private handleAuthResult(message: AuthResultMessage): void {
    const { success, role, agent_name, agent_id, error } = message;

    if (success && role) {
      const token = localStorage.getItem('auth_token');
      if (token) {
        auth.login(token, role, agent_name || null, agent_id || null);
        // SuperAdmin goes to admin panel, others go to instances
        if (role === 'super_admin') {
          app.setView('admin');
        } else {
          app.setView('instances');
        }
        // 服务器认证成功后会自动发送实例列表，不需要再请求
      }
    } else {
      app.setError(error || 'Authentication failed');
      auth.logout();
    }
  }

  private handleInstanceList(message: InstanceListMessage): void {
    app.setInstances(message.instances);
  }

  private handleInstanceCreated(message: InstanceCreatedMessage): void {
    app.addInstance(message.instance);
  }

  private handleInstanceClosed(message: InstanceClosedMessage): void {
    app.removeInstance(message.instance_id);
    // 如果当前连接的实例被关闭，断开连接
    if (this.currentInstanceId === message.instance_id) {
      this.currentInstanceId = null;
      app.disconnectFromInstance();
      app.setError('Instance was closed');
    }
  }

  private handlePtyOutput(message: PtyOutputMessage): void {
    if (this.terminalOutputHandler && message.instance_id === this.currentInstanceId) {
      // 标记已收到输出，清除超时
      if (!this.receivedOutput) {
        this.receivedOutput = true;
        this.clearAttachTimeout();
      }

      // 使用 UTF-8 安全的 base64 解码
      try {
        const decodedData = base64ToUtf8(message.data);
        this.terminalOutputHandler(decodedData);
      } catch {
        // 如果解码失败，尝试直接使用原始数据
        console.warn('Base64 decode failed, using raw data');
        this.terminalOutputHandler(message.data);
      }
    }
  }

  private handleUserJoined(message: UserJoinedMessage): void {
    app.updateInstanceUsers(message.instance_id, message.user_count);
  }

  private handleUserLeft(message: UserLeftMessage): void {
    app.updateInstanceUsers(message.instance_id, message.user_count);
  }

  private handleError(message: ErrorMessage): void {
    app.setError(message.message);
  }

  private handleAgentStatusChanged(message: AgentStatusChangedMessage): void {
    const { agent_id, online } = message;
    console.log(`Agent ${agent_id} status changed: ${online ? 'online' : 'offline'}`);

    // 更新 admin store 中的 agent 状态
    admin.updateAgentStatus(agent_id, online);

    if (!online) {
      // Agent 离线时，将其所有实例状态更新为 suspended
      app.updateInstancesStatusByAgent(agent_id, 'suspended');

      // 如果当前连接的实例属于该 agent，通知用户
      if (this.currentInstanceId) {
        app.setError('Agent disconnected - terminal may be unavailable');
      }
    } else {
      // Agent 上线时，将其所有实例状态更新为 running
      app.updateInstancesStatusByAgent(agent_id, 'running');
    }
  }

  // ========================================================================
  // Admin message handlers
  // ========================================================================

  private handleAdminStats(message: AdminStatsMessage): void {
    admin.setStats(message.agents, message.stats);
  }

  private handleAgentDisconnected(message: AgentDisconnectedMessage): void {
    console.log(`Agent ${message.agent_id} was force disconnected`);
    admin.updateAgentStatus(message.agent_id, false);
  }

  private handleAgentDeleted(message: AgentDeletedMessage): void {
    console.log(`Agent ${message.agent_id} was deleted`);
    admin.removeAgent(message.agent_id);
  }

  // ========================================================================
  // Tag message handlers
  // ========================================================================

  private handleTagList(message: TagListMessage): void {
    setAllTags(message.tags);
  }

  private handleAgentTags(message: AgentTagsMessage): void {
    setAgentTags(message.agent_id, message.tags);
  }

  private handleTagAdded(message: TagAddedMessage): void {
    addTagToAgent(message.agent_id, message.tag);
  }

  private handleTagRemoved(message: TagRemovedMessage): void {
    removeTagFromAgent(message.agent_id, message.tag);
  }

  // ========================================================================
  // Audit log message handlers
  // ========================================================================

  private handleAuditLogList(message: AuditLogListMessage): void {
    if (this.auditLogHandler) {
      this.auditLogHandler(message.logs, message.total);
    }
  }

  // ========================================================================
  // Working Agent message handlers
  // ========================================================================

  private handleWorkingAgentSelected(message: WorkingAgentSelectedMessage): void {
    const { agent_id, agent_name, success, error } = message;
    console.log(`Working agent selected: ${agent_name} (${agent_id}), success: ${success}`);

    if (success) {
      admin.setWorkingAgent(agent_id, agent_name);
      // Switch to instances view to show the agent's instances
      app.setView('instances');
    } else {
      app.setError(error || 'Failed to select working agent');
    }
  }

  private handleWorkingAgentCleared(): void {
    console.log('Working agent cleared');
    admin.clearWorkingAgent();
    // Clear instances and switch back to admin panel
    app.setInstances([]);
    app.setView('admin');
  }

  private startHeartbeat(): void {
    this.heartbeatInterval = setInterval(() => {
      this.send({ type: 'heartbeat' });
    }, 30000); // 每30秒发送一次心跳
  }

  private stopHeartbeat(): void {
    if (this.heartbeatInterval) {
      clearInterval(this.heartbeatInterval);
      this.heartbeatInterval = null;
    }
  }
}

// 单例导出
export const wsService = new WebSocketService();
