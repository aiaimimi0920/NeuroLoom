//! HAP 客户端

use std::net::SocketAddr;

use tokio::sync::mpsc;
use uuid::Uuid;

use crate::protocol::{HapMessage, HapMessageType};

/// HAP 客户端配置
#[derive(Debug, Clone)]
pub struct HapClientConfig {
    /// 服务器地址
    pub server_addr: SocketAddr,
    /// 本地 Agent ID
    pub agent_id: Uuid,
    /// 心跳间隔 (秒)
    pub heartbeat_interval: u64,
}

impl Default for HapClientConfig {
    fn default() -> Self {
        Self {
            server_addr: "127.0.0.1:8765".parse().unwrap(),
            agent_id: Uuid::new_v4(),
            heartbeat_interval: 30,
        }
    }
}

/// HAP 客户端
pub struct HapClient {
    config: HapClientConfig,
    /// 消息发送通道
    message_tx: mpsc::Sender<HapMessage>,
    /// 是否已连接
    connected: bool,
}

impl HapClient {
    /// 创建新客户端
    pub fn new(config: HapClientConfig) -> Self {
        let (message_tx, _) = mpsc::channel(1024);
        Self {
            config,
            message_tx,
            connected: false,
        }
    }

    /// 创建默认客户端
    pub fn default_client() -> Self {
        Self::new(HapClientConfig::default())
    }

    /// 连接到服务器
    pub async fn connect(&mut self) -> nl_core::Result<()> {
        // TODO: 实现实际的 WebSocket 连接
        self.connected = true;
        Ok(())
    }

    /// 断开连接
    pub async fn disconnect(&mut self) {
        self.connected = false;
    }

    /// 发送消息
    pub async fn send(&self, msg: HapMessage) -> nl_core::Result<()> {
        if !self.connected {
            return Err(nl_core::NeuroLoomError::Protocol("Not connected".to_string()));
        }
        self.message_tx
            .send(msg)
            .await
            .map_err(|e| nl_core::NeuroLoomError::Protocol(e.to_string()))
    }

    /// 广播任务
    pub async fn broadcast_task(&self, task: &str, requirements: Vec<String>) -> nl_core::Result<()> {
        let msg = HapProtocol::task_broadcast(self.config.agent_id, task, requirements);
        self.send(msg).await
    }

    /// 提交竞标
    pub async fn submit_bid(&self, task_id: Uuid, price: f64, eta_secs: u64) -> nl_core::Result<()> {
        let msg = HapProtocol::bid(self.config.agent_id, task_id, price, eta_secs);
        self.send(msg).await
    }

    /// 提交任务结果
    pub async fn submit_result(&self, task_id: Uuid, result: &str) -> nl_core::Result<()> {
        let msg = HapProtocol::task_result(self.config.agent_id, task_id, result);
        self.send(msg).await
    }

    /// 是否已连接
    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// 获取配置
    pub fn config(&self) -> &HapClientConfig {
        &self.config
    }
}
