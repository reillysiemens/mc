use std::env::set_current_dir;

use anyhow::Context;
use camino::{Utf8Path, Utf8PathBuf};
use fs_err::tokio as fs;

/// Prepare a workspace directory for the Minecraft server.
///
/// Creates the directory if it doesn't exist, verifies write permissions,
/// and changes the process's working directory to it.
pub async fn prepare(directory: &Utf8Path) -> anyhow::Result<()> {
    // Create directory if it doesn't exist
    fs::create_dir_all(directory).await?;

    // Canonicalize path
    let directory: Utf8PathBuf = fs::canonicalize(directory).await?.try_into()?;

    // Verify write permissions
    let metadata = fs::metadata(&directory).await?;
    if metadata.permissions().readonly() {
        anyhow::bail!("Directory is not writable: {directory}");
    }

    // Change working directory
    tracing::debug!("Changing working directory to {directory}");
    set_current_dir(&directory)
        .with_context(|| format!("Failed to change to directory: {directory}"))?;

    Ok(())
}
