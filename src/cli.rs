mod env;
mod logging;

use clap::{
    Parser,
    builder::styling::{AnsiColor, Effects, Styles},
};

// Use a cargo-inspired colorscheme.
const STYLE: Styles = Styles::styled()
    .usage(AnsiColor::BrightGreen.on_default().effects(Effects::BOLD))
    .header(AnsiColor::BrightGreen.on_default().effects(Effects::BOLD))
    .literal(AnsiColor::BrightCyan.on_default().effects(Effects::BOLD))
    .placeholder(AnsiColor::Cyan.on_default());

/// Minecraft server management toolkit
#[derive(Debug, Parser)]
#[clap(version, styles=STYLE)]
pub struct Args {
    /// Control logging level
    #[arg(
        long,
        value_enum,
        env = env::LOG_LEVEL,
        default_value_t = logging::LogLevel::default()
    )]
    pub log_level: logging::LogLevel,

    // https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/struct.SubscriberBuilder.html#method.with_env_filter
    /// Control logging filter, may override verbosity
    #[arg(long, env = env::LOG_FILTER, default_value = logging::DEFAULT_FILTER )]
    pub log_filter: String,

    /// Server version
    #[arg(long, env = env::SERVER_VERSION)]
    pub server_version: Option<String>,
}
