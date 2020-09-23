import machine
import time
from machine import Pin
from machine import I2C
from machine import Timer
import network
import ssd1306
from ina219 import INA219
from pcf8574 import PCF8574
import uploader
import vpi

# Setup the hardware
# -------------------
# IRQ
Irq = Pin(16,Pin.OUT,value=1)
# Led
Led = Pin(2,Pin.OUT,value=1)
# Rst
Rst=Pin(13,Pin.OUT, value=0)
# Action button
But=Pin(0,Pin.IN)
# Power button & aux button simulator
pBut=Pin(12,Pin.OUT,value=1)
aBut=Pin(14,Pin.OUT,value=1)

# I2c inerface
I2c=I2C(scl=Pin(5), sda=Pin(4), freq=100000)
# Timer for pseudo pwm for current & counter
Tim=Timer(-1)
Tmc=0
Port_last=0x0F
Pseudo_pwm = [0,0,0,0,0,0,0,0,0,0]
# Oled screen
Oled=None
#IO expander
Io=None
# IN219 current & V meter
Ina=None
# firmaware flag
Firmware_found=False
# Vpi Objecte
Pi=None

# Create access point for testing
Ap = network.WLAN(network.AP_IF) # create access-point interface
Ap.active(True)         # activate the interface
#Ap.config(essid='VpiTester') # set the ESSID of the access point
print("Web server opened")
print(Ap.ifconfig())
print("Type run() to run the tests")


def set_current(n):
    global Port_last
    if n != Port_last:
        Io.port = (Io.port & 0xF0) |  (0x0F & n)
        Port_last = n
        #print("Io Status {} {:02X}h".format(n,Io.port))

def current_callback(a):
    global Tmc
    set_current(Pseudo_pwm[Tmc])
    Tmc= 0 if Tmc >= 9 else Tmc + 1


def start_timer():
    Tim.init(period=100,mode=Timer.PERIODIC,callback=current_callback)
def stop_timer():
    Tim.deinit()

def col(n):
    return "\033[32m{}\033[0m".format(n) if n else "\033[31m{}\033[0m".format(n)

def init_i2c():
    global Oled
    global Ina
    global Io
    global Firmware_found
    global Tim
    #reset_board()
    #boot_loader=I2c.is_ready(0x22)
    devs=I2c.scan()
    if 0x3C in devs:
        print("Oled screen detected")
        Oled=ssd1306.SSD1306_I2C(128, 64, I2c)
        Oled.poweron()
        Oled.fill(0)
        Oled.contrast(255)
        inf("Init!",1000)
        Oled.show()
    if 0x40 in devs:
        print("INA219 detected")
        Ina=INA219(0.1, I2c,max_expected_amps=2.5)
        Ina.configure(voltage_range=INA219.RANGE_16V)
    if 0x20 in devs:
        print("PCF8574 io expander detected ",end='')
        Io=PCF8574(I2c)
        time.sleep_ms(500)
        print("IO:{:02X}h".format(Io.port) )
        set_current(0)
        Tmc=0
    if 0x22 in devs:
        print("Bootloader found")
    if 0x33 in devs:
        print("Vpi Board found")
        Firmware_found=True
        
    return (Ina is not None) and (Io is not None)



        

def inf(s,delay=0):
    if Oled is not None:
        s2=s.replace('\033[0m','').replace('\033[32m','').replace('\033[31m','')
        s2= s2[:112] if len(s2) > 112 else s2
        lines = len(s2) // 16
        lines = lines+1 if (len(s2) % 16)!= 0 else lines
        #Oled.fill(0)
        Oled.scroll(0, lines* 8)
        Oled.fill_rect(0,0,127,8*lines,0) 
        for l in range(lines):
            Oled.text(s2[l*16:200],0,8*(l+1))

        #Oled.text(s2,0,10)
        Oled.show()
    print(s)
    time.sleep_ms(delay)

def wait_for_click(ms):
    started=time.ticks_ms()
    while But.value() == 1 and (time.ticks_ms()-started) < ms:
        time.sleep_ms(10)
        
    return (But.value() == 0)

def click(but,ms,post=0):
    but.value(0)
    time.sleep_ms(ms)
    but.value(1)
    if post >0:
        time.sleep_ms(post)


def test_is_off():
    v=Ina.supply_voltage()
    return v<1.0

def test_is_on():
    v=Ina.supply_voltage()
    return v>4.50

def current_monitor(n=1,or_condition=None):
    while n>0:
        inf("{:.2f}V {:.1f}mA".format(Ina.supply_voltage(),Ina.current() ))
        if or_condition is not None:
            if or_condition():
                return True
        time.sleep(1)
        n=n-1
    return False
    
def upload_firmware():
    inf("Up...",500)
    up=uploader.Stm8s_i2c_uploader(I2c,Rst)
    inf("Up:{}".format(up.upload("firmware.bin")),1000)
 
def run():
    global Pi
    #init_i2c()
    if not Firmware_found:
        inf("VPiFW not found",50)
        upload_firmware()
    else:
        inf("Vpi found press to force")
        if wait_for_click(5000):
            upload_firmware()
                    
    Pi=vpi.Vpi(I2c)
    #Pi.cmd(vpi.CMD_RESET)
    inf("Running tests...",1000)
    Led.value(0)
    inf("ID {}".format(Pi.id()),200)
    inf(Pi.dump_conf())
    set_current(3) # Perform all basic test with 1A load
    time.sleep_ms(500)
    current_monitor(2)
    # Each function is a test set
    test_on_off()
    but_and_irq()
    wdg_wake()
    beep()
    #fan()
    test_dynamic_current()
    test_full_current()

    Led.value(1)
    Pi.cmd(vpi.CMD_HARD)
    set_current(0)
    
def test_dynamic_current():
    global Pseudo_pwm
    inf("# Section - dyn current")
    # Reset Pseudo pwm
    for i in range(10):
        Pseudo_pwm[i]=0
    
    # Iterate all currents
    for i in range(41):
        # Full part
        l=i//4
        for j in range(l):
            Pseudo_pwm[j]=0x0F

        if l<10:
            r= i % 4
            v= 3 if r==2 else r
            v= 7 if r==3 else v
            Pseudo_pwm[l]=v
        
        start_timer()
        volt=10
        current=0
        for j in range(10):
            s= Ina.supply_voltage()
            volt = s if s<volt else volt
            current += Ina.current()
            time.sleep_ms(100)
        stop_timer()
        inf(">{}/{:.2f}V {:.1f}mA".format(i,volt,current/10))
    set_current(0)

def test_full_current():
    inf("# Section Test full current")
    set_current(0x0f)
    current_monitor(5*60)
    set_current(0)        
    
def test_on_off():
     # Section 1 - Basic on-off
    inf("# Section - On/Off")
    Pi.boot()
    inf(">Booting",500)
    current_monitor(1)
    inf("==>Is running:{} Vbus:{}".format(col(Pi.is_running()),col(test_is_on()) ))
    inf("==>Out enabled:{}".format( col(Io.pin(4) == 0) ))
    inf(">Hard off",500)
    Pi.cmd(vpi.CMD_HARD)
    inf("==>Is off:{} Vbus:{}".format( col(Pi.is_running() == False), col(test_is_off()) ),100)
    inf("==>Out disabled:{}".format( col(Io.pin(4) == 1) ) ,100)
    inf(">Power button",500)
    click(pBut,1000)
    current_monitor(3)
    inf("==>Vbus:{}".format(col(test_is_on()) ))
    inf(">Hard Button",500)
    click(pBut,8500)
    current_monitor(1)
    inf("==>Is off:{} Vbus:{}".format(col(Pi.is_running() == False), col(test_is_off())))
    inf(">Grace Shutdown",500)
    Pi.cmd(vpi.CMD_SHUT)
    Pi.boot()
    Pi.cmd(vpi.CMD_SHUT)
    current_monitor(16,test_is_off) # Grace time is 15
    inf("==>Is off:{} Vbus:{}".format(col(Pi.is_running() == False), col(test_is_off())))
    
def but_and_irq():
    Pi.boot()
    inf("# Section - buttons & IRQ",200)
    inf(">Irq flag")
    click(Irq,100)
    prev=Pi.has_irq()
    Pi.clear()
    inf("==>Irq:{},Cleared:{}".format( col(prev), col(Pi.has_irq() == False) ))
    inf(">Irq Wake")
    Pi.cmd(vpi.CMD_WEN)
    Pi.cmd(vpi.CMD_IEN)
    Pi.cmd(vpi.CMD_HARD)
    current_monitor(3)
    click(Irq,100)
    current_monitor(3)
    inf("==>Irq Wake:{}".format( col(test_is_on()) ))
    Pi.cmd(vpi.CMD_WDI)
    Pi.cmd(vpi.CMD_IDI)
    Pi.boot()
    inf(">Clicks",200)
    click(pBut,10,1300)
    inf("==>Debounce:{}".format( col( Pi.has_click()==False) ))
    Pi.clear()
    click(pBut,150)
    click(aBut,150)
    click(aBut,900)
    click(pBut,1000,1300)
    inf("==>Clicks Has:{} , match:{}".format( col(Pi.has_click()), col(Pi.get_clicks() == (1,1,1,1))   ))
    Pi.clear()
    inf("==>Clear: {}".format( col( Pi.has_click()==False) ))
    

def wdg_wake():
    Pi.boot()
    inf("#Wake & WDG")
    Pi.wdg(5)
    inf(Pi.dump_conf())
    current_monitor(3)
    Pi.feed()
    current_monitor(3)
    inf("==>Feed:{}".format( col(test_is_on())  ))
    current_monitor(6)
    inf("==>WDG:{}".format(  col(test_is_off()) ))
    current_monitor(6)
    Pi.wake(1)
    current_monitor(2)
    Pi.cmd(vpi.CMD_HARD)
    current_monitor(121,test_is_on)
    inf("==>Wake:{}".format( col(test_is_on()) ))
    Pi.boot()

def beep():
    Pi.boot()
    inf("#Beep")
    for i in range(0,3):
        w=Pi.beep(i,2,10,5)
        time.sleep_ms(w+10)

    inf("==>Beep:Check sound")


# Init I2C
init_i2c()


    