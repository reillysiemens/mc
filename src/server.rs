use anyhow::{Context, Result};
use camino::Utf8Path;
use std::process::Stdio;
use tokio::process::{Child, Command};

/// Start a Minecraft server as a child process.
pub async fn start(server_dir: &Utf8Path) -> Result<Child> {
    let jar_path = server_dir.join("server.jar");

    let mut cmd = Command::new("/usr/bin/java");

    cmd.args(["-Xmx4096M", "-Xms4096M", "-jar", jar_path.as_str(), "nogui"])
        .current_dir(server_dir)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    let child = cmd
        .spawn()
        .context("Failed to spawn Minecraft server process")?;

    Ok(child)
}
