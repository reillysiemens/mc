FROM docker.io/lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# TODO: Why does the console output report 'Compiling mc v0.0.1 (/app)'?
# The version within the container does report as the Cargo.toml version...
FROM chef AS builder 
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release

# We do not need the Rust toolchain to run the binary!
FROM fedora:latest AS runtime
WORKDIR /app
COPY --from=builder /app/target/release/mc /usr/local/bin
RUN dnf install -y java-21-openjdk-headless
ENTRYPOINT ["/usr/local/bin/mc"]

# TODO: Are these the right labels? Should annotations be used instead?
# LABEL maintainer="Reilly Siemens <reilly@tuckersiemens.com>"
# LABEL version="0.1.0"
# LABEL description="An opinionated Minecraft server"
# TODO: Add more labels.
# See https://github.com/opencontainers/image-spec/blob/main/annotations.md for more options:
# LABEL org.opencontainers.image.licenses="<license>"
# LABEL org.opencontainers.image.created="<timestamp>"
# ...