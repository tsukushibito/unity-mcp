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

# Update npm to the latest version
echo "🔄 Updating npm to the latest version..."
npm install -g npm@latest
echo "✅ npm updated successfully"

# Setup Claude Code
echo "Setting up Claude Code..."
npm install -g @anthropic-ai/claude-code
echo "Claude Code setup complete."

# Setup Gemini CLI
echo "Setting up Gemini CLI..."
npm install -g @google/gemini-cli
echo "Gemini CLI setup complete."

# Setup Charm Crush
echo "Setting up Charm Crush..."
npm install -g @charmland/crush
echo "Charm Crush setup complete."

# Claude Monitor setup
echo "🖥️ Setting up Claude Monitor..."
uv tool install claude-monitor
echo "Claude Monitor setup complete."

# Install Cipher MCP
echo "🔐 Installing Cipher MCP..."
npm install -g @byterover/cipher
echo "Cipher MCP installation complete."

echo "🚀 DevContainer setup completed successfully!"