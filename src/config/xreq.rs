use crate::{ConfigValidate, LoadConfig, RequestProfile};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 包含多个请求配置
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RequestConfig {
    #[serde(flatten)]
    pub profiles: HashMap<String, RequestProfile>,
}

impl LoadConfig for RequestConfig {}

impl RequestConfig {
    // 接受一个RequestProfile集合，构建RequestConfig
    pub fn new(profiles: HashMap<String, RequestProfile>) -> Self {
        Self { profiles }
    }
    // 获取指定名称的 RequestProfile
    pub fn get_profile(&self, name: &str) -> Option<&RequestProfile> {
        self.profiles.get(name)
    }
}

impl ConfigValidate for RequestConfig {
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
