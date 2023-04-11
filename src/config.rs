use anyhow::{Ok, Result};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::Path};

use crate::{utils::diff_text, ExtraArgs, RequestProfile};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DiffConfig {
    #[serde(flatten)]
    pub profiles: HashMap<String, DiffProfile>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DiffProfile {
    pub req1: RequestProfile,
    pub req2: RequestProfile,
    pub res: ResponseProfile,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResponseProfile {
    #[serde(skip_serializing_if = "Vec::is_empty ", default)]
    pub skip_headers: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty ", default)]
    pub skip_body: Vec<String>,
}

impl DiffConfig {
    pub fn load_yaml(path: impl AsRef<Path>) -> Result<Self> {
        let absolute_path = std::env::current_dir().unwrap().join(path.as_ref());
        let content = fs::read_to_string(absolute_path).unwrap();
        Self::from_yaml(&content)
    }

    pub fn from_yaml(content: &str) -> anyhow::Result<Self> {
        Ok(serde_yaml::from_str(&content)?)
    }

    pub fn get_profile(&self, name: &str) -> Option<&DiffProfile> {
        self.profiles.get(name)
    }
}

/// Diff the two requests
/// 对两个请求进行差异比较
impl DiffProfile {
    pub async fn diff(&self, args: &ExtraArgs) -> Result<String> {
        // 用 args 覆盖请求中的参数：headers，query，body
        // use args to override the parameters in the request
        let res1 = self.req1.send(&args).await?;
        let res2 = self.req2.send(&args).await?;

        // 过滤响应内容字段
        // filter response content fields
        let text1 = res1.get_text(&self.res).await?;
        let text2 = res2.get_text(&self.res).await?;

        Ok(diff_text(&text1, &text2)?)
    }
}
