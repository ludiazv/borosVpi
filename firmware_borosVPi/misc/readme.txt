Tester board micropython Notes.

Tester uses a Wemos D1 as core that need to be flashed with .bin file of upython for ESP8266 using esptool or thonny ide.

Required libraries:
- loggin: loggin.py copied from micropython-lib repository
- ina219: ina219.py copied from https://github.com/chrisb2/pyb_ina219
- pcf8574: pcf8574.py copied from https://github.com/mcauser/micropython-pcf8574

This to libraries need to be cross compiled with:
- python -m mpy_cross -o ina219.mpy -march=xtensa ina219.py
- python -m mpy_cross -o logging.mpy -march=xtensa logging.py
- python -m mpy_cross -o pcf8574.mpy -march=xtensa pcf8574.py


and then copied with rshell to the board:

rshell -p /dev/<dev> --rts 1 cp *.mpy /pyboard/


Then thonny IDE can be used to upload main.py that contain the tester software.



