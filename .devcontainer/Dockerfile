FROM mcr.microsoft.com/devcontainers/base:ubuntu

# Update package manager and install essential packages
RUN apt-get update && apt-get install -y \
    curl \
    wget \
    git \
    build-essential \
    software-properties-common \
    apt-transport-https \
    ca-certificates \
    gnupg \
    lsb-release \
    && rm -rf /var/lib/apt/lists/*

# Switch to vscode user for user-specific installations
USER vscode

# Install .NET SDK
RUN wget https://packages.microsoft.com/config/ubuntu/$(lsb_release -rs)/packages-microsoft-prod.deb -O packages-microsoft-prod.deb \
    && dpkg -i packages-microsoft-prod.deb \
    && rm packages-microsoft-prod.deb \
    && apt-get update \
    && apt-get install -y dotnet-sdk-8.0 \
    && rm -rf /var/lib/apt/lists/*

# Install Node.js (using NodeSource repository) and eslint
RUN curl -fsSL https://deb.nodesource.com/setup_lts.x | bash - \
    && apt-get install -y nodejs \
    && rm -rf /var/lib/apt/lists/* \
    && npm install -g eslint \
    && npm install -g npm@latest

# Install protoc compiler
RUN apt-get update && apt-get install -y protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Install GitHub CLI
RUN curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg | dd of=/usr/share/keyrings/githubcli-archive-keyring.gpg \
    && chmod go+r /usr/share/keyrings/githubcli-archive-keyring.gpg \
    && echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" | tee /etc/apt/sources.list.d/github-cli.list > /dev/null \
    && apt-get update \
    && apt-get install gh -y \
    && rm -rf /var/lib/apt/lists/*

# Install Rust for vscode user
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
    && echo 'source $HOME/.cargo/env' >> ~/.bashrc

# Install UV (Python package manager) for vscode user
RUN curl -LsSf https://astral.sh/uv/install.sh | sh

# Install npm packages globally for vscode user
RUN npm install -g @anthropic-ai/claude-code \
    && npm install -g @google/gemini-cli \
    && npm install -g @charmland/crush \
    && npm install -g @byterover/cipher

# Install Python tools using UV
RUN ~/.local/bin/uv tool install claude-monitor

# Set environment variables for vscode user
ENV PATH="/home/vscode/.cargo/bin:${PATH}"
ENV PATH="/home/vscode/.local/bin:${PATH}"
