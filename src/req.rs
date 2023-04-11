// 引入需要使用的库
use anyhow::{Ok, Result};
use reqwest::{
    header::{self, HeaderMap, HeaderName, HeaderValue},
    Client, Method, Response, Url,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{fmt::Write, ops::Deref, str::FromStr};

// 引入模块
use crate::{ExtraArgs, ResponseProfile};

// 定义一个请求的结构体 RequestProfile
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RequestProfile {
    // 定义请求方法，默认为GET请求
    #[serde(with = "http_serde::method", default)]
    pub method: Method,
    // 定义请求的URL地址
    pub url: Url,
    // 定义请求参数，为JSON格式的数据
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub params: Option<serde_json::Value>,
    // 定义请求头，为HTTP的HeaderMap类型
    #[serde(
        skip_serializing_if = "HeaderMap::is_empty",
        with = "http_serde::header_map",
        default
    )]
    pub headers: HeaderMap,
    // 定义请求体，为JSON格式的数据
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub body: Option<serde_json::Value>,
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

    // 生成请求的HeaderMap、请求参数、请求体
    fn generate(&self, args: &ExtraArgs) -> Result<(HeaderMap, serde_json::Value, String)> {
        let mut headers = HeaderMap::new();
        let mut query = self.params.clone().unwrap_or_else(|| json!({}));
        let mut body = self.body.clone().unwrap_or_else(|| json!({}));

        // 将ExtraArgs中的headers合并到headers中
        for (k, v) in &args.headers {
            headers.insert(HeaderName::from_str(k).unwrap(), v.parse().unwrap());
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

    pub(crate) fn validate(&self) -> Result<()> {
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
    // 为 Response 对象添加一个获取文本的方法，该方法接受一个 ResponseProfile 对象并返回一个字符串
    pub async fn get_text(self, profile: &ResponseProfile) -> Result<String> {
        // 获取 Response 对象
        let res = self.0;
        // 获取响应头字符串
        let mut output = get_heardes_text(&res, &profile.skip_headers)?;

        // 获取响应的 content type
        let content_type = get_content_type(res.headers());
        // 获取响应文本
        let text = res.text().await?;

        // 根据 content type 处理文本
        match content_type.as_deref() {
            Some("application/json") => {
                // 过滤 JSON 字符串
                let text = filter_json(&text, &profile.skip_body)?;
                writeln!(&mut output, "{}", text)?;
            }
            Some("text/html") => {
                // 不做处理
                writeln!(&mut output, "{}", text)?;
            }
            _ => {
                // 不做处理
                writeln!(&mut output, "{}", text)?;
            }
        }

        Ok(output)
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

// 获取响应的 content type
fn get_content_type(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().unwrap().split(';').next().map(|v| v.to_string()))
}

// 获取响应头的文本表示
fn get_heardes_text(res: &Response, skip_headers: &[String]) -> Result<String> {
    let mut output = String::new();
    // 输出响应的版本和状态码
    writeln!(&mut output, "{:?} {}", res.version(), res.status())?;

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
