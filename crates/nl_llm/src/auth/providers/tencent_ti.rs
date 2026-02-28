use crate::auth::traits::Authenticator;
use async_trait::async_trait;
use chrono::Utc;
use hmac::{Hmac, Mac};
use reqwest::header::HeaderValue;
use sha2::{Digest, Sha256};
use std::fmt::Write;

type HmacSha256 = Hmac<Sha256>;

/// 腾讯云 API V3 (TC3-HMAC-SHA256) 鉴权结构
#[derive(Debug, Clone)]
pub struct TencentTiAuth {
    secret_id: String,
    secret_key: String,
    // 默认 action, 对于大模型通常是 "ChatCompletions" 或其他
    action: String,
    host: String,
    service: String,
    version: String,
    region: String,
}

impl TencentTiAuth {
    pub fn new(
        secret_id: impl Into<String>,
        secret_key: impl Into<String>,
        action: impl Into<String>,
        host: impl Into<String>,
        service: impl Into<String>,
        version: impl Into<String>,
        region: impl Into<String>,
    ) -> Self {
        Self {
            secret_id: secret_id.into(),
            secret_key: secret_key.into(),
            action: action.into(),
            host: host.into(),
            service: service.into(),
            version: version.into(),
            region: region.into(),
        }
    }

    fn sha256_hex(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        let mut hex = String::with_capacity(64);
        for byte in result {
            write!(&mut hex, "{:02x}", byte).unwrap();
        }
        hex
    }

    fn hmac_sha256(key: &[u8], msg: &str) -> Vec<u8> {
        let mut mac = HmacSha256::new_from_slice(key).expect("HMAC can take key of any size");
        mac.update(msg.as_bytes());
        mac.finalize().into_bytes().to_vec()
    }

    pub fn sign_request(
        &self,
        payload: &[u8],
    ) -> anyhow::Result<reqwest::header::HeaderMap> {
        let timestamp = Utc::now().timestamp();
        let date = Utc::now().format("%Y-%m-%d").to_string();

        let host = &self.host;
        let action = &self.action;
        let version = &self.version;
        let region = &self.region;

        // 1. Build Canonical Request
        let http_request_method = "POST";
        let canonical_uri = "/";
        let canonical_query_string = "";
        
        let canonical_headers = format!(
            "content-type:application/json\nhost:{}\nx-tc-action:{}\n",
            host,
            action.to_lowercase()
        );
        let signed_headers = "content-type;host;x-tc-action";
        let hashed_request_payload = Self::sha256_hex(payload);

        let canonical_request = format!(
            "{}\n{}\n{}\n{}\n{}\n{}",
            http_request_method,
            canonical_uri,
            canonical_query_string,
            canonical_headers,
            signed_headers,
            hashed_request_payload
        );

        // 2. Build String to Sign
        let algorithm = "TC3-HMAC-SHA256";
        let credential_scope = format!("{}/{}/tc3_request", date, self.service);
        let hashed_canonical_request = Self::sha256_hex(canonical_request.as_bytes());

        let string_to_sign = format!(
            "{}\n{}\n{}\n{}",
            algorithm,
            timestamp,
            credential_scope,
            hashed_canonical_request
        );

        // 3. Sign String
        let k_date = Self::hmac_sha256(format!("TC3{}", self.secret_key).as_bytes(), &date);
        let k_service = Self::hmac_sha256(&k_date, &self.service);
        let k_signing = Self::hmac_sha256(&k_service, "tc3_request");
        
        // Final signature in hex
        let signature_bytes = Self::hmac_sha256(&k_signing, &string_to_sign);
        let mut signature = String::with_capacity(64);
        for byte in signature_bytes {
            write!(&mut signature, "{:02x}", byte).unwrap();
        }

        // 4. Build Authorization header
        let authorization = format!(
            "{} Credential={}/{}, SignedHeaders={}, Signature={}",
            algorithm,
            self.secret_id,
            credential_scope,
            signed_headers,
            signature
        );

        // 5. Append Headers
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "Authorization",
            HeaderValue::from_str(&authorization).map_err(|e| anyhow::anyhow!(e))?,
        );
        headers.insert(
            "X-TC-Action",
            HeaderValue::from_str(action).map_err(|e| anyhow::anyhow!(e))?,
        );
        headers.insert(
            "X-TC-Timestamp",
            HeaderValue::from_str(&timestamp.to_string()).map_err(|e| anyhow::anyhow!(e))?,
        );
        headers.insert(
            "X-TC-Version",
            HeaderValue::from_str(version).map_err(|e| anyhow::anyhow!(e))?,
        );
        if !region.is_empty() {
            headers.insert(
                "X-TC-Region",
                HeaderValue::from_str(region).map_err(|e| anyhow::anyhow!(e))?,
            );
        }

        Ok(headers)
    }
}

#[async_trait]
impl Authenticator for TencentTiAuth {
    fn id(&self) -> &str {
        "tencent_ti"
    }

    fn is_authenticated(&self) -> bool {
        !self.secret_id.is_empty() && !self.secret_key.is_empty()
    }

    fn auth_type(&self) -> crate::site::context::AuthType {
        crate::site::context::AuthType::ApiKey
    }

    fn inject(&self, mut req: reqwest::RequestBuilder) -> anyhow::Result<reqwest::RequestBuilder> {
        let req_clone = req.try_clone().ok_or_else(|| anyhow::anyhow!("Failed to clone RequestBuilder for signing"))?;
        let built_req = req_clone.build()?;
        
        let payload = if let Some(body) = built_req.body() {
            if let Some(bytes) = body.as_bytes() {
                bytes.to_vec()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        let headers_to_add = self.sign_request(&payload)?;
        
        for (k, v) in headers_to_add.iter() {
            req = req.header(k, v);
        }

        Ok(req)
    }
}
