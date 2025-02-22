FROM rust:1.85-bullseye AS build

# create a new empty shell project
RUN USER=root cargo new --bin medibot
WORKDIR /medibot

# copy over your manifests
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

# this build step will cache your dependencies
RUN cargo build --release
RUN rm src/*.rs

# copy your source tree
COPY ./src ./src

# build for release
RUN rm ./target/release/deps/medibot*
RUN cargo build --release

# our final base
FROM debian:bullseye-slim

# copy the build artifact from the build stage
COPY --from=build /medibot/target/release/medibot .

# set the startup command to run your binary
CMD ["./medibot"]
