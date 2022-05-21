#!/bin/bash
./build.sh
near deploy --wasmFile res/ino.wasm --accountId daonation.testnet
near call daonation.testnet new '' --account-id daonation.testnet