#/bin/bash

set -e  # Exit on any error

echo "🚀 Starting DevContainer setup..."

# SSH keys setup
echo "🔑 Setting up SSH keys permissions..."
if [ -d /home/vscode/.ssh ]; then
    sudo chown -R vscode:vscode /home/vscode/.ssh
    chmod 700 /home/vscode/.ssh
    find /home/vscode/.ssh -type f -exec chmod 600 {} \;
    echo "✅ SSH keys permissions configured"
else
    echo "ℹ️  SSH directory not found, skipping SSH setup"
fi


# Codex CLI setup
echo "📦 Setting up Codex CLI configuration..."

# Create ~/.codex directory
mkdir -p ~/.codex

# Link config.toml
if [ -f /workspaces/unity-mcp/.codex/config.toml ]; then
    ln -sf /workspaces/unity-mcp/.codex/config.toml ~/.codex/config.toml
    echo "✅ config.toml symlinked"
else
    echo "⚠️  config.toml not found"
fi

# Link prompts directory
if [ -d /workspaces/unity-mcp/.codex/prompts ]; then
    ln -sf /workspaces/unity-mcp/.codex/prompts ~/.codex/prompts
    echo "✅ prompts directory symlinked"
else
    echo "⚠️  prompts directory not found"
fi

echo "🚀 DevContainer setup completed successfully!"