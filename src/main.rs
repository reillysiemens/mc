use clap::Parser;

mod cli;

#[tokio::main]
async fn main() {
    let args = cli::Args::parse();
    tracing_subscriber::fmt()
        .with_max_level(args.log_level)
        .init();
    tracing::info!("Hello, World!");
}
