use std::process::Stdio;
use std::time::Duration;

use anyhow::{Context, Result};
use camino::Utf8Path;
use tokio::io::AsyncWriteExt;
use tokio::process::{Child, Command};
use tokio::signal::unix::{SignalKind, signal};

const GRACEFUL_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);

/// Spawn a Minecraft server as a child process.
///
/// Returns the child process handle for lifecycle management.
fn spawn(server_dir: &Utf8Path) -> Result<Child> {
    let jar_path = server_dir.join("server.jar");

    let mut cmd = Command::new("java");

    // TODO: Make settings configurable.
    cmd.args(["-Xmx4096M", "-Xms4096M", "-jar", jar_path.as_str(), "nogui"])
        .current_dir(server_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    tracing::debug!("Starting Minecraft server");

    cmd.spawn()
        .context("Failed to spawn Minecraft server process")
}

/// Attempt to gracefully shut down the server by sending the "stop" command.
///
/// Waits up to `GRACEFUL_SHUTDOWN_TIMEOUT` for the server to exit. If the
/// timeout is exceeded, the server is forcefully killed.
async fn graceful_shutdown(
    child: &mut Child,
    mut stdin: Option<tokio::process::ChildStdin>,
) -> Result<()> {
    // Try to send "stop" command via stdin
    if let Some(ref mut stdin) = stdin {
        tracing::debug!("Sending stop command to Minecraft server");
        stdin
            .write_all(b"stop\n")
            .await
            .context("Failed to send stop command to server")?;
    } else {
        anyhow::bail!("Minecraft server stdin is not available, cannot send stop command");
    }

    // Wait for graceful exit or timeout
    tokio::select! {
        result = child.wait() => {
            let status = result.context("Failed to wait on Minecraft server process")?;
            tracing::debug!("Server exited with status: {status}");
            Ok(())
        }
        () = tokio::time::sleep(GRACEFUL_SHUTDOWN_TIMEOUT) => {
            tracing::warn!(
                "Server did not exit within {} seconds, sending SIGKILL",
                GRACEFUL_SHUTDOWN_TIMEOUT.as_secs()
            );
            child.kill().await.context("Failed to kill server process")?;
            Ok(())
        }
    }
}

/// Run the Minecraft server, handling SIGTERM for graceful shutdown.
///
/// This function spawns the server and waits for either:
/// - The server to exit on its own
/// - A SIGTERM signal, which triggers graceful shutdown
pub async fn run(server_dir: &Utf8Path) -> Result<()> {
    let mut sigterm =
        signal(SignalKind::terminate()).context("Failed to register SIGTERM handler")?;
    tracing::debug!("SIGTERM handler registered");

    let mut child = spawn(server_dir)?;

    // Take stdin before we start waiting, so we can use it for graceful shutdown
    let stdin = child.stdin.take();

    tokio::select! {
        result = child.wait() => {
            let status = result.context("Failed to wait on Minecraft server process")?;
            anyhow::ensure!(
                status.success(),
                "Minecraft server exited with non-zero status: {status}"
            );
        }
        _ = sigterm.recv() => {
            tracing::debug!("Received SIGTERM signal, initiating graceful shutdown");
            graceful_shutdown(&mut child, stdin).await?;
        }
    }

    Ok(())
}
