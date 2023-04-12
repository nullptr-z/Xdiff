use super::RequestProfile;
use crate::{is_default, utils::diff_text, ConfigValidate, ExtraArgs, LoadConfig};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 配置文件结构体, 用于保存多个 DiffProfile
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DiffConfig {
    // 不定项字段，包含多个 DiffProfile
    #[serde(flatten)]
    pub profiles: HashMap<String, DiffProfile>,
}

/// 保存需要进行差异比较的请求配置；\
/// 包含比较 `req1:req2` 两个请求的配置和一个响应`res`配置
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DiffProfile {
    // 请求1配置
    pub req1: RequestProfile,
    // 请求2配置
    pub req2: RequestProfile,
    // 响应配置
    #[serde(skip_serializing_if = "is_default", default)]
    pub res: ResponseProfile,
}

/// 用于保存需要跳过的响应头和响应体字段
#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq)]
pub struct ResponseProfile {
    // 跳过的响应头字段
    #[serde(skip_serializing_if = "Vec::is_empty ", default)]
    pub skip_headers: Vec<String>,
    // 跳过的响应体字段
    #[serde(skip_serializing_if = "Vec::is_empty ", default)]
    pub skip_body: Vec<String>,
}

impl ResponseProfile {
    pub fn new(skip_headers: Vec<String>, skip_body: Vec<String>) -> Self {
        Self {
            skip_headers,
            skip_body,
        }
    }
}
impl LoadConfig for DiffConfig {}

impl DiffConfig {
    // 接受一个DiffProfile集合，构建DiffConfig
    pub fn new(profiles: HashMap<String, DiffProfile>) -> Self {
        Self { profiles }
    }

    // 获取指定名称的 DiffProfile
    pub fn get_profile(&self, name: &str) -> Option<&DiffProfile> {
        self.profiles.get(name)
    }
}

/// 对两个请求进行差异比较
impl DiffProfile {
    // 创建new函数，传入请求配置[1,2]，和响应：req1,req2,res
    pub fn new(req1: RequestProfile, req2: RequestProfile, res: ResponseProfile) -> Self {
        Self { req1, req2, res }
    }

    // 差异比较，返回结果
    pub async fn diff(&self, args: &ExtraArgs) -> Result<String> {
        // 用 args 覆盖请求中的参数：headers，query，body
        // use args to override the parameters in the request
        let res1 = self.req1.send(&args).await?;
        let res2 = self.req2.send(&args).await?;

        // 过滤响应内容字段
        // filter response content fields
        let text1 = res1.get_text(&self.res).await?;
        let text2 = res2.get_text(&self.res).await?;

        diff_text(&text1, &text2)
    }
}

impl ConfigValidate for DiffProfile {
    // 校验请求配置[1,2]是否正确，使用 RequestProfile 的 validate 方法验证
    fn validate(&self) -> Result<()> {
        self.req1.validate().context("req1 failed to validate")?;
        self.req2.validate().context("req2 failed to validate")?;

        Ok(())
    }
}

impl ConfigValidate for DiffConfig {
    // 校验请求配置是否正确，使用 RequestProfile 的 validate 方法验证
    fn validate(&self) -> Result<()> {
        for (name, profile) in &self.profiles {
            profile
                .validate()
                .context(format!("failed to validate profile`验证失败: `{}`", name))?;
        }
        Ok(())
    }
}
