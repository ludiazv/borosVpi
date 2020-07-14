#!/bin/bash

echo "VPI create release"
echo "=================="
mkdir -p dist
for i in $(vpid/buildall.sh travis archs) 
do
    mv vpid/target/$i/debian/*.deb dist/
done
ls -l dist



