#!/bin/bash
# Weaven Game Testing Environment Setup Script
# For Claude Code cloud sessions (SessionStart hook)
#
# Installs: .NET SDK 8.0, Unity 6000.3 LTS (headless), AltTester Driver
# Usage: Add to SessionStart hook in .claude/settings.json
#   or run manually: bash .claude/scripts/setup-game-testing.sh

set -euo pipefail

UNITY_VERSION="6000.3.12f1"
UNITY_CHANGESET="fca03ac9b0d5"
UNITY_INSTALL_DIR="/opt/unity"
# Note: --strip-components=1 removes top-level "Editor/" dir from tar
# so Unity binary ends up at $UNITY_INSTALL_DIR/Unity (not Editor/Unity)
DOTNET_INSTALL_DIR="/opt/dotnet"
ALTTESTER_PROJECT_DIR="/opt/alttester-tests"

log() { echo "[setup-game-testing] $*"; }

# ── .NET SDK 8.0 ──────────────────────────────────────────────
if [ ! -f "$DOTNET_INSTALL_DIR/dotnet" ]; then
  log "Installing .NET SDK 8.0..."
  wget -q https://dot.net/v1/dotnet-install.sh -O /tmp/dotnet-install.sh
  chmod +x /tmp/dotnet-install.sh
  /tmp/dotnet-install.sh --channel 8.0 --install-dir "$DOTNET_INSTALL_DIR" 2>&1 | tail -3
  rm -f /tmp/dotnet-install.sh
  log ".NET SDK installed: $($DOTNET_INSTALL_DIR/dotnet --version)"
else
  log ".NET SDK already installed: $($DOTNET_INSTALL_DIR/dotnet --version)"
fi

# ── Unity Editor (headless Linux) ─────────────────────────────
if [ ! -f "$UNITY_INSTALL_DIR/Unity" ]; then
  log "Downloading Unity $UNITY_VERSION..."
  UNITY_URL="https://download.unity3d.com/download_unity/${UNITY_CHANGESET}/LinuxEditorInstaller/Unity-${UNITY_VERSION}.tar.xz"
  wget -q --show-progress "$UNITY_URL" -O /tmp/Unity.tar.xz 2>&1 | tail -3

  log "Extracting Unity (this may take a few minutes)..."
  mkdir -p "$UNITY_INSTALL_DIR"
  tar -xf /tmp/Unity.tar.xz -C "$UNITY_INSTALL_DIR" --strip-components=1
  rm -f /tmp/Unity.tar.xz
  log "Unity installed: $UNITY_INSTALL_DIR/Unity"
else
  log "Unity already installed at $UNITY_INSTALL_DIR"
fi

# ── AltTester Driver (NUnit test project) ─────────────────────
if [ ! -d "$ALTTESTER_PROJECT_DIR/AltTesterTests" ]; then
  log "Setting up AltTester test project..."
  export PATH="$DOTNET_INSTALL_DIR:$PATH"
  mkdir -p "$ALTTESTER_PROJECT_DIR"
  cd "$ALTTESTER_PROJECT_DIR"
  dotnet new nunit -n AltTesterTests 2>&1 | tail -3
  cd AltTesterTests
  dotnet add package AltTester-Driver --version 2.2.5 2>&1 | tail -3
  dotnet build 2>&1 | tail -3
  log "AltTester test project ready at $ALTTESTER_PROJECT_DIR/AltTesterTests"
else
  log "AltTester test project already exists"
fi

# ── Export PATH via CLAUDE_ENV_FILE ───────────────────────────
if [ -n "${CLAUDE_ENV_FILE:-}" ]; then
  echo "export PATH=\"$UNITY_INSTALL_DIR:$DOTNET_INSTALL_DIR:\$PATH\"" >> "$CLAUDE_ENV_FILE"
  echo "export UNITY_PATH=\"$UNITY_INSTALL_DIR/Unity\"" >> "$CLAUDE_ENV_FILE"
  echo "export DOTNET_ROOT=\"$DOTNET_INSTALL_DIR\"" >> "$CLAUDE_ENV_FILE"
  log "Environment variables written to CLAUDE_ENV_FILE"
else
  log "CLAUDE_ENV_FILE not set. Export PATH manually:"
  log "  export PATH=\"$UNITY_INSTALL_DIR:$DOTNET_INSTALL_DIR:\$PATH\""
fi

log "Setup complete."
