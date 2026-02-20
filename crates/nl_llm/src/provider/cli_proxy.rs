//! CLI Proxy - 命令行通信模块
//!
//! 与本地 LLM CLI 交互（Claude、Gemini 等）。

use std::process::Command;

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
        let env = self.config.env.clone();
        let input = input.to_string();

        // 在阻塞任务中执行
        let output = tokio::task::spawn_blocking(move || {
            let mut cmd = Command::new(&command);
            cmd.args(&args);
            for (key, value) in &env {
                cmd.env(key, value);
            }

            // 如果有输入，通过 stdin 传递
            if !input.is_empty() {
                cmd.stdin(std::process::Stdio::piped());
            }

            let mut child = cmd.spawn()?;

            if !input.is_empty() {
                use std::io::Write;
                if let Some(mut stdin) = child.stdin.take() {
                    stdin.write_all(input.as_bytes())?;
                }
            }

            let output = child.wait_with_output()?;
            Ok::<_, std::io::Error>(String::from_utf8_lossy(&output.stdout).to_string())
        })
        .await
        .map_err(|e| crate::NeuroLoomError::LlmProvider(e.to_string()))?
        .map_err(|e| crate::NeuroLoomError::LlmProvider(e.to_string()))?;

        Ok(output)
    }

    /// 检查命令是否可用
    pub fn is_available(&self) -> bool {
        Command::new(&self.config.command)
            .arg("--version")
            .output()
            .is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_proxy_config() {
        let proxy = CliProxy::claude();
        assert_eq!(proxy.config.command, "claude");
        assert!(proxy.config.args.is_empty());
    }

    #[test]
    fn test_gemini_proxy_config() {
        let proxy = CliProxy::gemini();
        assert_eq!(proxy.config.command, "gemini");
    }
}
