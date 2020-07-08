# Simple builder image for packaing debian files in a arm64 buster
FROM arm64v8/rust:1.43-slim-buster

RUN cargo install cargo-deb 
WORKDIR /project