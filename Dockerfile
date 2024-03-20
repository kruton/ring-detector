# Dockerfile for creating a statically-linked Rust application using docker's
# multi-stage build feature. This also leverages the docker build cache to avoid
# re-downloading dependencies if they have not changed.
FROM rust:1.76.0 AS build

# Download the target for static linking.
RUN rustup target add x86_64-unknown-linux-musl
RUN apt-get update && apt-get install -y --no-install-recommends \
    musl-tools \
    protobuf-compiler \
    cmake

# Install and cache dependencies layers
# Rather than copying everything every time, re-use cached dependency layers
# to install/build deps only when Cargo.* files change.
RUN USER=root cargo new /home/app --bin

WORKDIR /home/app

# Download the dependencies so we don't have to do this every time.
COPY Cargo.toml Cargo.lock ./
RUN echo "fn main() {}" > dummy.rs
RUN sed -i.backup -e 's#src/main.rs#dummy.rs#' -e 's#src/lib.rs#dummy.rs#' Cargo.toml
RUN cargo build --verbose --release --target x86_64-unknown-linux-musl
RUN mv Cargo.toml.backup Cargo.toml && rm -f dummy.rs

# Copy the source and build the application.
COPY . ./

RUN cargo build --verbose --bins --release --target x86_64-unknown-linux-musl

# Copy the statically-linked binary into a scratch container.
FROM scratch
LABEL org.opencontainers.image.source https://github.com/kruton/ring-detector
COPY --from=build /home/app/target/x86_64-unknown-linux-musl/release/ring-detector .
USER 1000
ENTRYPOINT ["./ring-detector"]
