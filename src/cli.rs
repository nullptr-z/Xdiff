/// `符号代表我要翻译它之前的内容
use crate::ExtraArgs;
use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};

/// Diff two http requests and compare the diffrence of the responses
/// 对比两个 HTTP 请求的差异，并比较响应的差异
#[derive(Parser, Debug, Clone)]
#[clap(version,author,about,long_about=None)]
pub struct Args {
    #[clap(subcommand)]
    pub action: Action,
}

#[derive(Subcommand, Debug, Clone)]
#[non_exhaustive]
pub enum Action {
    #[clap(about = "Diff two http requests and compare the diffrence of the responses")]
    Run(RunArgs),
    /// 解析URLs生成一个 Profile
    /// Parse URLs and generate a Profile
    Parse,
}

#[derive(Parser, Debug, Clone)]
pub struct RunArgs {
    /// profile node name \
    /// 要使用配置中的节点名称 \
    /// `short: -p ,long: --profile`
    #[clap(short, long, value_parser)]
    pub profile: String,
    /// Overrides args, Could be used to override the query, headers and boyd of the qeurst
    /// 覆盖参数，可用于覆盖请求的查询、header和body\
    /// 对于查询参数，请使用 `-e key=value`\
    /// For query params use `-e key=value`
    /// For hearder, use `-e %key=value`\
    /// For body, use `-e @key=value`\
    /// example：`-e %Content-Type=application/json -e @name=hello`
    #[clap(short,long,value_parser=parse_key_val,number_of_values=1)]
    pub extar_params: Vec<KeyVal>,

    /// COnfiguration to use \
    /// 要使用的配置文件\
    /// `short: -c ,long: --config`
    #[clap(short, long, value_parser)]
    pub config: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyValType {
    Query,
    Header,
    Body,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyVal {
    pub key_type: KeyValType,
    pub key: String,
    pub value: String,
}

fn parse_key_val(s: &str) -> Result<KeyVal> {
    let mut parts = s.splitn(2, '=');
    let retrieve = |parts: Option<&str>| -> Result<String> {
        Ok(parts
            .ok_or_else(|| anyhow!("Invalid key value pair`无效的键值对: {:?}", s))?
            .trim()
            .to_string())
    };
    let key = retrieve(parts.next())?;
    let value = retrieve(parts.next())?;

    let (key_type, key) = match key.chars().next() {
        Some('%') => (KeyValType::Header, key[1..].to_string()),
        Some('@') => (KeyValType::Body, key[1..].to_string()),
        Some(v) if v.is_ascii_alphabetic() => (KeyValType::Query, key.to_string()), // is_ascii_alphabetic() 检查是否为字母
        _ => return Err(anyhow!("Invalid key type`无效的键类型")),
    };

    Ok(KeyVal {
        key_type,
        key,
        value: value.to_string(),
    })
}

impl From<Vec<KeyVal>> for ExtraArgs {
    fn from(args: Vec<KeyVal>) -> Self {
        let mut headers = vec![];
        let mut query = vec![];
        let mut body = vec![];

        for kv in args {
            match kv.key_type {
                KeyValType::Header => headers.push((kv.key, kv.value)),
                KeyValType::Query => query.push((kv.key, kv.value)),
                KeyValType::Body => body.push((kv.key, kv.value)),
            }
        }

        Self {
            headers,
            query,
            body,
        }
    }
}
