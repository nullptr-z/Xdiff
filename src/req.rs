use std::{ops::Deref, str::FromStr};

use anyhow::Result;
use reqwest::{
    header::{self, HeaderMap, HeaderName},
    Client, Method, Response, Url,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use similar::DiffableStr;

use crate::{ExtraArgs, ResponseProfile};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RequestProfile {
    #[serde(with = "http_serde::method", default)]
    pub method: Method,
    pub url: Url,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub params: Option<serde_json::Value>,
    #[serde(
        skip_serializing_if = "HeaderMap::is_empty",
        with = "http_serde::header_map",
        default
    )]
    pub headers: HeaderMap,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub body: Option<serde_json::Value>,
}

#[derive(Debug)]
pub struct ResponseExt(Response);

impl Deref for ResponseExt {
    type Target = Response;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl RequestProfile {
    pub async fn send(&self, args: &ExtraArgs) -> Result<ResponseExt> {
        let (headers, query, body) = self.generate(args)?;
        let client = Client::new();
        let req = client
            .request(self.method.clone(), self.url.clone())
            .headers(headers)
            .query(&query.unwrap())
            .body(body)
            .build()?;

        let res = client.execute(req).await?;

        Ok(ResponseExt(res))
    }

    fn generate(&self, args: &ExtraArgs) -> Result<(HeaderMap, Option<serde_json::Value>, String)> {
        let mut headers = HeaderMap::new();
        let mut query = None;
        let body = "";

        for (k, v) in &args.headers {
            headers.insert(HeaderName::from_str(k).unwrap(), v.parse().unwrap());
        }

        if !headers.contains_key(header::CONTENT_TYPE) {
            headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
        }

        if !args.query.is_empty() {
            query = Some(serde_json::json!(args.query));
        }

        // 根据不同的 content type，将body序列化(serialize)为不同的格式
        // Serialize the body into different formats according to different content types
        let content_type = get_content_type(&headers);
        match content_type {
            Some("application/json") => {
                let body = serde_json::to_string(&body)?;
                Ok((headers, query, body))
            }
            Some("application/x-www-form-urlencoded" | "multipart/form-data") => {
                let body = serde_urlencoded::to_string(&body)?;
                Ok((headers, query, body))
            }
            _ => Err(anyhow::anyhow!(
                "Unsupported content type`不支持的内容类型 {:?}`",
                content_type
            )),
        }
    }
}

fn get_content_type(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::CONTENT_TYPE)
        .map(|v| v.to_str().unwrap().split(';').next())
        .flatten()
}

impl ResponseExt {
    pub async fn filter_text(&self, profile: &ResponseProfile) -> Result<String> {
        let mut output = String::new();
        let headers = self.headers();
        for (h_name, h_value) in headers {
            if !profile.skip_headers.contains(&h_name.to_string()) {
                output.push_str(&format!("{}: {:?}\r)", h_name, h_value));
            }
        }

        let text = self.text().await?;
        let content_type = get_content_type(&headers);
        match content_type {
            Some("application/json") => {
                let text = self.filter_json(&text, &profile.skip_body)?;
                output.push_str(&serde_json::to_string_pretty(&text)?);
            }
            _ => {
                output.push_str(&text);
            }
        }
        let mut json = serde_json::Value::Null;
        todo!()
    }

    fn filter_json(&self, text: &str, skip: &[String]) -> Result<String> {
        let mut json: serde_json::Value = serde_json::from_str(text)?;

        match json {
            serde_json::Value::Object(ref mut map) => {
                for k in skip {
                    map.remove(k);
                }
            }
            _ => {
                // Todo json array
            }
        }
        Ok(serde_json::to_string_pretty(&json)?)
    }
}
