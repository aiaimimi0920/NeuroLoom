//! HAP 服务器

use std::net::SocketAddr;

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Response,
    routing::get,
    Router,
};
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::protocol::{HapMessage, HapProtocol};

/// HAP 服务器配置
#[derive(Debug, Clone)]
pub struct HapServerConfig {
    /// 监听地址
    pub addr: SocketAddr,
    /// Agent ID
    pub agent_id: Uuid,
}

impl Default for HapServerConfig {
    fn default() -> Self {
        Self {
            addr: "0.0.0.0:8765".parse().unwrap(),
            agent_id: Uuid::new_v4(),
        }
    }
}

/// HAP 服务器
pub struct HapServer {
    config: HapServerConfig,
    /// 消息广播通道
    message_tx: broadcast::Sender<HapMessage>,
}

impl HapServer {
    /// 创建新服务器
    pub fn new(config: HapServerConfig) -> Self {
        let (message_tx, _) = broadcast::channel(1024);
        Self { config, message_tx }
    }

    /// 创建默认服务器
    pub fn default_server() -> Self {
        Self::new(HapServerConfig::default())
    }

    /// 构建 Axum 路由
    pub fn build_router(&self) -> Router {
        let message_tx = self.message_tx.clone();
        Router::new()
            .route("/ws", get(|ws: WebSocketUpgrade| async move {
                ws.on_upgrade(|socket| handle_socket(socket, message_tx))
            }))
    }

    /// 启动服务器
    pub async fn start(&self) -> nl_core::Result<()> {
        let app = self.build_router();
        let listener = tokio::net::TcpListener::bind(&self.config.addr)
            .await
            .map_err(|e| nl_core::NeuroLoomError::Protocol(e.to_string()))?;

        axum::serve(listener, app)
            .await
            .map_err(|e| nl_core::NeuroLoomError::Protocol(e.to_string()))?;

        Ok(())
    }

    /// 获取消息接收器
    pub fn subscribe(&self) -> broadcast::Receiver<HapMessage> {
        self.message_tx.subscribe()
    }

    /// 获取配置
    pub fn config(&self) -> &HapServerConfig {
        &self.config
    }
}

/// 处理 WebSocket 连接
async fn handle_socket(socket: WebSocket, message_tx: broadcast::Sender<HapMessage>) {
    let (mut sender, mut receiver) = socket.split();

    while let Some(msg) = receiver.recv().await {
        if let Ok(msg) = msg {
            if let Message::Text(text) = msg {
                if let Ok(hap_msg) = HapMessage::from_json(&text) {
                    let _ = message_tx.send(hap_msg);
                }
            }
        } else {
            break;
        }
    }
}
