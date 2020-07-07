# STM8S uploader module for micropython
import os
import time
import math

BLOCK_SIZE  = const(64)
BOOT_ADDR   = const(0x22)
BOOT_ACK    = b'\xaa\xbb' 


def crc8_update(data, crc):
    crc ^= data
    for i in range(0, 8):
        if crc & 0x80 != 0:
            crc = (crc << 1) ^ 0x07
        else:
            crc <<= 1
    return crc & 0xFF

def get_crc(file):
    crc = 0
    data = open(file, 'rb')
    with data as f:
        chunk = f.read(BLOCK_SIZE)
        while chunk:
            chunk = chunk + (b'\xFF' * (BLOCK_SIZE - len(chunk)))
            for i in chunk:
                crc = crc8_update(i, crc)
            chunk = f.read(BLOCK_SIZE)
    return crc.to_bytes(1,"big")


class Stm8s_i2c_uploader:
    def __init__(self,i2c,rst_pin,inverted=True):
        self.i2c=i2c
        self.rst_pin=rst_pin
        if inverted:
            self.high=0
            self.low=1
        else:
            self.high=1
            self.low=0
        self.crc=0
        
    def rst(self):
        self.rst_pin.value(self.high)
        time.sleep_ms(100)
        self.rst_pin.value(self.low)
        time.sleep_ms(300)
        self.rst_pin.value(self.high)
        time.sleep_ms(15)
    
    def enter_bootloader(self,file):
        req= b'\xde\xad\xbe\xef'
        n=math.ceil(os.stat(file)[6]/BLOCK_SIZE)
        req= req + n.to_bytes(1,"big") + self.crc + self.crc
        self.rst()
        self.i2c.writeto(BOOT_ADDR,req)
        time.sleep_ms(10)
        req=self.i2c.readfrom(BOOT_ADDR,2)
        return (req == BOOT_ACK)


    def upload(self,file):
        # Compute the CRC of file
        self.crc=get_crc(file)
        # Enter into booot loader
        if self.enter_bootloader(file):
            data = open(file, 'rb')
            #total = 0
            #idx = 0
            with data as f:
                chunk = f.read(BLOCK_SIZE)
                while chunk:
                    chunk=chunk  + (b'\xFF' * (BLOCK_SIZE - len(chunk)))
                    self.i2c.writeto(BOOT_ADDR,chunk)                    
                    chunk = f.read(BLOCK_SIZE)
                    time.sleep_ms(50) # I2c could be to fast , give some time to process the block
        
            time.sleep_ms(10)
            ret=self.i2c.readfrom(BOOT_ADDR,2)
            self.rst()
            return (ret == BOOT_ACK)
        else:
            return False







