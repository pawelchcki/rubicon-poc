ARG RUST_VERSION=1.70.0
FROM rust:${RUST_VERSION}-slim-bullseye AS build
ARG APP_NAME
WORKDIR /app

COPY rust-toolchain.toml .
RUN cargo build || true
COPY Cargo.toml .
COPY Cargo.lock .
RUN mkdir -p src && touch src/lib.rs
RUN cargo build || true
COPY .cargo .cargo
COPY *.json .
COPY src/ src/
RUN cargo build --release
RUN cargo build



FROM scratch AS final
COPY --from=build /app/target/x86_64-unknown-linux-none/debug/librubicon_poc.so debug/librubicon_poc.so
COPY --from=build /app/target/x86_64-unknown-linux-none/release/librubicon_poc.so release/librubicon_poc.so
