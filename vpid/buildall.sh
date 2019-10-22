#!/bin/bash

ARCHS=( "armv7-unknown-linux-gnueabihf" "armv7-unknown-linux-musleabihf" "aarch64-unknown-linux-musl" "aarch64-unknown-linux-gnu" )


echo "Vpi cross compilation script"

for i in "${ARCHS[@]}"
do
    echo "Building for Release $i"
    cross build --target=$i --release
done

