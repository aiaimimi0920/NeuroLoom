//! iFlow Provider (Cookie Auth)
//!
//! 基于 Cookie 获取和刷新 iFlow API Key。
//! 参考: CLIProxyAPI/internal/auth/iflow/iflow_auth.go

use std::path::Path;
use std::time::Duration;

use chrono::{DateTime, Utc};
use regex::Regex;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};

use crate::prompt_ast::PromptAst;

const IFLOW_API_KEY_ENDPOINT: &str = "https://platform.iflow.cn/api/openapi/apikey";
const IFLOW_CHAT_ENDPOINT: &str = "https://apis.iflow.cn/v1/chat/completions";
const IFLOW_DEFAULT_MODEL: &str = "qwen3-max";

/// iFlow 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IFlowConfig {
    /// 用户的 Cookie (必须包含 BXAuth 字段)
    pub cookie: String,
    /// 模型名称
    pub model: String,
    /// 缓存的 API Key
    pub api_key: Option<String>,
    /// API Key 过期时间 (格式: "2006-01-02 15:04")
    pub expire_time: Option<String>,
    /// 用户邮箱/手机号
    pub email: Option<String>,
}

impl Default for IFlowConfig {
    fn default() -> Self {
        Self {
            cookie: String::new(),
            model: IFLOW_DEFAULT_MODEL.to_string(),
            api_key: None,
            expire_time: None,
            email: None,
        }
    }
}

/// iFlow Token 持久化存储结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IFlowTokenStorage {
    /// API Key
    #[serde(rename = "api_key")]
    pub api_key: String,
    /// 用户邮箱/手机号
    #[serde(rename = "email")]
    pub email: String,
    /// 过期时间
    #[serde(rename = "expire")]
    pub expire_time: String,
    /// BXAuth Cookie (仅保存 BXAuth 字段)
    #[serde(rename = "cookie")]
    pub cookie: String,
    /// 上次刷新时间
    #[serde(rename = "last_refresh")]
    pub last_refresh: String,
    /// 存储类型标识
    #[serde(rename = "type")]
    pub storage_type: String,
}

impl IFlowTokenStorage {
    /// 保存到文件
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> crate::Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                crate::NeuroLoomError::LlmProvider(format!(
                    "iflow token: create directory failed: {e}"
                ))
            })?;
        }

        let content = serde_json::to_string_pretty(self).map_err(|e| {
            crate::NeuroLoomError::LlmProvider(format!("iflow token: serialize failed: {e}"))
        })?;

        std::fs::write(path, content).map_err(|e| {
            crate::NeuroLoomError::LlmProvider(format!("iflow token: write file failed: {e}"))
        })?;

        Ok(())
    }

    /// 从文件加载
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> crate::Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            crate::NeuroLoomError::LlmProvider(format!("iflow token: read file failed: {e}"))
        })?;

        let storage: Self = serde_json::from_str(&content).map_err(|e| {
            crate::NeuroLoomError::LlmProvider(format!("iflow token: deserialize failed: {e}"))
        })?;

        Ok(storage)
    }
}

/// API Key 响应结构
#[derive(Debug, Clone, Deserialize)]
struct IFlowApiKeyResponse {
    #[serde(default)]
    success: bool,
    #[serde(default)]
    #[allow(dead_code)]
    code: Option<String>,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    exception: Option<serde_json::Value>,
    #[serde(default)]
    data: Option<IFlowKeyData>,
    #[serde(default)]
    #[allow(dead_code)]
    extra: Option<serde_json::Value>,
}

/// API Key 数据
#[derive(Debug, Clone, Deserialize)]
struct IFlowKeyData {
    /// 是否已过期
    #[serde(rename = "hasExpired", default)]
    has_expired: bool,
    /// 过期时间 (格式: "2006-01-02 15:04")
    #[serde(rename = "expireTime")]
    expire_time: String,
    /// 用户名称/邮箱
    #[serde(rename = "name")]
    name: String,
    /// API Key (完整，POST 刷新后才有)
    #[serde(rename = "apiKey", default)]
    api_key: String,
    /// API Key (掩码，GET 请求返回)
    #[serde(rename = "apiKeyMask")]
    api_key_mask: Option<String>,
}

/// 刷新请求体
#[derive(Debug, Serialize)]
struct IFlowRefreshRequest {
    name: String,
}

/// 刷新结果
#[derive(Debug, Clone)]
pub struct IFlowRefreshResult {
    pub api_key: String,
    pub expire_time: String,
    pub email: String,
    pub needs_refresh: bool,
    pub time_until_expiry: Duration,
}

/// iFlow Provider
pub struct IFlowProvider {
    config: IFlowConfig,
    client: reqwest::Client,
}

impl IFlowProvider {
    pub fn new(config: IFlowConfig) -> Self {
        // reqwest 默认启用 gzip 解压
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();
        Self { config, client }
    }

    /// 从持久化存储创建 Provider
    pub fn from_storage(storage: &IFlowTokenStorage) -> Self {
        let config = IFlowConfig {
            cookie: storage.cookie.clone(),
            model: IFLOW_DEFAULT_MODEL.to_string(),
            api_key: Some(storage.api_key.clone()),
            expire_time: Some(storage.expire_time.clone()),
            email: Some(storage.email.clone()),
        };
        Self::new(config)
    }

    /// 检查 API Key 是否需要刷新（2天内过期）
    pub fn should_refresh_api_key(&self) -> crate::Result<(bool, Duration)> {
        let expire_time = self.config.expire_time.as_ref().ok_or_else(|| {
            crate::NeuroLoomError::LlmProvider("iflow cookie: expire time is empty".to_string())
        })?;

        should_refresh_api_key(expire_time)
    }

    /// 使用 Cookie 换取/刷新 API Key
    pub async fn refresh_api_key(&mut self) -> crate::Result<IFlowRefreshResult> {
        if self.config.cookie.trim().is_empty() {
            return Err(crate::NeuroLoomError::LlmProvider(
                "iflow cookie is empty".to_string(),
            ));
        }

        // Step 1: GET 获取基础信息 (主要是 name)
        let key_info = self.fetch_api_key_info().await?;

        // 检查是否已过期
        if key_info.has_expired {
            return Err(crate::NeuroLoomError::LlmProvider(
                "iflow api key has expired, please re-authenticate".to_string(),
            ));
        }

        // Step 2: POST 刷新并获取完整 API Key
        let refreshed_key = self.refresh_api_key_internal(&key_info.name).await?;

        // 更新配置
        self.config.api_key = Some(refreshed_key.api_key.clone());
        self.config.expire_time = Some(refreshed_key.expire_time.clone());
        self.config.email = Some(refreshed_key.name.clone());

        // 检查是否需要下次刷新
        let (needs_refresh, time_until_expiry) =
            should_refresh_api_key(&refreshed_key.expire_time)?;

        Ok(IFlowRefreshResult {
            api_key: refreshed_key.api_key,
            expire_time: refreshed_key.expire_time,
            email: refreshed_key.name,
            needs_refresh,
            time_until_expiry,
        })
    }

    /// 转换为持久化存储
    pub fn to_storage(&self) -> Option<IFlowTokenStorage> {
        let api_key = self.config.api_key.as_ref()?;
        let email = self.config.email.as_ref()?;
        let expire_time = self.config.expire_time.as_ref()?;

        // 只保存 BXAuth 字段
        let bx_auth = extract_bx_auth(&self.config.cookie);
        let cookie_to_save = if !bx_auth.is_empty() {
            format!("BXAuth={bx_auth};")
        } else {
            String::new()
        };

        Some(IFlowTokenStorage {
            api_key: api_key.clone(),
            email: email.clone(),
            expire_time: expire_time.clone(),
            cookie: cookie_to_save,
            last_refresh: Utc::now().to_rfc3339(),
            storage_type: "iflow".to_string(),
        })
    }

    /// 获取当前 API Key
    pub fn get_api_key(&self) -> Option<&str> {
        self.config.api_key.as_deref()
    }

    /// 构建完整的浏览器请求头
    fn build_browser_headers(&self, include_content_type: bool) -> crate::Result<HeaderMap> {
        let mut headers = HeaderMap::new();

        // Cookie
        headers.insert(
            "Cookie",
            HeaderValue::from_str(&self.config.cookie).map_err(|e| {
                crate::NeuroLoomError::LlmProvider(format!("invalid cookie: {e}"))
            })?,
        );

        // 标准浏览器头
        headers.insert(
            "Accept",
            HeaderValue::from_static("application/json, text/plain, */*"),
        );
        headers.insert(
            "User-Agent",
            HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36",
            ),
        );
        headers.insert(
            "Accept-Language",
            HeaderValue::from_static("zh-CN,zh;q=0.9,en;q=0.8"),
        );
        // 注意：不要手动设置 Accept-Encoding，让 reqwest 自动处理 gzip 解压
        headers.insert("Connection", HeaderValue::from_static("keep-alive"));

        // Sec-Fetch headers (防风控)
        headers.insert("Sec-Fetch-Dest", HeaderValue::from_static("empty"));
        headers.insert("Sec-Fetch-Mode", HeaderValue::from_static("cors"));
        headers.insert("Sec-Fetch-Site", HeaderValue::from_static("same-origin"));

        if include_content_type {
            headers.insert(
                "Content-Type",
                HeaderValue::from_static("application/json"),
            );
        }

        Ok(headers)
    }

    /// GET 请求获取 API Key 信息
    async fn fetch_api_key_info(&self) -> crate::Result<IFlowKeyData> {
        let headers = self.build_browser_headers(false)?;

        let resp = self
            .client
            .get(IFLOW_API_KEY_ENDPOINT)
            .headers(headers)
            .send()
            .await
            .map_err(|e| {
                crate::NeuroLoomError::LlmProvider(format!(
                    "iflow cookie: GET request failed: {e}"
                ))
            })?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(crate::NeuroLoomError::LlmProvider(format!(
                "iflow cookie: GET request failed with status {}: {}",
                status, text.trim()
            )));
        }

        // 先获取原始文本用于调试
        let text = resp.text().await.map_err(|e| {
            crate::NeuroLoomError::LlmProvider(format!(
                "iflow cookie: read response text failed: {e}"
            ))
        })?;

        let key_resp: IFlowApiKeyResponse = serde_json::from_str(&text).map_err(|e| {
            crate::NeuroLoomError::LlmProvider(format!(
                "iflow cookie: decode GET response failed: {} (raw: {})",
                e, &text[..text.len().min(200)]
            ))
        })?;

        if !key_resp.success {
            return Err(crate::NeuroLoomError::LlmProvider(format!(
                "iflow cookie: GET request not successful: {}",
                key_resp.message.unwrap_or_default()
            )));
        }

        // 获取 data 字段
        let mut data = key_resp.data.ok_or_else(|| {
            crate::NeuroLoomError::LlmProvider(
                "iflow cookie: response missing data field".to_string(),
            )
        })?;

        // 处理初始响应中 apiKey 可能为空的情况
        if data.api_key.is_empty() {
            if let Some(mask) = &data.api_key_mask {
                data.api_key = mask.clone();
            }
        }

        Ok(data)
    }

    /// POST 请求刷新 API Key
    async fn refresh_api_key_internal(&self, name: &str) -> crate::Result<IFlowKeyData> {
        if name.trim().is_empty() {
            return Err(crate::NeuroLoomError::LlmProvider(
                "iflow cookie refresh: name is empty".to_string(),
            ));
        }

        let mut headers = self.build_browser_headers(true)?;

        // POST 需要额外的 Origin 和 Referer
        headers.insert(
            "Origin",
            HeaderValue::from_static("https://platform.iflow.cn"),
        );
        headers.insert(
            "Referer",
            HeaderValue::from_static("https://platform.iflow.cn/"),
        );

        let body = IFlowRefreshRequest {
            name: name.to_string(),
        };

        let resp = self
            .client
            .post(IFLOW_API_KEY_ENDPOINT)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                crate::NeuroLoomError::LlmProvider(format!(
                    "iflow cookie refresh: POST request failed: {e}"
                ))
            })?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(crate::NeuroLoomError::LlmProvider(format!(
                "iflow cookie refresh: POST request failed with status {}: {}",
                status,
                text.trim()
            )));
        }

        let key_resp: IFlowApiKeyResponse = resp.json().await.map_err(|e| {
            crate::NeuroLoomError::LlmProvider(format!(
                "iflow cookie refresh: decode POST response failed: {e}"
            ))
        })?;

        if !key_resp.success {
            return Err(crate::NeuroLoomError::LlmProvider(format!(
                "iflow cookie refresh: POST request not successful: {}",
                key_resp.message.unwrap_or_default()
            )));
        }

        key_resp.data.ok_or_else(|| {
            crate::NeuroLoomError::LlmProvider(
                "iflow cookie refresh: response missing data field".to_string(),
            )
        })
    }

    /// 将 Prompt AST 编译为 OpenAI 兼容的请求体 (iFlow 兼容 OpenAI 格式)
    pub fn compile_request(&self, ast: &PromptAst) -> serde_json::Value {
        let mut body = serde_json::json!({
            "model": self.config.model,
            "messages": ast.to_openai_messages(),
            "stream": false
        });

        // 注入 Thinking (Reasoning) 参数
        self.apply_thinking_params(&mut body);

        body
    }

    /// 根据模型名称注入 iFlow 特有的 Thinking 参数
    fn apply_thinking_params(&self, body: &mut serde_json::Value) {
        let model = self.config.model.to_lowercase();

        if self.is_enable_thinking_model(&model) {
            // GLM / DeepSeek 等使用 chat_template_kwargs.enable_thinking
            if let Some(obj) = body.as_object_mut() {
                let kwargs = obj.entry("chat_template_kwargs").or_insert(serde_json::json!({}));
                if let Some(kwargs_obj) = kwargs.as_object_mut() {
                    kwargs_obj.insert("enable_thinking".to_string(), serde_json::Value::Bool(true));
                    
                    // GLM 模型需要 clear_thinking=false
                    if model.starts_with("glm") {
                         kwargs_obj.insert("clear_thinking".to_string(), serde_json::Value::Bool(false));
                    }
                }
            }
        } else if model.starts_with("minimax") {
            // MiniMax 使用 reasoning_split
            if let Some(obj) = body.as_object_mut() {
                 obj.insert("reasoning_split".to_string(), serde_json::Value::Bool(true));
            }
        }
    }

    fn is_enable_thinking_model(&self, model: &str) -> bool {
        if model.starts_with("glm") {
            return true;
        }
        matches!(
            model,
            "qwen3-max-preview" | "deepseek-v3.2" | "deepseek-v3.1" | "deepseek-r1"
        )
    }

    /// 执行聊天补全
    pub async fn complete(&mut self, ast: &PromptAst) -> crate::Result<String> {
        // 检查是否需要刷新 API Key
        if let Ok((needs_refresh, _)) = self.should_refresh_api_key() {
            if needs_refresh {
                self.refresh_api_key().await?;
            }
        }

        // 确保有 API Key
        let api_key = if let Some(k) = &self.config.api_key {
            k.clone()
        } else {
            self.refresh_api_key().await?.api_key
        };

        let body = self.compile_request(ast);

        let resp = self
            .client
            .post(IFLOW_CHAT_ENDPOINT)
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                crate::NeuroLoomError::LlmProvider(format!("iflow chat: request failed: {e}"))
            })?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(crate::NeuroLoomError::LlmProvider(format!(
                "iflow chat failed: {} - {}",
                status,
                text.trim()
            )));
        }

        let json: serde_json::Value = resp.json().await.map_err(|e| {
            crate::NeuroLoomError::LlmProvider(format!("iflow chat: decode response failed: {e}"))
        })?;

        // 提取 content
        json["choices"][0]["message"]["content"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| {
                crate::NeuroLoomError::LlmProvider(
                    "iflow chat: invalid response format".to_string(),
                )
            })
    }
}

/// 检查 API Key 是否需要刷新（2天内过期）
pub fn should_refresh_api_key(expire_time: &str) -> crate::Result<(bool, Duration)> {
    if expire_time.trim().is_empty() {
        return Err(crate::NeuroLoomError::LlmProvider(
            "iflow cookie: expire time is empty".to_string(),
        ));
    }

    // 解析时间格式 "2006-01-02 15:04"
    let expire = chrono::NaiveDateTime::parse_from_str(expire_time.trim(), "%Y-%m-%d %H:%M")
        .map_err(|e| {
            crate::NeuroLoomError::LlmProvider(format!(
                "iflow cookie: parse expire time failed: {e}"
            ))
        })?;

    let expire_utc = DateTime::<Utc>::from_naive_utc_and_offset(expire, Utc);
    let now = Utc::now();
    let two_days = Duration::from_secs(48 * 60 * 60);

    let needs_refresh = expire_utc.signed_duration_since(now).num_seconds() < two_days.as_secs() as i64;
    let time_until_expiry = if expire_utc > now {
        Duration::from_secs((expire_utc - now).num_seconds() as u64)
    } else {
        Duration::ZERO
    };

    Ok((needs_refresh, time_until_expiry))
}

/// 从 Cookie 中提取 BXAuth 字段
pub fn extract_bx_auth(cookie: &str) -> String {
    // 使用 regex 提取 BXAuth 值
    let re = Regex::new(r"BXAuth=([^;]+)").unwrap();
    re.captures(cookie)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_bx_auth() {
        let cookie = "session=abc123; BXAuth=xyz789; path=/";
        assert_eq!(extract_bx_auth(cookie), "xyz789");

        let cookie_no_bxauth = "session=abc123; path=/";
        assert_eq!(extract_bx_auth(cookie_no_bxauth), "");

        let cookie_bxauth_first = "BXAuth=first123; session=abc";
        assert_eq!(extract_bx_auth(cookie_bxauth_first), "first123");
    }

    #[test]
    fn test_should_refresh_api_key() {
        // 已过期
        let past = "2020-01-01 00:00";
        let (needs_refresh, _) = should_refresh_api_key(past).unwrap();
        assert!(needs_refresh);

        // 未来时间
        let future = "2099-12-31 23:59";
        let (needs_refresh, _) = should_refresh_api_key(future).unwrap();
        assert!(!needs_refresh);
    }

    #[test]
    fn test_token_storage_save_load() {
        let storage = IFlowTokenStorage {
            api_key: "test-key".to_string(),
            email: "test@example.com".to_string(),
            expire_time: "2099-12-31 23:59".to_string(),
            cookie: "BXAuth=test-auth;".to_string(),
            last_refresh: Utc::now().to_rfc3339(),
            storage_type: "iflow".to_string(),
        };

        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("iflow_token_test.json");

        storage.save_to_file(&path).unwrap();
        let loaded = IFlowTokenStorage::load_from_file(&path).unwrap();

        assert_eq!(loaded.api_key, storage.api_key);
        assert_eq!(loaded.email, storage.email);

        // 清理
        let _ = std::fs::remove_file(&path);
    }
}
