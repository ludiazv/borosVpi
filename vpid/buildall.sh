#!/bin/bash

# image to use the binaries once creating debs 
STRIP_BASE="aarch64-unknown-linux-gnu"

if [[ "$1" == "travis" ]] ; then
    #ARCHS=( "armv7-unknown-linux-gnueabihf" "armv7-unknown-linux-musleabihf" "aarch64-unknown-linux-musl" "aarch64-unknown-linux-gnu" "arm-unknown-linux-musleabihf" "arm-unknown-linux-gnueabihf")
    ARCHS=( "aarch64-unknown-linux-gnu" )
else
    ARCHS=( "armv7-unknown-linux-gnueabihf" "aarch64-unknown-linux-gnu" )
fi

if [[ "$2" == "archs" ]] ; then
    echo "${ARCHS[@]}"
    exit 0
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
printf "Check cross tool..."
cargo install --list | grep cross
if [ $? -ne 0 ] ; then
    set -e
    echo "not installed. force install"  
    cargo install cross --force
    set +e
else
    echo "Installed!"
fi
printf "Cross version in use:"
CROSS_VERSION=$(cross --version | grep cross | cut -d ' ' -f 2)
echo $CROSS_VERSION

if [ "$1" == "travis" ] ; then
    printf "Check for cargo-deb ..."
    cargo install --list | grep cargo-deb
    if [ $? -ne 0 ] ; then
        set -e
        echo "not installed. force install"  
        cargo install cargo-deb --force
        set +e
    else
        echo "Installed!"
    fi
    cargo deb --version
else
    printf "Check docker image for packager tool..."
    docker images | grep vpi-packager
    if [ $? -ne 0 ] ; then
        echo "not installed. Building docker image this take a while "
        pushd scripts
            docker build -t vpi-packager -f scripts/Dockerfile .
        popd
    else
        echo "Installed!"
    fi
fi

set -e
echo "Cargo update to generate Cargo.lock"
cargo update

printf "Start building vpid + vpidctl version:"
VPID_VERSION=$(cargo pkgid -v --manifest-path ./vpid/Cargo.toml | cut -d \# -f 2)
echo "$VPID_VERSION"

echo "Building release versions...."
for i in "${ARCHS[@]}"
do
    echo "Building for Release $i"
    cross build --target=$i --release
    if [ $? -eq 0 ] ; then
        echo "Strip release binaries for $i"
        STRIP="/usr/bin/aarch64-linux-gnu-strip"
        docker run -it --rm -v $(pwd)/target/$i/release:/project  rustembedded/cross:$STRIP_BASE-$CROSS_VERSION $STRIP -v /project/vpid
        docker run -it --rm -v $(pwd)/target/$i/release:/project  rustembedded/cross:$STRIP_BASE-$CROSS_VERSION $STRIP -v /project/vpidctl
    fi
done

set +e
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
    ["../assets/vpidEnv","etc/vpid/","666"],
    ["../assets/vpid.service","lib/systemd/system/","644"]
]
EOF
    #cross deb --no-build --target=$i --verbose --manifest-path=./vpid/Cargo.toml
    T=$(echo "$i" | cut -d '-' -f 1,4)
    DEBV=${VPID_VERSION}_${T}
    echo "Using sintetic package version as $DEBV"
    if [ "$1" == "travis" ] ; then
        cargo deb --no-build --verbose --target=$i --manifest-path=./vpid/Cargo.toml --deb-version $DEBV
    else
        docker run -it --userns=host --rm -w /project -v $(pwd):/project vpi-packager \
               sh -c "cargo deb --version && cargo deb --no-build --verbose --target=$i --manifest-path=./vpid/Cargo.toml --deb-version $DEBV"
           # --user $(id -u):$(id -g)
    fi

done

rm vpid/Cargo.toml
#mv vpid/Cargo.toml used.toml
mv vpid/Cargo.toml.bup vpid/Cargo.toml 


echo "==============================="
exit 0

