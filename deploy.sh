#!/bin/bash
./build.sh
near deploy --wasmFile res/ino.wasm --accountId ninjadev_1.testnet
near call ninjadev_1.testnet new '' --account-id ninjadev_1.testnet