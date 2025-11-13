FROM rust:1.91.1-alpine3.22
RUN apk add --no-cache build-base libsodium-dev
WORKDIR /opt/markov
COPY . .
RUN cargo build --release
CMD ["/opt/markov/target/release/markov"]
