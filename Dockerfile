FROM rust:1-slim AS build
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY benches ./benches
RUN cargo build --release

FROM debian:stable-slim
COPY --from=build /build/target/release/natsort_port /usr/local/bin/natsort_port
ENTRYPOINT ["natsort_port"]
CMD ["sort"]
