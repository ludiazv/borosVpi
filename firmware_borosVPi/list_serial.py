import serial.tools.list_ports as lp 
import re

for p in lp.comports():
    if re.search(".+usb.+",p.device) is not None:
        print(p.device)
