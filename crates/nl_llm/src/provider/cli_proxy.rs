//! CLI Proxy - PTY 通信模块
//!
//! 劫持本地 Claude/Gemini 命令行的 PTY 通信。

use std::process::Command;

use anyhow::Result;
use portable_pty::{native_pty_system, PtySize, CommandBuilder};
use tokio::sync::mpsc;

/// CLI Provider 配置
#[derive(Debug, Clone)]
pub struct CliProxyConfig {
    /// 命令路径
    pub command: String,
    /// 参数
    pub args: Vec<String>,
    /// 环境变量
    pub env: Vec<(String, String)>,
}

/// CLI Proxy - 与本地 LLM CLI 交互
pub struct CliProxy {
    config: CliProxyConfig,
}

impl CliProxy {
    /// 创建新的 CLI Proxy
    pub fn new(config: CliProxyConfig) -> Self {
        Self { config }
    }

    /// 创建 Claude CLI Proxy
    pub fn claude() -> Self {
        Self::new(CliProxyConfig {
            command: "claude".to_string(),
            args: vec![],
            env: vec![],
        })
    }

    /// 创建 Gemini CLI Proxy
    pub fn gemini() -> Self {
        Self::new(CliProxyConfig {
            command: "gemini".to_string(),
            args: vec![],
            env: vec![],
        })
    }

    /// 执行命令并获取输出
    pub async fn execute(&self, input: &str) -> crate::Result<String> {
        let command = self.config.command.clone();
        let args = self.config.args.clone();
        let input = input.to_string();

        // 在阻塞任务中执行
        let output = tokio::task::spawn_blocking(move || {
            let mut cmd = Command::new(&command);
            cmd.args(&args);

            let output = cmd.output()?;
            Ok::<_, std::io::Error>(String::from_utf8_lossy(&output.stdout).to_string())
        })
        .await
        .map_err(|e| crate::NeuroLoomError::LlmProvider(e.to_string()))?
        .map_err(|e| crate::NeuroLoomError::LlmProvider(e.to_string()))?;

        Ok(output)
    }

    /// 启动交互式 PTY 会话
    pub fn start_pty(&self) -> crate::Result<PtySession> {
        let pty_system = native_pty_system();

        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| crate::NeuroLoomError::LlmProvider(e.to_string()))?;

        let mut cmd = CommandBuilder::new(&self.config.command);
        cmd.args(&self.config.args);

        for (key, value) in &self.config.env {
            cmd.env(key, value);
        }

        let _child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| crate::NeuroLoomError::LlmProvider(e.to_string()))?;

        Ok(PtySession {
            _master: pair.master,
        })
    }
}

/// PTY 会话
pub struct PtySession {
    _master: Box<dyn portable_pty::PtyMaster>,
}

// PtySession 可以扩展以支持交互式通信
