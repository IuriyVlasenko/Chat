#!/usr/bin/env bash
set -euo pipefail

APP_DIR="${APP_DIR:-/opt/chat}"
BIN_NAME="${BIN_NAME:-Chat}"

echo "Building release..."
cargo build --release

echo "Installing binary..."
sudo mkdir -p "$APP_DIR/target/release"
sudo cp "target/release/$BIN_NAME" "$APP_DIR/target/release/$BIN_NAME"

if [ -f ".env" ]; then
  echo "Installing .env..."
  sudo cp ".env" "$APP_DIR/.env"
fi

echo "Reloading systemd..."
sudo systemctl daemon-reload

echo "Restarting service..."
sudo systemctl restart chat.service

echo "Done."
