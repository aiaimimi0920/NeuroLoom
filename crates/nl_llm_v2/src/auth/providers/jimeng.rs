use hex;
use hmac::{Hmac, Mac};
use reqwest::header::{HeaderMap, HeaderValue};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::auth::Authenticator;
use anyhow::Result;

type HmacSha256 = Hmac<Sha256>;

pub struct JimengAuth {
    access_key: String,
    secret_key: String,
    region: String,
    service: String,
}

impl JimengAuth {
    pub fn new(ak: impl Into<String>, sk: impl Into<String>) -> Self {
        Self {
            access_key: ak.into(),
            secret_key: sk.into(),
            region: "cn-north-1".to_string(), // 火山引擎默认区域
            service: "cv".to_string(),        // 即梦服务名
        }
    }

    fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
        let mut mac = HmacSha256::new_from_slice(key).expect("HMAC can take key of any size");
        mac.update(data);
        mac.finalize().into_bytes().to_vec()
    }

    fn sha256_hex(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hex::encode(hasher.finalize())
    }

    /// Sign the request according to Volcengine API Signature V4
    pub fn sign_request(
        &self,
        method: &str,
        host: &str,
        path: &str,
        query: &str,
        headers: &mut HeaderMap,
        body: &[u8],
    ) -> Result<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let utc_time = chrono::DateTime::from_timestamp(now as i64, 0).unwrap();

        let x_date = utc_time.format("%Y%m%dT%H%M%SZ").to_string();
        let short_date = utc_time.format("%Y%m%d").to_string();

        let hex_payload_hash = Self::sha256_hex(body);

        headers.insert("Host", HeaderValue::from_str(host).unwrap());
        headers.insert("X-Date", HeaderValue::from_str(&x_date).unwrap());
        headers.insert(
            "X-Content-Sha256",
            HeaderValue::from_str(&hex_payload_hash).unwrap(),
        );

        if !headers.contains_key("Content-Type") {
            headers.insert("Content-Type", HeaderValue::from_static("application/json"));
        }

        // Canonical Headers
        let content_type = headers.get("Content-Type").unwrap().to_str().unwrap();
        let canonical_headers = format!(
            "content-type:{}\nhost:{}\nx-content-sha256:{}\nx-date:{}\n",
            content_type, host, hex_payload_hash, x_date
        );
        let signed_headers = "content-type;host;x-content-sha256;x-date";

        let canonical_request = format!(
            "{}\n{}\n{}\n{}\n{}\n{}",
            method, path, query, canonical_headers, signed_headers, hex_payload_hash
        );

        let hashed_canonical_request = Self::sha256_hex(canonical_request.as_bytes());
        let credential_scope = format!("{}/{}/{}/request", short_date, self.region, self.service);

        let string_to_sign = format!(
            "HMAC-SHA256\n{}\n{}\n{}",
            x_date, credential_scope, hashed_canonical_request
        );

        let k_date = Self::hmac_sha256(self.secret_key.as_bytes(), short_date.as_bytes());
        let k_region = Self::hmac_sha256(&k_date, self.region.as_bytes());
        let k_service = Self::hmac_sha256(&k_region, self.service.as_bytes());
        let k_signing = Self::hmac_sha256(&k_service, b"request");
        let signature = hex::encode(Self::hmac_sha256(&k_signing, string_to_sign.as_bytes()));

        let authorization = format!(
            "HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
            self.access_key, credential_scope, signed_headers, signature
        );

        headers.insert(
            reqwest::header::AUTHORIZATION,
            HeaderValue::from_str(&authorization)
                .map_err(|e| anyhow::anyhow!("Invalid Volcengine sig: {}", e))?,
        );

        Ok(())
    }
}

// Implement Authenticator if you use it in global Request Pipeline
#[async_trait::async_trait]
impl Authenticator for JimengAuth {
    fn id(&self) -> &str {
        "jimeng"
    }

    fn is_authenticated(&self) -> bool {
        !self.access_key.is_empty() && !self.secret_key.is_empty()
    }

    fn auth_type(&self) -> crate::site::context::AuthType {
        crate::site::context::AuthType::ApiKey
    }

    fn inject(&self, builder: reqwest::RequestBuilder) -> Result<reqwest::RequestBuilder> {
        Ok(builder)
    }
}
