use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use crate::auth::traits::Authenticator;
use crate::provider::extension::{ProviderExtension, ModelInfo};
use crate::concurrency::ConcurrencyConfig;
use std::sync::Arc;

/// 智谱默认 API 基础 URL
const DEFAULT_BASE_URL: &str = "https://open.bigmodel.cn/api/paas/v4";

/// 智谱 AI (Zhipu / BigModel) 静态模型列表扩展
///
/// 国内版 API: https://open.bigmodel.cn/api/paas/v4
pub struct ZhipuExtension {
    /// API 基础 URL，用于构建余额查询等管理端点
    base_url: String,
}

impl ZhipuExtension {
    pub fn new() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }

    /// 设置自定义基础 URL（用于代理场景）
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into().trim_end_matches('/').to_string();
        self
    }

    /// 构建余额查询 URL
    fn build_billing_url(&self) -> String {
        format!("{}/billing/quota", self.base_url)
    }
}

impl Default for ZhipuExtension {
    fn default() -> Self {
        Self::new()
    }
}

fn zhipu_models() -> Vec<ModelInfo> {
    vec![
        // === 常规模型 ===
        ModelInfo {
            id: "glm-5".to_string(),
            description: "GLM-5 — 旗舰模型，128K context".to_string(),
        },
        ModelInfo {
            id: "glm-4".to_string(),
            description: "GLM-4 — 多模态模型，128K context".to_string(),
        },
        ModelInfo {
            id: "glm-4-flash".to_string(),
            description: "GLM-4 Flash — 轻快模型，128K context".to_string(),
        },
        ModelInfo {
            id: "glm-4-plus".to_string(),
            description: "GLM-4 Plus — 增强模型，128K context".to_string(),
        },
        ModelInfo {
            id: "glm-4-air".to_string(),
            description: "GLM-4 Air — 轻量模型，128K context".to_string(),
        },
        ModelInfo {
            id: "glm-4-long".to_string(),
            description: "GLM-4 Long — 长上下文模型，1M context".to_string(),
        },
        // === 思考模型（推理增强） ===
        ModelInfo {
            id: "glm-z1-airx".to_string(),
            description: "GLM-Z1 AirX — 思考模型，128K context".to_string(),
        },
        ModelInfo {
            id: "glm-z1-flash".to_string(),
            description: "GLM-Z1 Flash — 快速思考模型，128K context".to_string(),
        },
    ]
}

/// 智谱余额 API 响应
/// API: GET {base_url}/billing/quota
#[derive(Deserialize)]
struct ZhipuQuotaResponse {
    success: bool,
    data: Option<ZhipuQuotaData>,
}

#[derive(Deserialize)]
struct ZhipuQuotaData {
    /// 总额度（单位：元）
    total_quota: Option<f64>,
    /// 已使用额度（单位：元）
    used_quota: Option<f64>,
    /// 剩余额度（单位：元）
    remain_quota: Option<f64>,
    /// 赠送额度
    granted_quota: Option<f64>,
}

#[async_trait]
impl ProviderExtension for ZhipuExtension {
    fn id(&self) -> &str {
        "zhipu"
    }

    async fn list_models(
        &self,
        _http: &Client,
        _auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Vec<ModelInfo>> {
        Ok(zhipu_models())
    }

    async fn get_balance(
        &self,
        http: &Client,
        auth: &mut dyn Authenticator,
    ) -> anyhow::Result<Option<String>> {
        let url = self.build_billing_url();
        let req = http.get(&url);
        let req = auth.inject(req)?;

        let resp = req.send().await?;
        let status = resp.status();

        if !status.is_success() {
            let err = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("智谱余额查询失败 ({}): {}", status, err));
        }

        let json: ZhipuQuotaResponse = resp.json().await
            .map_err(|e| anyhow::anyhow!("解析余额响应失败: {}", e))?;

        if !json.success {
            return Ok(Some("余额查询失败（API 返回 success=false）".to_string()));
        }

        if let Some(data) = json.data {
            let mut parts = Vec::new();

            if let Some(total) = data.total_quota {
                parts.push(format!("总额度: ¥{:.2}", total));
            }
            if let Some(used) = data.used_quota {
                parts.push(format!("已用: ¥{:.2}", used));
            }
            if let Some(remain) = data.remain_quota {
                parts.push(format!("剩余: ¥{:.2}", remain));
            }
            if let Some(granted) = data.granted_quota {
                parts.push(format!("赠送: ¥{:.2}", granted));
            }

            if parts.is_empty() {
                Ok(Some("账户有效（无额度信息）".to_string()))
            } else {
                Ok(Some(parts.join(", ")))
            }
        } else {
            Ok(Some("账户有效（无额度详情）".to_string()))
        }
    }

    fn concurrency_config(&self) -> ConcurrencyConfig {
        // 智谱: 默认并发限制
        ConcurrencyConfig::new(10)
    }
}

pub fn extension() -> Arc<ZhipuExtension> {
    Arc::new(ZhipuExtension::new())
}
