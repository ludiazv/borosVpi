#!/bin/bash

if [ ! -d /sys/class/gpio/gpio4 ] ; then
   echo "Exporting 4"
   echo "4" > /sys/class/gpio/export
   sleep 1
fi

echo "out" > /sys/class/gpio/gpio4/direction
echo "0"   > /sys/class/gpio/gpio4/value
sleep 1
echo "1"   > /sys/class/gpio/gpio4/value
sleep 1
echo "0"  > /sys/class/gpio/gpio4/value

