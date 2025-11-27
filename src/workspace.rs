use std::env::set_current_dir;

use anyhow::Context;
use camino::{Utf8Path, Utf8PathBuf};
use fs_err::tokio as fs;

const EULA_PATH: &str = "eula.txt";

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

    accept_eula().await?;

    Ok(())
}

/// Ensure that eula.txt exists and contains `eula=true`.
async fn accept_eula() -> anyhow::Result<()> {
    let eula_path = Utf8Path::new(EULA_PATH);

    // Check if eula.txt exists and already contains eula=true
    if let Ok(content) = fs::read_to_string(eula_path).await
        && content.lines().any(is_eula_accepted)
    {
        tracing::debug!("EULA already accepted in {EULA_PATH}");
        return Ok(());
    }

    // TODO: Should this require an environment variable or direct the user to a link?
    // File doesn't exist or doesn't contain eula=true, (re)create it
    tracing::debug!("Accepting EULA in {EULA_PATH}");
    fs::write(eula_path, "eula=true\n").await?;

    Ok(())
}

/// Check if a line contains `eula=true` (case insensitive for the boolean value).
fn is_eula_accepted(line: &str) -> bool {
    let trimmed = line.trim();
    let Some((key, value)) = trimmed.split_once('=') else {
        return false;
    };
    key.trim() == "eula" && value.trim().eq_ignore_ascii_case("true")
}
