# musl: static binary, OpenSSL included (busmatick TLS servers rustls can't negotiate)
FROM rust:1-alpine AS chef
RUN apk add --no-cache gcc musl-dev openssl-dev openssl-libs-static pkgconf \
  && cargo install cargo-chef --locked
ENV CARGO_TARGET_DIR=/target OPENSSL_STATIC=1
WORKDIR /src

FROM chef AS recipe
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=recipe /src/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --locked -p mova-server \
  && cp /target/release/mova-server /mova-server

FROM gcr.io/distroless/static-debian12:nonroot
COPY --from=builder /mova-server /mova-server
ENV TZ=America/Montevideo \
  SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt
EXPOSE 3000
ENTRYPOINT ["/mova-server"]
