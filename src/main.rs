use anyhow::Result;
use clap::Parser;
use std::io::Write;
use xdiff::{
    cli::{Action, Args, RunArgs},
    DiffConfig,
};

// ----------
// 1:02:01
// ----------

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    match args.action {
        Action::Run(args) => run(args).await?,
        _ => panic!("Not implemented`没有该实现 "),
    }

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
