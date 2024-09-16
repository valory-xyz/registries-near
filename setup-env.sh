#!/bin/bash

# Clean everything
# rustup self uninstall
# rm -rf ${HOME}/.cargo 

RUSTVER="1.79"

# rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup toolchain install stable
rustup default stable

# near-cli
sudo npm install -g near-cli-rs@latest

# checks
near --version


