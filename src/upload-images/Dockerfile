# Taken from https://dev.to/rogertorres/first-steps-with-docker-rust-30oi

# Rust as the base image
FROM rust:1.58-slim as build

# Create a new empty shell project
RUN USER=root cargo new --bin upload-images
WORKDIR /upload-images

# Copy our manifests
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

# Build only the dependencies to cache them
RUN cargo build --release
RUN rm src/*.rs

# Copy the source code
COPY ./src ./src

# Build for release.
RUN rm ./target/release/deps/upload_images*
RUN cargo build --release

# The final base image
FROM debian:buster-slim

# Needed by rustls-tls
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates
RUN update-ca-certificates

# Copy from the previous build
COPY --from=build /upload-images/target/release/upload-images /usr/src/upload-images

# Run the binary
CMD ["/usr/src/upload-images"]
