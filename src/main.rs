mod cli;
mod fetch;
mod manifest;
mod server;
mod workspace;

use std::env::current_dir;

use clap::Parser;

use fetch::Fetch;
use manifest::Type;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = cli::Args::parse();
    tracing_subscriber::fmt()
        .with_max_level(args.log_level)
        .with_env_filter(EnvFilter::try_new(args.log_filter)?)
        .init();

    // ---- Initial workspace preparation ----

    let directory = args.directory.unwrap_or(current_dir()?.try_into()?);
    workspace::prepare(&directory).await?;

    // ---- Getting the server ----

    let fetch = match args.server_version {
        Some(version) => Fetch::Version(version),
        None => Fetch::Latest(Type::Release),
    };

    fetch.execute().await?;

    // ---- Running the server ----

    // TODO: Wrap the child process in something that interrupts SIGTERM and
    // tries to cleanly shutdown.
    server::start(&directory).await?;

    Ok(())
}
