FROM rust:1.50 as build
WORKDIR /usr/src/picky_poll_backend

COPY Cargo.lock ./
COPY Cargo.toml ./
COPY cleanup/Cargo.toml cleanup/Cargo.toml
COPY service/Cargo.toml service/Cargo.toml
RUN mkdir .cargo
RUN cargo vendor > .cargo/config

COPY . .
RUN cargo build --release

FROM debian:buster-slim as cleanup
RUN apt-get update && rm -rf /var/lib/apt/lists/*
# COPY --from=cleanup-install /usr/local/cargo/bin/cleanup /usr/local/bin/picky_poll_cleanup
COPY --from=build usr/src/picky_poll_backend/target/release/cleanup /usr/local/bin/picky_poll_cleanup
ENV RUST_LOG="info"
CMD ["picky_poll_cleanup"]

FROM debian:buster-slim as service
RUN apt-get update && rm -rf /var/lib/apt/lists/*
COPY --from=build usr/src/picky_poll_backend/target/release/service /usr/local/bin/picky_poll_backend
ENV RUST_LOG="info"
CMD ["picky_poll_backend"]