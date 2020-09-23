#!/bin/bash
WIFI_INTERFACE="en2"
echo "Running tester automation"
echo "========================="
echo "¡script only works in macOs!"
echo "Enable wifi please"
networksetup -getairportpower $WIFI_INTERFACE
networksetup -setairportpower $WIFI_INTERFACE on
networksetup -setairportnetwork $WIFI_INTERFACE VpiTester
sleep 6
{ echo; echo 'run()'; echo; echo 'quit()'; cat - ; } | telnet -K 192.168.4.1 | tee tmp.res
echo "Report"
echo "======"
cat tmp.res | egrep "(#.*|>.*)"
#rm tmp.res
networksetup -setairportpower $WIFI_INTERFACE off
networksetup -getairportpower $WIFI_INTERFACE 


