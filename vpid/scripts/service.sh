#!/bin/bash


if [ "$1" == "clean" ] ; then

    echo "Cleaning service..."
    sudo systemctl stop vpid.service
    sudo rm /lib/systemd/system/vpid.service
    sudo systemctl daemon-reload
    rm -R /etc/vpid
    exit 0

fi

echo "Install service for testig..."
mkdir -p /etc/vpid
cp vpiEnv       /etc/vpid/vpidEnv
cp vpid.yml     /etc/vpid/vpid.yml
cp vpid.service /lib/systemd/system/vpid.service


