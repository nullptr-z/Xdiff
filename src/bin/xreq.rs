use anyhow::Result;
use clap::Parser;
use dialoguer::{theme::ColorfulTheme, Input};
use std::{fmt::Write as _, io::Write};
use xdiff::{
    cli::{Action, Args, RunArgs},
    get_body_text, get_heardes_text, get_status_text, highlight_text, print_error, LoadConfig,
    RequestConfig, RequestProfile,
};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // tudo 1:02:01
    // 从Parse获取的yaml字符串，转换为DiffConfig,运行 run方法

    let result = match args.action {
        Action::Run(args) => run(args).await,
        Action::Parse => parse().await,
        _ => panic!("Not implemented`没有该实现 "),
    };

    print_error(result)?;

    Ok(())
}

async fn run(args: RunArgs) -> Result<()> {
    let config_file = args.config.unwrap_or_else(|| "./xreq.yml".to_string());
    let config = RequestConfig::load_yaml(&config_file)?;
    let profile = config.get_profile(&args.profile).ok_or_else(|| {
        anyhow::anyhow!(
            "Profile {} not found in config file {}`配置文件中未找到",
            args.profile,
            config_file
        )
    })?;

    let extra_args = args.extar_params.into();
    let res = profile.send(&extra_args).await?.into_inner();
    let url = profile.get_url(&extra_args)?;

    // 获取响应字符串
    let mut output = String::new();

    let status = get_status_text(&res);
    let header = get_heardes_text(&res, &[])?;
    let body = get_body_text(res, &[]).await?;

    if atty::is(atty::Stream::Stdout) {
        writeln!(&mut output, "Url: {}\n", url)?;
        writeln!(
            &mut output,
            "\n{}\n{}\n{}",
            status,
            // header,
            highlight_text(&header, "yaml")?,
            highlight_text(&body, "json")?
        )?;
    } else {
        writeln!(&mut output, "{}", body)?;
    }

    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    write!(stdout, "{}", output)?;

    Ok(())
}

async fn parse() -> Result<()> {
    let theme = ColorfulTheme::default();
    // 从控制台获取输入的url
    let url: String = Input::with_theme(&theme)
        .with_prompt("Enter url")
        .interact_text()?;
    // 从控制台获取输入的name
    let name: String = Input::with_theme(&theme)
        .with_prompt("Enter name")
        .interact_text()?;

    let profile: RequestProfile = url.parse()?;
    let config = RequestConfig::new(vec![(name, profile)].into_iter().collect());
    let result = serde_yaml::to_string(&config)?;

    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    if atty::is(atty::Stream::Stdout) {
        write!(stdout, "---\n{}", highlight_text(&result, "yaml")?)?;
    } else {
        write!(stdout, "{}", result)?;
    }

    Ok(())
}
