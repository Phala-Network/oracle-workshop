#!/bin/bash

for contract in $(ls target/ink); do
    mkdir -p "./bin/$contract"
    cp "target/ink/$contract/"*.{wasm,contract,json} "./bin/$contract"
done
