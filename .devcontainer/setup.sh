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


echo "🚀 DevContainer setup completed successfully!"