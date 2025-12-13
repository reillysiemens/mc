use std::env::set_current_dir;

use anyhow::Context;
use camino::{Utf8Path, Utf8PathBuf};
use fs_err::tokio as fs;
use jiff::Zoned;

const EULA_PATH: &str = "eula.txt";
const EULA_HEADER: &str = "By changing the setting below to TRUE you are indicating your agreement to our EULA (https://aka.ms/MinecraftEULA).";

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
    let date = Zoned::now().strftime("%a %b %d %H:%M:%S %Z %Y");
    let content = format!("#{EULA_HEADER}\n#{date}\neula=true\n");
    fs::write(eula_path, &content).await?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test]
    fn test_is_eula_accepted_exact_match() {
        assert!(is_eula_accepted("eula=true"));
    }

    #[test_case("eula=TRUE" ; "all uppercase")]
    #[test_case("eula=True" ; "capitalized")]
    #[test_case("eula=TrUe" ; "mixed case")]
    #[test_case("eula=tRuE" ; "another mixed case")]
    fn test_is_eula_accepted_case_insensitive(given: &str) {
        assert!(is_eula_accepted(given));
    }

    #[test_case("  eula=true  " ; "leading and trailing whitespace")]
    #[test_case("eula  =  true" ; "whitespace around equals")]
    #[test_case("  eula  =  true  " ; "whitespace everywhere")]
    fn test_is_eula_accepted_with_whitespace(given: &str) {
        assert!(is_eula_accepted(given));
    }

    #[test_case("eula=false" ; "lowercase false")]
    #[test_case("eula=FALSE" ; "uppercase false")]
    fn test_is_eula_accepted_false_value(given: &str) {
        assert!(!is_eula_accepted(given));
    }

    #[test_case("license=true" ; "wrong key license")]
    #[test_case("accept=true" ; "wrong key accept")]
    fn test_is_eula_accepted_wrong_key(given: &str) {
        assert!(!is_eula_accepted(given));
    }

    #[test_case("eula true" ; "space instead of equals")]
    #[test_case("eulatrue" ; "no separator")]
    fn test_is_eula_accepted_no_equals(given: &str) {
        assert!(!is_eula_accepted(given));
    }

    #[test_case("" ; "empty string")]
    #[test_case("=" ; "just equals")]
    #[test_case("eula=" ; "missing value")]
    #[test_case("=true" ; "missing key")]
    fn test_is_eula_accepted_empty_or_invalid(given: &str) {
        assert!(!is_eula_accepted(given));
    }
}
