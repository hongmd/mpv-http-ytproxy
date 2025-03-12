#!/bin/sh
set -eux

# clone repo
git clone https://github.com/spvkgn/mpv-http-ytproxy.git

# install Rust toolchain and build
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
. $HOME/.cargo/env
export PATH=$PATH:/usr/local/cargo/bin
cargo build --release

# install lua script to mpv dir
scriptdir=~/.config/mpv/scripts/http-ytproxy
mkdir -p $scriptdir
cp -a -t $scriptdir target/release/http-ytproxy main.lua
cd $scriptdir

# generate private keys for mitm proxy (they don't matter)
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 3650 -passout pass:"third-wheel" -subj "/C=US/ST=private/L=province/O=city/CN=hostname.example.com"
echo done!
