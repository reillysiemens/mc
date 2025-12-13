use anyhow::{Context, Result};
use camino::Utf8Path;
use std::process::Stdio;
use tokio::process::Command;

/// Start a Minecraft server as a child process.
pub async fn start(server_dir: &Utf8Path) -> Result<()> {
    let jar_path = server_dir.join("server.jar");

    let mut cmd = Command::new("/usr/bin/java");

    // TODO: Make settings configurable.
    cmd.args(["-Xmx4096M", "-Xms4096M", "-jar", jar_path.as_str(), "nogui"])
        .current_dir(server_dir)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    // TODO: Maybe log the full command line at trace level?
    tracing::debug!("Starting Minecraft server with");

    let status = cmd
        .spawn()
        .context("Failed to spawn Minecraft server process")?
        .wait()
        .await
        .context("Failed to wait on Minecraft server process")?;

    anyhow::ensure!(
        status.success(),
        "Minecraft server exited with non-zero status: {status}"
    );

    Ok(())
}
