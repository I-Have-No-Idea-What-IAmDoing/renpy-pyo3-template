FROM mcr.microsoft.com/devcontainers/rust:1-1-bullseye

# Install system dependencies
RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get -y install --no-install-recommends \
       python3 python3-pip wget xz-utils libssl-dev pkg-config

# Install Zig 0.13
RUN ZIG_VERSION="0.13.0" \
    && ARCH=$(uname -m) \
    && if [ "$ARCH" = "x86_64" ]; then ZIG_ARCH="x86_64"; else ZIG_ARCH="aarch64"; fi \
    && wget -q "https://ziglang.org/download/${ZIG_VERSION}/zig-linux-${ZIG_ARCH}-${ZIG_VERSION}.tar.xz" \
    && tar -xf "zig-linux-${ZIG_ARCH}-${ZIG_VERSION}.tar.xz" -C /usr/local \
    && ln -s "/usr/local/zig-linux-${ZIG_ARCH}-${ZIG_VERSION}/zig" /usr/local/bin/zig \
    && rm "zig-linux-${ZIG_ARCH}-${ZIG_VERSION}.tar.xz"

# Switch to vscode user and install cargo-zigbuild & rust targets
USER vscode
RUN cargo install cargo-zigbuild \
    && rustup target add \
       x86_64-pc-windows-gnu \
       x86_64-unknown-linux-gnu \
       aarch64-unknown-linux-gnu \
       aarch64-apple-darwin \
       x86_64-apple-darwin
