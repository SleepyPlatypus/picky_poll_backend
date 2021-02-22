FROM rust:1.50 as build
WORKDIR /usr/src/picky_poll_backend
COPY Cargo.lock ./
COPY Cargo.toml ./
RUN mkdir .cargo
RUN cargo vendor > .cargo/config

COPY . .
RUN cargo install --path .

FROM debian:buster-slim
RUN apt-get update && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/picky_poll_backend /usr/local/bin/picky_poll_backend
CMD ["picky_poll_backend"]