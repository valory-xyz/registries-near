#!/bin/bash
TARGET=$( cat Cargo.toml | grep name |  awk '{print $3}' | sed 's/"//g' ).wasm

wasm_check=$(rustup target list | grep wasm32-unknown-unknown | grep installed)
if [ "x${wasm_check}" = "x" ];
then
    rustup target add wasm32-unknown-unknown
fi

# cargo clean
rm -rf artifacts/${TARGET}

cargo build --target wasm32-unknown-unknown --profile release
cp target/wasm32-unknown-unknown/release/${TARGET} artifacts/
if [ -f artifacts/${TARGET} ];
then
    file artifacts/${TARGET}
	echo "Build ${TARGET} successful"
else
    echo "Build ${TARGET} failed!"
fi
