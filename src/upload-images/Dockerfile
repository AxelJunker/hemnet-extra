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
RUN cargo build
RUN rm src/*.rs

# Copy the source code
COPY ./src ./src

# Build for release.
# Don't use `--release` to get readable backtraces in Docker
RUN rm ./target/debug/deps/upload_images*
RUN cargo build

# The final base image
FROM debian:buster-slim

# Copy from the previous build
COPY --from=build /upload-images/target/debug/upload-images /usr/src/upload-images

# Run the binary
CMD ["/usr/src/upload-images"]
