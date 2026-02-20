//! God Mode - 原生文件读写操作

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// God Mode 操作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GodModeAction {
    /// 读取文件
    ReadFile { path: PathBuf },
    /// 写入文件
    WriteFile { path: PathBuf, content: String },
    /// 删除文件
    DeleteFile { path: PathBuf },
    /// 创建目录
    CreateDir { path: PathBuf },
    /// 列出目录
    ListDir { path: PathBuf },
    /// 执行命令
    Execute { command: String, args: Vec<String> },
    /// 设置环境变量
    SetEnv { key: String, value: String },
    /// 获取环境变量
    GetEnv { key: String },
}

/// God Mode 操作结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GodModeResult {
    /// 是否成功
    pub success: bool,
    /// 输出内容
    pub output: String,
    /// 错误信息
    pub error: Option<String>,
}

/// God Mode 执行器
pub struct GodModeExecutor {
    /// 是否启用 (安全开关)
    enabled: bool,
}

impl GodModeExecutor {
    /// 创建新执行器
    pub fn new() -> Self {
        Self { enabled: true }
    }

    /// 禁用
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// 启用
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// 执行操作
    pub async fn execute(&self, action: GodModeAction) -> nl_core::Result<GodModeResult> {
        if !self.enabled {
            return Ok(GodModeResult {
                success: false,
                output: String::new(),
                error: Some("God Mode is disabled".to_string()),
            });
        }

        match action {
            GodModeAction::ReadFile { path } => self.read_file(&path).await,
            GodModeAction::WriteFile { path, content } => self.write_file(&path, &content).await,
            GodModeAction::DeleteFile { path } => self.delete_file(&path).await,
            GodModeAction::CreateDir { path } => self.create_dir(&path).await,
            GodModeAction::ListDir { path } => self.list_dir(&path).await,
            GodModeAction::Execute { command, args } => self.execute_command(&command, &args).await,
            GodModeAction::SetEnv { key, value } => self.set_env(&key, &value),
            GodModeAction::GetEnv { key } => self.get_env(&key),
        }
    }

    async fn read_file(&self, path: &Path) -> nl_core::Result<GodModeResult> {
        match tokio::fs::read_to_string(path).await {
            Ok(content) => Ok(GodModeResult {
                success: true,
                output: content,
                error: None,
            }),
            Err(e) => Ok(GodModeResult {
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
            }),
        }
    }

    async fn write_file(&self, path: &Path, content: &str) -> nl_core::Result<GodModeResult> {
        match tokio::fs::write(path, content).await {
            Ok(_) => Ok(GodModeResult {
                success: true,
                output: format!("Wrote to {}", path.display()),
                error: None,
            }),
            Err(e) => Ok(GodModeResult {
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
            }),
        }
    }

    async fn delete_file(&self, path: &Path) -> nl_core::Result<GodModeResult> {
        match tokio::fs::remove_file(path).await {
            Ok(_) => Ok(GodModeResult {
                success: true,
                output: format!("Deleted {}", path.display()),
                error: None,
            }),
            Err(e) => Ok(GodModeResult {
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
            }),
        }
    }

    async fn create_dir(&self, path: &Path) -> nl_core::Result<GodModeResult> {
        match tokio::fs::create_dir_all(path).await {
            Ok(_) => Ok(GodModeResult {
                success: true,
                output: format!("Created directory {}", path.display()),
                error: None,
            }),
            Err(e) => Ok(GodModeResult {
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
            }),
        }
    }

    async fn list_dir(&self, path: &Path) -> nl_core::Result<GodModeResult> {
        match tokio::fs::read_dir(path).await {
            Ok(mut entries) => {
                let mut files = Vec::new();
                while let Ok(Some(entry)) = entries.next_entry().await {
                    files.push(entry.file_name().to_string_lossy().to_string());
                }
                Ok(GodModeResult {
                    success: true,
                    output: files.join("\n"),
                    error: None,
                })
            }
            Err(e) => Ok(GodModeResult {
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
            }),
        }
    }

    async fn execute_command(&self, command: &str, args: &[String]) -> nl_core::Result<GodModeResult> {
        let output = tokio::process::Command::new(command)
            .args(args)
            .output()
            .await;

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                Ok(GodModeResult {
                    success: output.status.success(),
                    output: if output.status.success() { stdout } else { stderr },
                    error: if output.status.success() { None } else { Some(stderr) },
                })
            }
            Err(e) => Ok(GodModeResult {
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
            }),
        }
    }

    fn set_env(&self, key: &str, value: &str) -> nl_core::Result<GodModeResult> {
        std::env::set_var(key, value);
        Ok(GodModeResult {
            success: true,
            output: format!("Set {}={}", key, value),
            error: None,
        })
    }

    fn get_env(&self, key: &str) -> nl_core::Result<GodModeResult> {
        match std::env::var(key) {
            Ok(value) => Ok(GodModeResult {
                success: true,
                output: value,
                error: None,
            }),
            Err(e) => Ok(GodModeResult {
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
            }),
        }
    }
}

impl Default for GodModeExecutor {
    fn default() -> Self {
        Self::new()
    }
}
