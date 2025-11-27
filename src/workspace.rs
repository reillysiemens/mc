use std::{env::set_current_dir, path::PathBuf};

use anyhow::Context;

/// Prepare a workspace directory for the Minecraft server.
///
/// Creates the directory if it doesn't exist, verifies write permissions,
/// and changes the process's working directory to it.
pub async fn prepare(directory: PathBuf) -> anyhow::Result<()> {
    // Create directory if it doesn't exist
    tokio::fs::create_dir_all(&directory)
        .await
        .with_context(|| format!("Failed to create directory: {}", directory.display()))?;

    // Canonicalize path
    let directory = tokio::fs::canonicalize(&directory)
        .await
        .with_context(|| format!("Failed to canonicalize directory: {}", directory.display()))?;

    // Verify write permissions
    let metadata = tokio::fs::metadata(&directory).await?;
    if metadata.permissions().readonly() {
        anyhow::bail!("Directory is not writable: {}", directory.display());
    }

    // Change working directory
    tracing::debug!("Changing working directory to {}", directory.display());
    set_current_dir(&directory)
        .with_context(|| format!("Failed to change to directory: {}", directory.display()))?;

    Ok(())
}
