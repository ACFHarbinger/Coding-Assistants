# Dev Container image: Node.js base plus the Rust and Android/Kotlin
# toolchains this Tauri app needs.
FROM node:20-slim

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y \
    curl \
    git \
    build-essential \
    pkg-config \
    libwebkit2gtk-4.1-dev \
    libssl-dev \
    librsvg2-dev \
    openjdk-21-jdk-headless \
    && rm -rf /var/lib/apt/lists/*

# Rust toolchain (Tauri backend)
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
ENV PATH="/root/.cargo/bin:${PATH}"

# Create a non-root user for development
RUN useradd -m -s /bin/bash vscode
USER vscode
WORKDIR /app

CMD ["/bin/bash"]
