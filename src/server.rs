use std::io::BufRead;
use std::process::Stdio;
use std::time::Duration;

use anyhow::{Context, Result};
use camino::Utf8Path;
use tokio::io::AsyncWriteExt;
use tokio::process::{Child, ChildStdin, Command};
use tokio::signal::unix::{SignalKind, signal};
use tokio::sync::mpsc;

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

/// Read lines from stdin and send them to the channel.
///
/// This runs on a dedicated OS thread (not tokio's blocking pool) so that
/// terminating it doesn't block runtime shutdown. The thread will be killed
/// when the process exits.
fn spawn_stdin_reader(tx: mpsc::Sender<String>) {
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        let reader = stdin.lock();

        for line in reader.lines() {
            match line {
                Ok(line) => {
                    // Use blocking_send since we're on a std thread, not async
                    if tx.blocking_send(line).is_err() {
                        break; // Receiver dropped
                    }
                }
                Err(_) => break,
            }
        }
        tracing::debug!("Stdin reader thread exiting");
    });
}

/// Write lines from the channel to the child's stdin.
///
/// Runs until the channel is closed or writing fails.
async fn write_to_child(mut child_stdin: ChildStdin, mut rx: mpsc::Receiver<String>) {
    while let Some(line) = rx.recv().await {
        if child_stdin
            .write_all(format!("{line}\n").as_bytes())
            .await
            .is_err()
        {
            break;
        }
    }
}

/// Run the Minecraft server, handling SIGTERM for graceful shutdown.
///
/// Forwards stdin to the server, allowing interactive commands. On SIGTERM,
/// sends the "stop" command for graceful shutdown.
pub async fn run(server_dir: &Utf8Path) -> Result<()> {
    let mut sigterm =
        signal(SignalKind::terminate()).context("Failed to register SIGTERM handler")?;
    tracing::debug!("SIGTERM handler registered");

    let mut child = spawn(server_dir)?;
    let child_stdin = child
        .stdin
        .take()
        .context("Failed to capture child stdin")?;

    // Channel for sending commands to the child's stdin.
    // Both the stdin reader and the main task (for SIGTERM) can send to this.
    let (tx, rx) = mpsc::channel::<String>(32);

    // Spawn a dedicated thread for reading stdin. Using a raw std::thread
    // (rather than tokio's blocking pool) ensures that the blocked read
    // won't prevent runtime shutdown.
    spawn_stdin_reader(tx.clone());

    // Spawn task: channel -> child stdin
    tokio::spawn(async move {
        write_to_child(child_stdin, rx).await;
    });

    let result = tokio::select! {
        result = child.wait() => {
            let status = result.context("Failed to wait on Minecraft server process")?;
            anyhow::ensure!(
                status.success(),
                "Minecraft server exited with non-zero status: {status}"
            );
            tracing::debug!("Server exited with status: {status}");
            Ok(())
        }
        _ = sigterm.recv() => {
            tracing::debug!("Received SIGTERM signal, initiating graceful shutdown");

            // Send stop command through the channel
            if tx.send("stop".to_string()).await.is_err() {
                tracing::warn!("Failed to send stop command, channel closed");
            }

            // Wait for graceful exit or force kill after timeout
            tokio::select! {
                result = child.wait() => {
                    let status = result.context("Failed to wait on Minecraft server process")?;
                    tracing::debug!("Server exited with status: {status}");
                }
                () = tokio::time::sleep(GRACEFUL_SHUTDOWN_TIMEOUT) => {
                    tracing::warn!(
                        "Server did not exit within {} seconds, sending SIGKILL",
                        GRACEFUL_SHUTDOWN_TIMEOUT.as_secs()
                    );
                    child.kill().await.context("Failed to kill server process")?;
                }
            }
            Ok(())
        }
    };

    result
}
