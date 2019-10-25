#!/bin/bash

#ARCHS=( "armv7-unknown-linux-gnueabihf" "armv7-unknown-linux-musleabihf" "aarch64-unknown-linux-musl" "aarch64-unknown-linux-gnu" )
ARCHS=( "arm-unknown-linux-musleabihf" "armv7-unknown-linux-musleabihf" "aarch64-unknown-linux-musl")

echo "Vpi cross compilation script..."
echo "==============================="
printf "Check cross tools..."
cargo install --list | grep cross
if [ $? -ne 0 ] ; then
    echo "not installed. force install"  
    cargo install cross --force
else
    echo "Installed!"
fi
printf "Check cargo-deb tool..."
cargo install --list | grep cargo-deb
if [ $? -ne 0 ] ; then
    echo "not installed. force install"
    cargo install cargo-deb --force
else
    echo "Installed!"
fi

for i in "${ARCHS[@]}"
do
    echo "Building for Release $i"
    cross build --target=$i --release
done

echo "==============================="
exit 0

