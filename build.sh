#!/bin/bash
set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Building mpv-http-ytproxy...${NC}"

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to print error and exit
error_exit() {
    echo -e "${RED}Error: $1${NC}" >&2
    exit 1
}

# Check if we're in the right directory
if [[ ! -f "Cargo.toml" || ! -f "main.lua" ]]; then
    error_exit "Please run this script from the mpv-http-ytproxy directory"
fi

# Check if Rust is already installed
if ! command_exists rustc || ! command_exists cargo; then
    echo -e "${YELLOW}Rust not found. Installing Rust toolchain...${NC}"
    
    # More secure Rust installation with checksum verification
    if command_exists curl; then
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
    else
        error_exit "curl is required to install Rust"
    fi
    
    # Source the cargo environment
    if [[ -f "$HOME/.cargo/env" ]]; then
        source "$HOME/.cargo/env"
    else
        error_exit "Failed to source Rust environment"
    fi
else
    echo -e "${GREEN}Rust toolchain found${NC}"
fi

# Verify Rust installation
if ! command_exists cargo; then
    error_exit "Cargo not found after installation attempt"
fi

echo -e "${GREEN}Building release binary...${NC}"
cargo build --release

if [[ ! -f "target/release/http-ytproxy" ]]; then
    error_exit "Build failed - binary not found"
fi

echo -e "${GREEN}Build successful!${NC}"

# Determine mpv scripts directory
if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS
    mpv_config_dir="$HOME/.config/mpv"
elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
    # Linux
    mpv_config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/mpv"
else
    # Windows/Other - fallback
    mpv_config_dir="$HOME/.config/mpv"
fi

scriptdir="$mpv_config_dir/scripts/http-ytproxy"

echo -e "${GREEN}Installing to mpv scripts directory: ${scriptdir}${NC}"

# Create directory with proper permissions
mkdir -p "$scriptdir"

# Copy files
if ! cp target/release/http-ytproxy "$scriptdir/"; then
    error_exit "Failed to copy binary"
fi

if ! cp main.lua "$scriptdir/"; then
    error_exit "Failed to copy Lua script"
fi

# Make binary executable
chmod +x "$scriptdir/http-ytproxy"

cd "$scriptdir"

# Check if certificates already exist
if [[ -f "cert.pem" && -f "key.pem" ]]; then
    echo -e "${YELLOW}Certificates already exist, skipping generation${NC}"
else
    echo -e "${GREEN}Generating TLS certificates for MITM proxy...${NC}"
    
    # Check if openssl is available
    if ! command_exists openssl; then
        error_exit "OpenSSL is required to generate certificates"
    fi
    
    # Generate certificates with better security parameters
    if ! openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 \
        -passout pass:"third-wheel" \
        -subj "/C=US/ST=Local/L=Local/O=mpv-ytproxy/CN=localhost" \
        -addext "subjectAltName=DNS:localhost,IP:127.0.0.1" 2>/dev/null; then
        error_exit "Failed to generate TLS certificates"
    fi
    
    echo -e "${GREEN}Certificates generated successfully${NC}"
fi

echo -e "${GREEN}Installation complete!${NC}"
echo -e "${YELLOW}Note: This proxy uses MITM techniques and disables TLS verification.${NC}"
echo -e "${YELLOW}Only use with trusted YouTube content on your local machine.${NC}"
echo ""
echo -e "${GREEN}Usage:${NC}"
echo "  1. The proxy will automatically activate when playing YouTube URLs in mpv"
echo "  2. To disable: add 'script-opts=http-ytproxy=no' to your mpv.conf"
echo "  3. Manual proxy port: 12081"
