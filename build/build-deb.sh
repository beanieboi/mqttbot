#!/bin/bash
set -e

# Check if version is provided
if [ -z "$1" ]; then
    echo "Usage: $0 <version>"
    echo "Example: $0 1.2.3"
    exit 1
fi

VERSION="$1"
echo "Building mqttbot .deb package version $VERSION completely in Podman..."
echo "Building in Podman container..."

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_ROOT"

cat > build/Dockerfile.build << EOF
FROM rust:1.87-bookworm

# Install required packages
RUN dpkg --add-architecture amd64 && \\
    apt-get update && apt-get install -y \\
    dpkg-dev cmake gcc-x86-64-linux-gnu \\
    libssl-dev:amd64 pkg-config \\
    && rm -rf /var/lib/apt/lists/*

# Add x86_64 target for cross-compilation
RUN rustup target add x86_64-unknown-linux-gnu

WORKDIR /app
COPY . .

# Set environment variables for cross-compilation
ENV CC_x86_64_unknown_linux_gnu=x86_64-linux-gnu-gcc
ENV CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-linux-gnu-gcc
ENV PKG_CONFIG_ALLOW_CROSS=1
ENV PKG_CONFIG_PATH_x86_64_unknown_linux_gnu=/usr/lib/x86_64-linux-gnu/pkgconfig
ENV OPENSSL_LIB_DIR=/usr/lib/x86_64-linux-gnu
ENV OPENSSL_INCLUDE_DIR=/usr/include/x86_64-linux-gnu

RUN cargo build --release --target x86_64-unknown-linux-gnu

RUN mkdir -p debian/usr/bin/
RUN cp target/x86_64-unknown-linux-gnu/release/mqttbot debian/usr/bin/

ARG VERSION=1.0.0

RUN sed -i "s/^Version: .*/Version: \${VERSION}/" debian/DEBIAN/control

RUN dpkg-deb --build debian mqttbot_${VERSION}_amd64.deb

RUN mkdir -p /dist && cp mqttbot_${VERSION}_amd64.deb /dist/
EOF

podman build -f build/Dockerfile.build --build-arg VERSION=$VERSION -t mqttbot-builder .
podman run --rm -v "$(pwd)/dist:/dist" mqttbot-builder cp /app/mqttbot_${VERSION}_amd64.deb /dist/

rm build/Dockerfile.build

echo ""
echo "Package built successfully: ../dist/mqttbot_${VERSION}_amd64.deb"
echo ""
echo "To install on your server:"
echo "1. Copy the .deb file to your server: scp ../dist/mqttbot_${VERSION}_amd64.deb user@server:/tmp/"
echo "2. Install: sudo dpkg -i /tmp/mqttbot_${VERSION}_amd64.deb"
echo "3. Configure: sudo nano /etc/mqttbot/mqttbot.env"
echo "4. Start: sudo systemctl start mqttbot"
echo "5. Check: sudo systemctl status mqttbot"
