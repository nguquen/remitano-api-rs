use std::time::{Duration, SystemTime};

use derive_builder::Builder;
use hmac::{Hmac, Mac};
use hyperx::header::HttpDate;
use md5::{Digest, Md5};
use reqwest::header::{HeaderMap, ACCEPT, AUTHORIZATION, CONTENT_TYPE, DATE, USER_AGENT};
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};
use sha1::Sha1;

const API_URL: &str = "https://api.remitano.com";

type HmacSha1 = Hmac<Sha1>;

#[derive(Default, Builder, Debug)]
pub struct RemitanoApi {
    pub key: String,

    pub secret: String,

    #[builder(default = r#"API_URL.to_string()"#)]
    pub api_url: String,

    #[builder(default = "3000")]
    pub timeout_ms: u64,
}

pub use reqwest::Method;

impl RemitanoApi {
    fn hmac(&self, data: &Option<Value>) -> anyhow::Result<String> {
        let value = match data {
            Some(data) => match data {
                Value::String(data) => data.as_bytes().to_vec(),
                _ => serde_json::to_vec(&data)?,
            },
            None => vec![],
        };

        let mut mac = HmacSha1::new_from_slice(self.secret.as_bytes())?;
        mac.update(&value);
        let result = mac.finalize().into_bytes();

        Ok(base64::encode(result))
    }

    fn md5(&self, data: &Option<Value>) -> anyhow::Result<String> {
        let value = match data {
            Some(data) => match data {
                Value::String(data) => data.as_bytes().to_vec(),
                _ => serde_json::to_vec(&data)?,
            },
            None => vec![],
        };

        let mut hasher = Md5::new();
        hasher.update(&value);
        let result = hasher.finalize();

        Ok(base64::encode(result))
    }

    pub async fn request<T: DeserializeOwned>(
        &self,
        method: Method,
        endpoint: &str,
        params: Option<Map<String, Value>>,
        body: Option<Value>,
    ) -> anyhow::Result<T> {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:85.0) Gecko/20100101 Firefox/85.0"
                .parse()?,
        );
        headers.insert(ACCEPT, "application/json".parse()?);
        headers.insert(CONTENT_TYPE, "application/json".parse()?);
        headers.insert("Content-MD5", self.md5(&body)?.parse()?);
        headers.insert(DATE, HttpDate::from(SystemTime::now()).to_string().parse()?);

        let query = if let Some(params) = &params {
            format!("?{}", &serde_qs::to_string(&params)?)
        } else {
            "".to_string()
        };

        let request_url = format!("api/v1/{}{}", &endpoint, &query);
        let request_str = format!(
            "{},application/json,{},/{},{}",
            &method,
            headers
                .get("Content-MD5")
                .map_or_else(|| Some(""), |v| v.to_str().ok())
                .unwrap(),
            &request_url,
            headers
                .get(DATE)
                .map_or_else(|| Some(""), |v| v.to_str().ok())
                .unwrap(),
        );
        let sig = self.hmac(&Some(Value::String(request_str)))?;
        headers.insert(
            AUTHORIZATION,
            format!("APIAuth {}:{}", &self.key, &sig).parse()?,
        );

        let client = reqwest::Client::new();
        let resp: T = client
            .request(method, format!("{}/{}", &self.api_url, &request_url))
            .headers(headers)
            .json(&body.unwrap_or_default())
            .timeout(Duration::from_millis(self.timeout_ms))
            .send()
            .await?
            .json()
            .await?;

        Ok(resp)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::*;

    #[test]
    fn test_md5() {
        let remitano_api = RemitanoApiBuilder::default()
            .key("key".to_string())
            .secret("secret".to_string())
            .build()
            .unwrap();

        let input = "hash me";
        let result = remitano_api.md5(&Some(json!(input))).unwrap();
        assert_eq!("F7Mdzpa51sbQprqV9HeW+w==", result);
    }

    #[test]
    fn test_hmac256() {
        let remitano_api = RemitanoApiBuilder::default()
            .key("key".to_string())
            .secret("secret".to_string())
            .build()
            .unwrap();

        let input = "hash me";
        let result = remitano_api.hmac(&Some(json!(input))).unwrap();
        assert_eq!("oSVlCBpf9BqviWbUjOm4DXEcgRo=", result);
    }
}
