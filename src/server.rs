use std::{io::BufRead, process::Stdio, time::Duration};

use anyhow::{Context, Result};
use camino::Utf8PathBuf;
use tokio::{
    io::AsyncWriteExt,
    process::{Child, ChildStdin, Command},
    signal::unix::{SignalKind, signal},
    sync::mpsc,
};

/// Configuration for running a Minecraft server.
#[derive(Debug, Clone)]
pub struct Config {
    /// Path to the directory containing `server.jar`.
    pub directory: Utf8PathBuf,
    /// How long to wait for graceful shutdown before killing the server.
    pub shutdown_timeout: Duration,
    /// Minimum heap size for the JVM (`-Xms`), e.g. `1G`, `512M`.
    pub min_memory: String,
    /// Maximum heap size for the JVM (`-Xmx`), e.g. `1G`, `512M`.
    pub max_memory: String,
}

/// Spawn a Minecraft server as a child process.
///
/// Returns the child process handle for lifecycle management.
fn spawn(config: &Config) -> Result<Child> {
    let jar_path = config.directory.join("server.jar");

    let xms = format!("-Xms{}", config.min_memory);
    let xmx = format!("-Xmx{}", config.max_memory);

    let mut cmd = Command::new("java");

    cmd.args([&xms, &xmx, "-jar", jar_path.as_str(), "nogui"])
        .current_dir(&config.directory)
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    tracing::debug!("Executing: java {xms} {xmx} -jar {jar_path} nogui");

    cmd.spawn().context("Failed to spawn server process")
}

/// Read lines from stdin and send them to the channel.
///
/// Runs on a dedicated OS thread (not tokio's blocking pool) so that the
/// blocked read won't prevent tokio runtime shutdown. The thread is killed
/// when the process exits.
fn spawn_stdin_reader(tx: mpsc::Sender<String>) {
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        for line in stdin.lock().lines() {
            match line {
                Ok(line) => {
                    if tx.blocking_send(line).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
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

/// Wait for the child process to exit and log the exit code.
async fn wait_for_child(child: &mut Child) -> Result<()> {
    let status = child
        .wait()
        .await
        .context("Failed to wait on server process")?;
    if let Some(code) = status.code() {
        tracing::debug!("Server terminated with code: {}", code);
    } else {
        tracing::warn!("Server terminated by signal");
    }
    Ok(())
}

/// Gracefully shut down the server by sending "stop" and waiting for exit.
///
/// If the server doesn't exit within the timeout, it is forcefully killed.
async fn shutdown(child: &mut Child, tx: &mpsc::Sender<String>, timeout: Duration) -> Result<()> {
    if tx.send("stop".to_string()).await.is_err() {
        tracing::warn!("Failed to send stop command, channel closed");
    }

    tokio::select! {
        result = wait_for_child(child) => result,
        () = tokio::time::sleep(timeout) => {
            tracing::warn!(
                "Server did not exit within {} seconds, sending SIGKILL",
                timeout.as_secs()
            );
            child.kill().await.context("Failed to kill server process")
        }
    }
}

/// Run the Minecraft server, handling SIGTERM for graceful shutdown.
///
/// Forwards stdin to the server, allowing interactive commands. On SIGTERM,
/// sends the "stop" command for graceful shutdown.
pub async fn run(config: &Config) -> Result<()> {
    let mut sigterm =
        signal(SignalKind::terminate()).context("Failed to register SIGTERM handler")?;
    tracing::debug!("SIGTERM handler registered");

    let mut child = spawn(config)?;
    let child_stdin = child
        .stdin
        .take()
        .context("Failed to capture child stdin")?;

    // Channel for sending commands to the child's stdin.
    // Both the stdin reader and the main task (for SIGTERM) can send to this.
    let (tx, rx) = mpsc::channel::<String>(32);

    // Spawn a reader to forward stdin lines to the child process.
    spawn_stdin_reader(tx.clone());

    // Spawn a task to write commands from the channel to the child's stdin.
    tokio::spawn(async move {
        write_to_child(child_stdin, rx).await;
    });

    tokio::select! {
        result = wait_for_child(&mut child) => result,
        _ = sigterm.recv() => {
            tracing::debug!("Received SIGTERM signal, initiating graceful shutdown");
            shutdown(&mut child, &tx, config.shutdown_timeout).await
        }
    }
}
