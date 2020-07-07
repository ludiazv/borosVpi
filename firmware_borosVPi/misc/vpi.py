# Vpi module
import time

VPI_ADDR= const(0x33)
CMD_NOP=const(0)
CMD_BOOT=const(66)
CMD_FEED=const(70)
CMD_HARD=const(72)
CMD_SHUT=const(83)
CMD_CLEAR=const(67)

#define VPI_CMD_ACT     ('A')
#define VPI_CMD_FAN     ('N')
#define VPI_CMD_LED     ('L')
CMD_BEEP=const(90)
CMD_OUTSET=const(49)
CMD_OUTCL=const(48)
CMD_RESET=const(84)
CMD_WDGSET=const(87)
CMD_WDGRST=const(86)
CMD_WEN=const(69)
CMD_WDI=const(68)
CMD_IEN=const(101)
CMD_IDI=const(100)

class Vpi:
    def __init__(self,i2c):
        self.i2c=i2c
        self.magik=0xAA
        self.id()

    def read(self,reg,n):
        b=reg.to_bytes(1,"big")
        self.i2c.writeto(VPI_ADDR,b)
        time.sleep_ms(5)
        return self.i2c.readfrom(VPI_ADDR,n)
    
    def write(self,reg,buff):
        b= reg.to_bytes(1,"big") + buff
        time.sleep_ms(3)
        return self.i2c.writeto(VPI_ADDR,b)
    
    def id(self):
        r=self.read(0,2)
        self.magik=r[0]
        return "ID:{:02X}h,VER:{:02X}h".format(r[0],r[1])
    
    def cmd(self,by):
        b= by.to_bytes(1,"big") + (by ^ self.magik).to_bytes(1,"big")
        self.write(0x2B,b)

    def boot(self):
        self.cmd(CMD_BOOT)
        
    def clear(self):
        self.cmd(CMD_CLEAR)
        
    def is_running(self):
        r=self.read(2,1)
        return (r[0] & (1<<3)) == 8

    def has_irq(self):
        r=self.read(2,1)
        return (r[0] & (1<<5)) == 32
    
    def has_click(self):
        r=self.read(2,1)
        return (r[0] & 1) == 1
            
    def get_clicks(self):
        r=self.read(5,4)
        return (r[0],r[1],r[2],r[3])
            
    def wdg(self,s):
        self.write(0x1B,s.to_bytes(1,"big"))
        if s==0:
            self.cmd(CMD_WDGRST)
        else:
            self.cmd(CMD_WDGSET)

    def wake(self,s):
        self.write(0x1C,s.to_bytes(2,"big"))
        if s==0:
            self.cmd(CMD_WDI)
        else:
            self.cmd(CMD_WEN)
    
    def feed(self):
        self.cmd(CMD_FEED)
    
    def dump_conf(self):
        return "PWMF:{},DIV:{},WDG:{},WAK:{},SH:{},SP:{},H:{},G:{}".format(
            int.from_bytes(self.read(0x18,2),2,"big"),
            self.read(0x1A,1)[0],
            self.read(0x1B,1)[0],
            int.from_bytes(self.read(0x1C,2),2,"big"),
            int.from_bytes(self.read(0x1E,2),2,"big"),
            int.from_bytes(self.read(0x20,2),2,"big"),
            self.read(0x22,1)[0],
            self.read(0x23,1)[0]
        )
    
    def beep(self,f,n,d,p):
        b=f.to_bytes(1,"big") + d.to_bytes(1,"big") + p.to_bytes(1,"big") + n.to_bytes(1,"big")
        
        self.write(0x26,b)
        self.cmd(CMD_BEEP)
        return n*(d*100+p*100)
        
    #def status(self):
    #    r=self.read(2,3)

