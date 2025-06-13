# Debian Package Creation

This directory contains all the files needed to create a .deb package for mqttbot.

## Building the Package

All build scripts are located in the `build/` directory. The compiled package will be output to `dist/`.

### Option 1: Complete Podman Build (Recommended):
```bash
build/build-deb-complete.sh
```

### Option 2: On Linux (with dpkg-deb installed):
```bash
build/build-deb.sh
```

### Option 3: On macOS using Podman with cross-compilation:
```bash
build/build-deb-podman.sh
```

### Option 4: Cross-compile setup on macOS (if Option 3 fails):
```bash
# Install cross-compilation toolchain
brew install filosottile/musl-cross/musl-cross
rustup target add x86_64-unknown-linux-musl

# Build with musl (static linking, no OpenSSL issues)
cargo build --release --target x86_64-unknown-linux-musl
cp target/x86_64-unknown-linux-musl/release/mqttbot debian/usr/bin/

# Then use Podman to build package
podman run --rm -v "$(pwd):/work" -w /work debian:bookworm-slim \
  bash -c "apt-get update && apt-get install -y dpkg-dev && dpkg-deb --build debian mqttbot_1.0.0_amd64.deb"
```

### Manual build process:
```bash
# 1. Build the Rust binary
cargo build --release

# 2. Copy binary to debian structure
cp target/release/mqttbot debian/usr/bin/

# 3. Build the package
dpkg-deb --build debian mqttbot_1.0.0_amd64.deb
```

## Installation on Server

1. Copy the .deb file to your server:
   ```bash
   scp dist/mqttbot_1.0.0_amd64.deb user@your-server:/tmp/
   ```

2. Install the package:
   ```bash
   sudo dpkg -i /tmp/mqttbot_1.0.0_amd64.deb
   ```

3. Edit the configuration file:
   ```bash
   sudo nano /etc/mqttbot/mqttbot.env
   ```

   Update with your actual credentials:
   - MQTT_HOST, MQTT_USERNAME, MQTT_PASSWORD
   - Adjust REFRESH_INTERVAL if needed

4. Start and enable the service:
   ```bash
   sudo systemctl start mqttbot
   sudo systemctl enable mqttbot
   ```

5. Check the service status:
   ```bash
   sudo systemctl status mqttbot
   sudo journalctl -u mqttbot -f
   ```

## Package Contents

- `/usr/bin/mqttbot` - The main binary
- `/etc/mqttbot/mqttbot.env` - Configuration file with environment variables
- `/lib/systemd/system/mqttbot.service` - Systemd service file
- Creates `mqttbot` system user for running the service

## Uninstalling

```bash
sudo dpkg -r mqttbot
```

This will stop the service, disable it, and remove the package files (but preserve the config file).
