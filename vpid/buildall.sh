#!/bin/bash

if [[ "$1" == "travis" ]] ; then
    ARCHS=( "armv7-unknown-linux-gnueabihf" "armv7-unknown-linux-musleabihf" "aarch64-unknown-linux-musl" "aarch64-unknown-linux-gnu" "arm-unknown-linux-musleabihf" "arm-unknown-linux-gnueabihf")
else
    ARCHS=( "armv7-unknown-linux-musleabihf" "aarch64-unknown-linux-musl")
fi

if [[ "$1" == "clean" ]] ; then
    echo "Clean all build artifacts and bodcker images..."
    cargo clean
    docker rmi $(docker images "rustembedded/cross"  --format "{{.ID}}")
    docker rmi $(docker images "arm64v8/rust"  --format "{{.ID}}")
    docker rmi vpi-packager
    docker image prune
    exit 0
fi

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

if [ "$1" == "travis" ] ; then
    printf "Check for cargo-deb ..."
    cargo install --list | grep cargo-deb
    if [ $? -ne 0 ] ; then
        echo "not installed. force install"  
        cargo install cargo-deb --force
    else
        echo "Installed!"
    fi
else
    printf "Check docker image for packager tool..."
    docker images | grep vpi-packager
    if [ $? -ne 0 ] ; then
        echo "not installed. Building docker image this take a while "
        pushd scripts
            docker build -t vpi-packager .
        popd
    else
        echo "Installed!"
    fi
fi

echo "Building release versions...."
for i in "${ARCHS[@]}"
do
    echo "Building for Release $i"
    cross build --target=$i --release
done

mv vpid/Cargo.toml vpid/Cargo.toml.bup

for i in "${ARCHS[@]}"
do
    echo "Packaging deb for $i"
    cat vpid/Cargo.toml.bup > vpid/Cargo.toml
    # Create a dynamic manifest for 
    cat <<EOF >> vpid/Cargo.toml

[package.metadata.deb]
name = "vpid"
maintainer = "LDV"
copyright = "2019-2020, LDV"
#license-file = ["LICENSE", "3"]
depends = "$auto, systemd"
extended-description = """\
User-space daemon for VPi mini board for controling the buttons and perfiphericals of the board."""
section = "utils"
priority = "optional"
assets = [
    ["../target/$i/release/vpid", "usr/bin/", "755"],
	["../target/$i/release/vpidctl", "usr/bin/","775"],
	["../assets/vpid.yml","etc/vpid/","666"],
    ["../assets/vpid.service","lib/systemd/system/","644"]
]
EOF
    #cross deb --no-build --target=$i --verbose --manifest-path=./vpid/Cargo.toml
    if [ "$1" == "travis" ] ; then
        cross deb -- version
        strip target/$i/release/vpidctl
        cross deb --no-build --verbose --target=$i --manifest-path=./vpid/Cargo.toml
    else
        docker run -it --userns=host --rm -w /project -v $(pwd):/project vpi-packager \
               sh -c "cargo deb --version && strip target/$i/release/vpidctl && cargo deb --no-build --verbose --target=$i --manifest-path=./vpid/Cargo.toml"
           # --user $(id -u):$(id -g)
    fi

done

rm vpid/Cargo.toml
mv vpid/Cargo.toml.bup vpid/Cargo.toml 


echo "==============================="
exit 0

