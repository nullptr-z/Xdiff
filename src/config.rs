// 引入需要使用的依赖
use anyhow::{Context, Ok, Result};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::Path};

use crate::{utils::diff_text, ExtraArgs, RequestProfile};

// 配置文件结构体
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DiffConfig {
    // 不定项字段，用于保存多个 DiffProfile
    #[serde(flatten)]
    pub profiles: HashMap<String, DiffProfile>,
}

// 请求配置结构体
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

// 判断是否为默认值
fn is_default<T: Default + PartialEq>(t: &T) -> bool {
    t == &T::default()
}

// 响应配置结构体
#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq)]
pub struct ResponseProfile {
    // 跳过的响应头字段
    #[serde(skip_serializing_if = "Vec::is_empty ", default)]
    pub skip_headers: Vec<String>,
    // 跳过的响应体字段
    #[serde(skip_serializing_if = "Vec::is_empty ", default)]
    pub skip_body: Vec<String>,
}

impl DiffConfig {
    /// 从文件加载配置
    /// load config from file
    pub fn load_yaml(path: impl AsRef<Path>) -> Result<Self> {
        let absolute_path = std::env::current_dir().unwrap().join(path.as_ref());
        let content = fs::read_to_string(absolute_path).unwrap();
        Self::from_yaml(&content)
    }

    /// 从字符串加载配置
    /// load config from string
    pub fn from_yaml(content: &str) -> anyhow::Result<Self> {
        let config: Self = serde_yaml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    // 校验请求配置是否正确，使用 RequestProfile 的 validate 方法验证
    fn validate(&self) -> Result<()> {
        for (name, profile) in &self.profiles {
            profile
                .validate()
                .context(format!("failed to validate profile`验证失败: `{}`", name))?;
        }
        Ok(())
    }

    // 获取指定名称的 DiffProfile
    pub fn get_profile(&self, name: &str) -> Option<&DiffProfile> {
        self.profiles.get(name)
    }
}

/// 对两个请求进行差异比较
impl DiffProfile {
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

    // 校验请求配置[1,2]是否正确，使用 RequestProfile 的 validate 方法验证
    pub(crate) fn validate(&self) -> Result<()> {
        self.req1.validate().context("req1 failed to validate")?;
        self.req2.validate().context("req2 failed to validate")?;

        Ok(())
    }
}
