#!/bin/bash

# HTTP YouTube Proxy with Rate Limiting
# Prevents HTTP 429 "Too Many Requests" errors

PROXY_BIN="./target/release/http-ytproxy"
CONFIG_FILE="config.toml"
DELAY_BETWEEN_REQUESTS=2  # seconds

echo "Starting HTTP YouTube Proxy with rate limiting..."
echo "Delay between chunked requests: ${DELAY_BETWEEN_REQUESTS}s"
echo "Chunk size: 50MB (configured in config.toml)"

# Check if binary exists
if [ ! -f "$PROXY_BIN" ]; then
    echo "Error: Proxy binary not found. Please run 'cargo build --release' first."
    exit 1
fi

# Check if config exists
if [ ! -f "$CONFIG_FILE" ]; then
    echo "Creating default config with anti-429 settings..."
    cat > "$CONFIG_FILE" << EOF
# mpv-http-ytproxy configuration - Anti-429 optimized

[proxy]
port = 12081
chunk_size = 52428800  # 50MB chunks to reduce request frequency
cert_file = "cert.pem"
key_file = "key.pem"
adaptive_chunking = true
min_chunk_size = 20971520  # 20MB minimum
max_chunk_size = 104857600  # 100MB maximum

[security]
cert_validity_days = 365

[logging]
level = "info"
log_timing = true

[performance]
http2 = true
connection_pool_size = 5  # Reduce connection pool
request_timeout = 45      # Longer timeout for large chunks
enable_compression = true
EOF
    echo "Config created: $CONFIG_FILE"
fi

# Start proxy with rate limiting wrapper
exec "$PROXY_BIN" --config "$CONFIG_FILE"
