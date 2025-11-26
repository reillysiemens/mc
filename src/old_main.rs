mod cli;
mod fetch;
mod manifest;

use std::{io, path::Path};

use clap::Parser;
use rc_zip_tokio::ReadZip;
use serde::Deserialize;

use fetch::Fetch;
use manifest::Type;

#[derive(Deserialize)]
struct ArchiveVersion {
    id: String,
}

// TODO: This function could use some extra context for the errors, especially
// the JSON deserialization ones.
//
// TODO: Is this really a good idea? Could the SHA1 sum just be checked instead?
async fn server_jar_version(path: &Path) -> io::Result<String> {
    let data = tokio::fs::read(path).await?;
    let archive = data.read_zip().await?;
    let entry = archive
        .by_name("version.json")
        .ok_or(io::Error::other("Missing version.json file in server.jar"))?;
    let data = entry.bytes().await?;
    let archive_version: ArchiveVersion = serde_json::from_reader(data.as_slice())?;
    Ok(archive_version.id)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = cli::Args::parse();
    tracing_subscriber::fmt()
        .with_max_level(args.log_level)
        .init();

    // ---- Getting the server ----

    // Has a specific version been provided via flag or env var?
    if let Some(ref version) = args.server_version {
        tracing::debug!("Launching Minecraft {version}");
    }

    // Is there a server.jar in the target directory already?
    let server = Path::new("server.jar");
    let jar_version = match server_jar_version(&server).await {
        Ok(version) => {
            tracing::debug!("Found server version {version} in {}", server.display());
            Some(version)
        }
        Err(err) => {
            tracing::debug!("Unable to find version in a server.jar: {err}");
            None
        }
    };

    use sha1::{Digest, Sha1};
    let data = tokio::fs::read("server.jar").await?;
    let now = std::time::Instant::now();
    let _digest = Sha1::digest(&data);
    let elapsed = now.elapsed();
    println!("Calculated digest in {elapsed:?}");

    // New logic?
    // Fetch manifest
    // version = manifest[latest] if version.is_none() else version
    // Go fetch version info
    // Compare checksum
    // If no match, download and replace

    // Do we need to download a new version?
    // let fetch = match (args.server_version, jar_version) {
    //     (Some(target), Some(current)) if current == target => {
    //         tracing::debug!("Found {target} in server.jar");
    //         Fetch::Ignore
    //     }
    //     (Some(target), Some(current)) => {
    //         tracing::debug!("Found version {current} in server.jar, but require {target}");
    //         Fetch::Version(target)
    //     }
    //     (Some(target), None) => {
    //         // TODO: Should this have some debugging too?
    //         Fetch::Version(target)
    //     }
    //     (None, Some(current)) => {
    //         // A current server version was found.
    //         // - If --no-update was requested, Fetch::Ignore.
    //         // What is the latest available? For this we need the manifest.
    //         //   - If it matches then Fetch::Ignore.
    //         //   - If it doesn't match, Fetch::Latest(Type::Release).
    //         todo!()
    //     }
    //     (None, None) => {
    //         tracing::debug!("No current or target server version present");
    //         Fetch::Latest(Type::Release)
    //     }
    // };

    // Download a new version...
    // This should probably be done to temp storage and switched at the last moment.

    // ---- Running the server ----

    // Is there a EULA in the current directory?
    // - Ensure it has been accepted.
    // - Always write eula=true to a file?
    // - Should this require an environment variable or direct the user to a link?

    // Start the Minecraft server.
    // - Wrap the child process in something that interrupts SIGTERM and tries
    //   to cleanly shutdown.

    Ok(())
}
