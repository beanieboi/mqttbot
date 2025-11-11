FROM rust:trixie AS builder
WORKDIR /usr/src/mqttbot
RUN apt-get update && \
    apt-get install -y --no-install-recommends cmake && \
    rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM debian:trixie-slim
# Install runtime dependencies (SSL/TLS libraries for MQTT)
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 && \
    rm -rf /var/lib/apt/lists/*

RUN useradd -m -u 1000 mqttbot
COPY --from=builder /usr/src/mqttbot/target/release/mqttbot /usr/local/bin/mqttbot
RUN chown mqttbot:mqttbot /usr/local/bin/mqttbot
USER mqttbot

CMD ["/usr/local/bin/mqttbot"]
