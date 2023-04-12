use anyhow::{Ok, Result};
use clap::Parser;
use dialoguer::{theme::ColorfulTheme, Input, MultiSelect};
use similar::DiffableStr;
use std::{io::Write, sync::MutexGuard};
use xdiff::{
    cli::{Action, Args, RunArgs},
    highlight_text, DiffConfig, DiffProfile, ExtraArgs, RequestProfile, ResponseProfile,
};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // tudo 1:02:01
    // 从Parse获取的yaml字符串，转换为DiffConfig,运行 run方法

    match args.action {
        Action::Run(args) => run(args).await?,
        Action::Parse => parse().await?,
        _ => panic!("Not implemented`没有该实现 "),
    }

    Ok(())
}

async fn parse() -> Result<()> {
    // 选择主题
    let theme = ColorfulTheme::default();
    // 从控制台获取用户输入的url1
    let url1: String = Input::with_theme(&theme)
        .with_prompt("Url1")
        .interact_text()?;
    // 从控制台获取用户输入的url2
    let url2: String = Input::with_theme(&theme)
        .with_prompt("Url2")
        .interact_text()?;

    // 解析出url1和url2的请求配置
    let req1: RequestProfile = url1.parse()?;
    let req2: RequestProfile = url2.parse()?;

    // 从控制台获取用户输入的profile_name
    let profile_name: String = Input::with_theme(&theme)
        .with_prompt("Profile")
        .interact_text()?;

    // 发送一个请求，从响应中生成 headers 的可选项
    let res = req1.send(&ExtraArgs::default()).await?;
    let headers = res.get_headers_keys();

    // 获取用户chosen`选择(多选)的下标，可选项为headers
    let chosen = MultiSelect::with_theme(&theme)
        .with_prompt("Select headers to skip")
        .items(&headers)
        .interact()?;
    // 从headers中获取用户选择的headers选项字符串数组
    let skip_headers = chosen.iter().map(|i| headers[*i].to_string()).collect();

    // 构建一个DiffProfile-start
    let res = ResponseProfile::new(skip_headers, vec![]);
    let profile = DiffProfile::new(req1, req2, res);
    // 完成DiffProfile的构建-end
    let config = DiffConfig::new(vec![(profile_name, profile)].into_iter().collect());
    // 将配置文件转换为yaml格式字符串
    let result = serde_yaml::to_string(&config)?;

    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    writeln!(stdout, "---\n{}---", highlight_text(&result, "yaml")?)?;
    run2(&result).await?;
    Ok(())
}

pub async fn run(args: RunArgs) -> Result<()> {
    let config_file = args.config.unwrap_or_else(|| "./xdiff.yml".to_string());
    let config = DiffConfig::load_yaml(&config_file)?;
    let profile = config.get_profile(&args.profile).ok_or_else(|| {
        anyhow::anyhow!(
            "Profile {} not found in config file {}`配置文件中未找到",
            args.profile,
            config_file
        )
    })?;

    let extra_args = args.extar_params.into();
    let output = profile.diff(&extra_args).await?;

    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    write!(stdout, "{}", output)?;

    Ok(())
}

pub async fn run2(content: &str) -> Result<()> {
    let config = DiffConfig::from_yaml(content)?;
    let profile = config.profiles.iter().next().unwrap().1;

    let output = profile.diff(&ExtraArgs::default()).await?;

    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    write!(stdout, "{}", output)?;

    Ok(())
}
