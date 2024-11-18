mod env;
mod logging;

use clap::{
    builder::styling::{AnsiColor, Styles},
    Parser,
};

// Clap v4 defaults to a colorless style. This emulates the colored v3 style.
const STYLE: Styles = Styles::styled()
    .usage(AnsiColor::Yellow.on_default().underline())
    .header(AnsiColor::Yellow.on_default().underline())
    .literal(AnsiColor::Green.on_default())
    .placeholder(AnsiColor::White.on_default());

/// Minecraft server management toolkit
#[derive(Debug, Parser)]
#[clap(version, styles=STYLE)]
pub struct Args {
    /// Control logging verbosity
    #[arg(
        long,
        value_enum,
        env = env::LOG_LEVEL,
        default_value_t = logging::LogLevel::default()
    )]
    pub log_level: logging::LogLevel,
}
