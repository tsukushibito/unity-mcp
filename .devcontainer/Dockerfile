FROM mcr.microsoft.com/devcontainers/base:ubuntu

# Update package manager and install essential packages
RUN apt-get update && apt-get install -y \
    curl \
    wget \
    git \
    git-lfs \
    build-essential \
    software-properties-common \
    apt-transport-https \
    ca-certificates \
    gnupg \
    lsb-release \
    && rm -rf /var/lib/apt/lists/*

# Install Node.js (using NodeSource repository) and eslint
RUN curl -fsSL https://deb.nodesource.com/setup_lts.x | bash - \
    && apt-get install -y nodejs \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

RUN mkdir -p /home/vscode/.npm-global /home/vscode/.npm-cache \
    && chown -R vscode:vscode /home/vscode

ENV NPM_CONFIG_PREFIX=/home/vscode/.npm-global
ENV NPM_CONFIG_CACHE=/home/vscode/.npm-cache
ENV PATH=/home/vscode/.npm-global/bin:$PATH

# Install .NET SDK
RUN wget https://packages.microsoft.com/config/ubuntu/$(lsb_release -rs)/packages-microsoft-prod.deb -O packages-microsoft-prod.deb \
    && dpkg -i packages-microsoft-prod.deb \
    && rm packages-microsoft-prod.deb \
    && apt-get update \
    && apt-get install -y dotnet-sdk-8.0 \
    && rm -rf /var/lib/apt/lists/*

# Install protoc compiler
ARG PROTOC_VERSION=31.1
RUN apt-get update && apt-get install -y curl unzip \
    && rm -rf /var/lib/apt/lists/* \
    && ARCH=$(uname -m | sed 's/aarch64/aarch_64/;s/x86_64/x86_64/') \
    && curl -fsSL -o /tmp/protoc.zip \
    "https://github.com/protocolbuffers/protobuf/releases/download/v${PROTOC_VERSION}/protoc-${PROTOC_VERSION}-linux-${ARCH}.zip" \
    && unzip -o /tmp/protoc.zip -d /usr/local \
    && rm -f /tmp/protoc.zip \
    && protoc --version

# -----------------------------------------------------------------------------
# Install grpc_csharp_plugin (and optional protoc) from NuGet Grpc.Tools
# - Puts grpc_csharp_plugin into /usr/local/bin (PATH)
# - Also extracts Grpc.Tools' protoc. If a system protoc already exists, it
# is saved as /usr/local/bin/protoc-grpctools to avoid overwriting.
# -----------------------------------------------------------------------------
ARG GRPC_TOOLS_VERSION=2.72.0
ARG TARGETARCH
RUN set -eux; \
    curl -fsSL "https://www.nuget.org/api/v2/package/Grpc.Tools/${GRPC_TOOLS_VERSION}" -o /tmp/grpc.tools.nupkg; \
    mkdir -p /opt/grpc-tools; \
    unzip -q /tmp/grpc.tools.nupkg -d /opt/grpc-tools; \
    case "${TARGETARCH:-$(dpkg --print-architecture)}" in \
    amd64|x86_64) OS_ARCH=linux_x64 ;; \
    arm64|aarch64) OS_ARCH=linux_arm64 ;; \
    *) echo "Unsupported TARGETARCH=${TARGETARCH}"; exit 1 ;; \
    esac; \
    install -m 0755 "/opt/grpc-tools/tools/${OS_ARCH}/grpc_csharp_plugin" /usr/local/bin/grpc_csharp_plugin; \
    if [ ! -x /usr/local/bin/protoc ]; then \
    install -m 0755 "/opt/grpc-tools/tools/${OS_ARCH}/protoc" /usr/local/bin/protoc; \
    else \
    install -m 0755 "/opt/grpc-tools/tools/${OS_ARCH}/protoc" /usr/local/bin/protoc-grpctools; \
    fi; \
    rm -rf /tmp/grpc.tools.nupkg /opt/grpc-tools; \
    command -v grpc_csharp_plugin >/dev/null; \
    /usr/local/bin/protoc --version || true; \
    [ -x /usr/local/bin/protoc-grpctools ] && /usr/local/bin/protoc-grpctools --version || true

# Install GitHub CLI
RUN curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg | dd of=/usr/share/keyrings/githubcli-archive-keyring.gpg \
    && chmod go+r /usr/share/keyrings/githubcli-archive-keyring.gpg \
    && echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" | tee /etc/apt/sources.list.d/github-cli.list > /dev/null \
    && apt-get update \
    && apt-get install gh -y \
    && rm -rf /var/lib/apt/lists/*

ARG USERNAME=vscode
USER ${USERNAME}

# Install Rust toolchain for vscode user
ENV RUSTUP_HOME=/home/${USERNAME}/.rustup \
    CARGO_HOME=/home/${USERNAME}/.cargo \
    PATH=/home/${USERNAME}/.cargo/bin:/home/${USERNAME}/.local/bin:${PATH}

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --default-toolchain stable --profile default --no-modify-path \
    && rustup component add rust-analyzer rust-src clippy \
    && rustup --version && rustc --version && cargo --version && rust-analyzer --version

# Install UV (Python package manager) for vscode user
RUN curl -LsSf https://astral.sh/uv/install.sh | sh

# Install npm packages globally for vscode user
RUN npm install -g eslint \
    && npm install -g npm@latest \
    && npm install -g @openai/codex \
    && npm install -g @anthropic-ai/claude-code \
    && npm install -g @google/gemini-cli \
    && npm install -g @charmland/crush \
    && npm install -g @byterover/cipher

# Install Python tools using UV
RUN ~/.local/bin/uv tool install claude-monitor

# Set environment variables for vscode user
ENV PATH="/home/vscode/.cargo/bin:${PATH}"
ENV PATH="/home/vscode/.local/bin:${PATH}"
