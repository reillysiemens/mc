use std::io::ErrorKind;

use anyhow::{Context, anyhow};
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
    // TODO: Consider using trace logging for some finer details like versions, SHA1, sizes, URLs, etc.
    pub async fn execute(&self) -> anyhow::Result<()> {
        // TODO: Should this use a default User-Agent?
        let client = Client::builder()
            .user_agent(USER_AGENT)
            .use_rustls_tls()
            .min_tls_version(tls::Version::TLS_1_3)
            .build()?;

        tracing::debug!("Fetching version manifest");
        let manifest: VersionManifest = client
            .get(VERSION_MANIFEST_URL)
            .send()
            .await?
            .json()
            .await?;

        // TODO: Consider logging whether a version is requested or is latest.
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

        tracing::debug!("Fetching version {} metadata", version.id);
        let version_metadata: VersionMetadata =
            client.get(version.url).send().await?.json().await?;

        let VersionMetadata {
            downloads: Downloads { server },
        } = version_metadata;

        match tokio::fs::read(SERVER_PATH).await {
            Ok(data) => {
                tracing::debug!("Found existing {SERVER_PATH}, verifying checksum");
                let actual = sha1_hex(&data);
                if actual == server.sha1 {
                    tracing::debug!("Checksum matches, skipping download");
                    return Ok(());
                }
                tracing::debug!(
                    "Checksum mismatch (expected: {}, actual: {})",
                    server.sha1,
                    actual
                );
            }
            Err(err) if err.kind() == ErrorKind::NotFound => {
                tracing::debug!("Existing {SERVER_PATH} not found");
            }
            Err(err) => return Err(err.into()),
        }

        // Download to temporary file first, then move on success
        let prefix: u64 = rand::random();
        let temp_path = format!("{prefix:x}-{SERVER_PATH}");
        let mut file = tokio::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp_path)
            .await
            .context("Failed to create temporary file")?;

        tracing::debug!("Fetching server version {}", version.id);
        let mut stream = client.get(server.url).send().await?.bytes_stream();

        tracing::debug!("Writing {} to {temp_path}", ByteSize(server.size));
        let mut hasher = Sha1::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            hasher.update(&chunk);
            file.write_all(&chunk).await?;
        }

        let computed = format!("{:x}", hasher.finalize());
        if computed != server.sha1 {
            tracing::error!(
                "SHA-1 checksum is invalid (expected: {}, actual: {})",
                server.sha1,
                computed
            );
            tokio::fs::remove_file(&temp_path)
                .await
                .context("Failed to cleanup temporary file")?;
            return Err(anyhow!(
                "Checksum mismatch: expected {}, got {}",
                server.sha1,
                computed
            ));
        }

        tracing::debug!("SHA-1 checksum is valid");
        tracing::debug!("Renaming {temp_path} to {SERVER_PATH}");
        tokio::fs::rename(&temp_path, SERVER_PATH)
            .await
            .context("Failed to move temporary file to final location")?;

        Ok(())
    }
}
