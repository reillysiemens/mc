use std::io::ErrorKind;

use anyhow::anyhow;
use bytesize::ByteSize;
use futures_util::StreamExt;
use reqwest::{Client, tls};
use sha1::{Digest, Sha1};
use tokio::io::AsyncWriteExt;

use crate::manifest::{Downloads, Type, VERSION_MANIFEST_URL, VersionManifest, VersionMetadata};

static SERVER_PATH: &str = "server.jar";

static USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

fn sha1_hex(data: &[u8]) -> String {
    format!("{:x}", Sha1::digest(data))
}

#[derive(Debug)]
pub enum Fetch {
    Version(String),
    Latest(Type),
}

impl Fetch {
    // TODO: Consider downloading this to temporary storage first and then moving when successful.
    pub async fn execute(&self) -> anyhow::Result<()> {
        // TODO: Should this use a default User-Agent?
        let client = Client::builder()
            .user_agent(USER_AGENT)
            .use_rustls_tls()
            .min_tls_version(tls::Version::TLS_1_3)
            .build()?;

        tracing::debug!("Fetching Minecraft version manifest");
        let manifest: VersionManifest = client
            .get(VERSION_MANIFEST_URL)
            .send()
            .await?
            .json()
            .await?;

        let version = match self {
            Fetch::Version(version) => manifest
                .version(version)
                .ok_or(anyhow!("No such version: {version}"))?,
            Fetch::Latest(r#type) => match r#type {
                Type::Release => manifest.version(&manifest.latest.release).ok_or(anyhow!(
                    "Latest release is inexplicably missing from the manifest"
                ))?,
                _ => unimplemented!("Other server types not yet implemented"),
            },
        };

        tracing::debug!("Fetching Minecraft version metadata");
        let version_metadata: VersionMetadata =
            client.get(version.url).send().await?.json().await?;

        let VersionMetadata {
            downloads: Downloads { server },
        } = version_metadata;

        match tokio::fs::read(SERVER_PATH).await {
            Ok(data) => {
                if sha1_hex(&data) == server.sha1 {
                    tracing::debug!("Skipping download. Server exists and matches checksum");
                    return Ok(());
                } else {
                    tracing::debug!("Server exists, but does not match checksum");
                }
            }
            Err(err) => match err.kind() {
                ErrorKind::NotFound => tracing::debug!("Existing server not found"),
                _ => {
                    tracing::error!("Unexpected I/O error");
                    Err(err)?
                }
            },
        };

        // TODO: Create random temporary storage location?
        let mut file = tokio::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(SERVER_PATH)
            .await?;

        tracing::debug!("Fetching Minecraft server");
        let mut stream = client.get(server.url).send().await?.bytes_stream();

        tracing::debug!("Writing {} to disk", ByteSize(server.size));
        while let Some(chunk) = stream.next().await {
            file.write_all(&chunk?).await?;
        }

        tracing::debug!("Validating SHA-1 checksum");
        let data = tokio::fs::read(SERVER_PATH).await?;
        if sha1_hex(&data) == server.sha1 {
            tracing::debug!("SHA-1 checksum is valid");
            Ok(())
        } else {
            tracing::error!("SHA-1 checksum is invalid");
            Err(anyhow!("Checksum mismatch"))
        }
    }
}
