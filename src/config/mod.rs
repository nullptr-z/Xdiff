mod xdiff;
mod xreq;

// 引入需要使用的依赖
pub use xdiff::*;
pub use xreq::*;

// 引入需要使用的库
use crate::ExtraArgs;
use anyhow::{Ok, Result};
use reqwest::{
    header::{self, HeaderMap, HeaderName, HeaderValue},
    Client, Method, Response, Url,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::json;
use std::{fmt::Write, fs, ops::Deref, path::Path, str::FromStr};

pub trait LoadConfig
where
    Self: Sized + ConfigValidate + DeserializeOwned,
{
    /// load config from file
    /// 从文件加载配置
    fn load_yaml(path: impl AsRef<Path>) -> Result<Self> {
        let absolute_path = std::env::current_dir().unwrap().join(path.as_ref());
        let content = fs::read_to_string(absolute_path).unwrap();
        Self::from_yaml(&content)
    }

    /// load config from string
    /// 从字符串加载配置
    fn from_yaml(content: &str) -> Result<Self> {
        let config: Self = serde_yaml::from_str(content)?;
        config.validate()?;
        Ok(config)
    }
}

pub trait ConfigValidate {
    fn validate(&self) -> Result<()>;
}

// 定义一个请求的结构体 RequestProfile
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RequestProfile {
    // 定义请求方法，默认为GET请求
    #[serde(with = "http_serde::method", default)]
    pub method: Method,
    // 定义请求的URL地址
    pub url: Url,
    // 定义请求参数，为JSON格式的数据
    #[serde(skip_serializing_if = "empty_json_value", default)]
    pub params: Option<serde_json::Value>,
    // 定义请求头，为HTTP的HeaderMap类型
    #[serde(
        skip_serializing_if = "HeaderMap::is_empty",
        with = "http_serde::header_map",
        default
    )]
    pub headers: HeaderMap,
    // 定义请求体，为JSON格式的数据
    #[serde(skip_serializing_if = "empty_json_value", default)]
    pub body: Option<serde_json::Value>,
}

// 如果返回结果为false, 将不会序列化该字段
fn empty_json_value(v: &Option<serde_json::Value>) -> bool {
    // 判断v是否为None，如果是则返回true，否则返回v.is_null()
    v.as_ref().map_or(true, |v| v.is_null() || v.is_object())
}

// 定义一个响应的扩展结构体 ResponseExt，实现Deref trait，以支持引用ResponseExt时能够访问Response对象
#[derive(Debug)]
pub struct ResponseExt(Response);

impl Deref for ResponseExt {
    type Target = Response;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// 为RequestProfile定义一些方法
impl RequestProfile {
    // 创建一个新的RequestProfile对象
    pub fn new(
        method: Method,
        url: Url,
        params: Option<serde_json::Value>,
        headers: HeaderMap,
        body: Option<serde_json::Value>,
    ) -> Self {
        RequestProfile {
            method,
            url,
            params,
            headers,
            body,
        }
    }

    // 发送请求，并返回一个Result<ResponseExt>对象
    pub async fn send(&self, args: &ExtraArgs) -> Result<ResponseExt> {
        // 生成请求的HeaderMap、请求参数、请求体
        let (headers, query, body) = self.generate(args)?;
        // 创建一个reqwest::Client对象
        let client = Client::new();
        // 根据请求的参数创建一个reqwest::Request对象
        let req = client
            .request(self.method.clone(), self.url.clone())
            .headers(headers)
            .query(&query)
            .body(body)
            .build()
            .unwrap();
        // 发送请求并返回ResponseExt对象
        let res = client.execute(req).await?;
        Ok(ResponseExt(res))
    }

    // 从ExtraArgs提取数据生成url
    pub fn get_url(&self, args: &ExtraArgs) -> Result<String> {
        let mut url = self.url.clone();
        let (_, params, _) = self.generate(args)?;

        if !params.as_object().unwrap().is_empty() {
            let query = serde_qs::to_string(&params)?;
            url.set_query(Some(&query));
        }
        Ok(url.to_string())
    }

    // 生成请求的HeaderMap、请求参数、请求体
    fn generate(&self, args: &ExtraArgs) -> Result<(HeaderMap, serde_json::Value, String)> {
        let mut headers = HeaderMap::new();
        let mut query = self.params.clone().unwrap_or_else(|| json!({}));
        let mut body = self.body.clone().unwrap_or_else(|| json!({}));

        // 将ExtraArgs中的headers合并到headers中
        for (k, v) in &args.headers {
            headers.insert(HeaderName::from_str(k)?, HeaderName::from_str(v)?.into());
        }

        // 如果headers中没有设置Content-Type，则设置为application/json
        if !headers.contains_key(header::CONTENT_TYPE) {
            headers.insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/json"),
            );
        }

        for (k, v) in &args.query {
            query[k] = v.parse()?;
        }

        for (k, v) in &args.body {
            body[k] = v.parse()?;
        }

        // 根据不同的 content type，将body序列化(serialize)为不同的格式
        // Serialize the body into different formats according to different content types
        let content_type = get_content_type(&headers);
        match content_type.as_deref() {
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

impl FromStr for RequestProfile {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        // 字符串里提取 url
        let mut url = Url::parse(s)?;
        // url里提取 query
        let qs = url.query_pairs();
        // 初始化一个空 JSON格式 params
        let mut params = json!({});
        // 从query里提取出来所有参数，保存在params
        for (k, v) in qs {
            params[&*k] = v.parse()?;
        }
        // 清除url里的query
        url.set_query(None);

        Ok(RequestProfile::new(
            Method::GET,
            url,
            Some(params),
            HeaderMap::new(),
            None,
        ))
    }
}

impl ConfigValidate for RequestProfile {
    fn validate(&self) -> Result<()> {
        if let Some(param) = self.params.as_ref() {
            if !param.is_object() {
                // params 必须是 Object 对象,得到错误 yml 配置选项
                return Err(anyhow::anyhow!(
                    "Params must be an object but got: \n{}\n",
                    serde_yaml::to_string(param).unwrap()
                ));
            }
        }
        if let Some(body) = self.body.as_ref() {
            // body 必须是 Object 对象
            if !body.is_object() {
                return Err(anyhow::anyhow!(
                    "Body must be an object but got: \n{}\n",
                    serde_yaml::to_string(body).unwrap()
                ));
            }
        }

        Ok(())
    }
}

impl ResponseExt {
    pub fn into_inner(self) -> Response {
        self.0
    }

    // 为 Response 对象添加一个获取文本的方法，该方法接受一个 ResponseProfile 对象并返回一个字符串
    pub async fn get_text(self, profile: &ResponseProfile) -> Result<String> {
        // 获取 Response 对象
        let res = self.0;
        // 获取响应字符串

        let mut output = String::new();
        let status = get_status_text(&res);
        let header = get_heardes_text(&res, &profile.skip_headers)?;
        let body = get_body_text(res, &profile.skip_body).await?;
        writeln!(&mut output, "{}\n{}\n{}", status, header, body)?;

        Ok(output)
    }

    pub fn get_headers_keys(&self) -> Vec<String> {
        let res = &self.0;
        let headers = res.headers();
        headers.iter().map(|(k, _)| k.to_string()).collect()
    }
}

// 过滤 JSON 字符串，返回过滤后的字符串
fn filter_json(text: &str, skip: &[String]) -> Result<String> {
    // 将 JSON 字符串解析为 serde_json::Value 对象
    let mut json: serde_json::Value = serde_json::from_str(text)?;

    if let serde_json::Value::Object(ref mut map) = json {
        // 对 JSON 对象进行过滤，去除指定的键值对
        for k in skip {
            map.remove(k);
        }
    }
    Ok(serde_json::to_string_pretty(&json)?)
}

/// 获取响应的 content type
fn get_content_type(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().unwrap().split(';').next().map(|v| v.to_string()))
}

/// 获取http版本、响应的状态码和状态文本
pub fn get_status_text(res: &Response) -> String {
    let status = res.status();
    format!(
        "{:?} {} {}",
        res.version(),
        status.as_str(),
        status.canonical_reason().unwrap_or("")
    )
}

// 获取响应头的文本表示
pub fn get_heardes_text(res: &Response, skip_headers: &[String]) -> Result<String> {
    let mut output = String::new();

    let headers = res.headers();
    // 输出所有非过滤的响应头
    for (h_name, h_value) in headers {
        if !skip_headers.contains(&h_name.to_string()) {
            writeln!(&mut output, "{}: {:?}", h_name, h_value)?;
        }
    }
    writeln!(&mut output)?;
    Ok(output)
}

pub async fn get_body_text(res: Response, skip_headers: &[String]) -> Result<String> {
    let mut output = String::new();
    let content_type = get_content_type(res.headers());
    let text = res.text().await?;
    match content_type.as_deref() {
        Some("application/json") => {
            let text = filter_json(&text, skip_headers)?;
            writeln!(&mut output, "{}", text)?;
        }
        _ => {
            writeln!(&mut output, "{}", text)?;
        }
    }
    Ok(output)
}
