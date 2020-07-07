#!/bin/bash
echo "Running tester session"
networksetup -setairportnetwork en2 VpiTester
sleep 5
telnet -K 192.168.4.1
