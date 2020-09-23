#!/bin/bash
if [ "$1" == "clean" ] ; then

    echo "Cleaning service..."
    sudo systemctl stop vpid.service
    sudo rm /lib/systemd/system/vpid.service
    sudo rm /etc/vpid/vpidEnv
    sudo rm /etc/vpid/vpid.yml
    sudo systemctl daemon-reload
    #sudo rm -Rf /etc/vpid
    exit 0

fi

echo "Install service for testig..."
sudo mkdir -p /etc/vpid
sudo cp vpidEnv       /etc/vpid/vpidEnv
sudo cp vpid.yml     /etc/vpid/vpid.yml
sudo cp vpid.service /lib/systemd/system/vpid.service
sudo systemctl daemon-reload
sudo systemctl start vpid.service
sudo journalctl -u vpid.service -f


