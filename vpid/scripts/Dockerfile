# Simple builder image for packaing debian files in a arm64 buster
FROM arm64v8/rust:1.43-slim-buster

ENV SSL_CERT_FILE  /etc/ssl/certs/ca-certificates.crt
RUN cargo install cargo-deb 

WORKDIR /project