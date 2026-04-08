#!/bin/bash
# Weaven Game Testing Environment Setup Script
# For Claude Code cloud sessions (SessionStart hook)
#
# Installs: .NET SDK 8.0 (for C# adapter tests)
#
# Note: Unity Editor runs via GameCI in GitHub Actions (requires Pro for
# headless execution; Personal licenses work through GameCI).
# This script focuses on what Claude can test directly: Rust + C#.
#
# Usage: Add to SessionStart hook in .claude/settings.json
#   or run manually: bash .claude/scripts/setup-game-testing.sh

set -euo pipefail

DOTNET_INSTALL_DIR="/opt/dotnet"

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

# ── Export PATH via CLAUDE_ENV_FILE ───────────────────────────
if [ -n "${CLAUDE_ENV_FILE:-}" ]; then
  echo "export PATH=\"$DOTNET_INSTALL_DIR:\$PATH\"" >> "$CLAUDE_ENV_FILE"
  echo "export DOTNET_ROOT=\"$DOTNET_INSTALL_DIR\"" >> "$CLAUDE_ENV_FILE"
  log "Environment variables written to CLAUDE_ENV_FILE"
else
  log "CLAUDE_ENV_FILE not set. Export PATH manually:"
  log "  export PATH=\"$DOTNET_INSTALL_DIR:\$PATH\""
fi

log "Setup complete. Unity tests run via GameCI (GitHub Actions)."
