# BorosVPi I2C protocol
VPi uses only I2C bus to communication with the SBC. The hardware supports up to 400khz (fast I2C).

## I2C transaction protocol
 As many I2C interfaces Vpi is based on registers. Some registers are Read only and other are also writable. Vpi act as an I2C slave so it only respond to I2C requests. I2C request are of two types:

- Write request: first byte is the register address and the following bytes are the values to fill in the register map. Register address is stored in a internal register pointer that is incremented with every byte transfered.
- Read request: the I2C master can ask the slave for reading a variable number or bytes. VPi responds with a byte until the STOP condition of I2C protocol is detected. The internal index is used and incremented to read multiple registers in one transfer.

Therfore for reading a registers a I2C transaction is needed. First the master must do a write transfer with one byte with the register to read. Then issue a read request for the number of bytes to read.

## I2C transaction timing, W/R controls.
In order to assure stability it's recommended to wait a period of time between transactions of about 5ms. After the command ```ACT``` (see below) as it require more time to execute. In this case a 20ms delay after sending the commando to the board is recommended.

W/R controls are implemented in the firmaware to avoid that Readonly resgisters or invalid registers are readed or written. The control is implemented as follows:
- Read request: If the register index is bigger than the last register the register index is set to 0 and the first register will be readed.
- Write request: If the re

## Register map

| Register | Len | R/W | Endianness | Volatile | Desctiption                            |
| -------- | --- | --- | ---------- | -------- | -------------------------------------  |
|   00     |  1  |  R  |     NA     | No       | Device ID: Fixed value 0xAA|
|   01     |  1  |  R  |     NA     | No       | Vpi firware version |
|   02     |  1  |  R  |     NA     | Yes      | Status:  10IWBERC bitfield |
|   03     |  1  |  R  |     NA     | Yes      | Flags:   10xxxOWI bitfield |
|   04     |  1  |  R  |     NA     | Yes      | 8-bit CRC of configuration | 
|   05     |  1  |  R  |     NA     | Yes      | Number of short clicks of power button |
|   06     |  1  |  R  |     NA     | Yes      | Number of long clicks of power button  |
|   07     |  1  |  R  |     NA     | Yes      | Number of short clicks of aux button   |
|   08     |  1  |  R  |     NA     | Yes      | Number of long  clicks of aux button   |
|   09     |  2  |  R  |     big    | Yes      | Fan rpm |
|   0B     |  1  |  R  |     NA     | Yes      | Number of errors identified in I2C interface |
|   0C     |  12 |  R  |     NA     | No       | 96-bit UUID |
|   18     |  2  | RW  |     big    | No       | PWM frequency for fan in Hz|
|   1A     |  1  | RW  |     NA     | No       | RPM revolution divisor |
|   1B     |  1  | RW  |     NA     | No       | Watch dog time in seconds |
|   1C     |  2  | RW  |     big    | No       | Autowake time in minutes |
|   1E     |  2  | RW  |     big    | No       | Button short click max time in ms |
|   20     |  2  | RW  |     big    | No       | Buttom max space time between in ms |
|   22     |  1  | RW  |     NA     | No       | Button hold time in second (hard shutdown) |
|   23     |  1  | RW  |     NA     | No       | Shutdown grace time in seconds |
|   24     |  1  | RW  |     NA     | No       | Led Mode  |
|   25     |  1  | RW  |     NA     | No       | Led Value for custom mode |
|   26     |  1  | RW  |     NA     | No       | Beep frequency <0=500Hz , 1=1Khz & 2=2khz> |
|   27     |  1  | RW  |     NA     | No       | Beep duration in 1/10th of seconds |
|   28     |  1  | RW  |     NA     | No       | Beep pause between beeps in 1/10 seconds |
|   29     |  1  | RW  |     NA     | No       | Number of beeps |
|   2A     |  1  | RW  |     NA     | No       | Fan speed (8bit pwm) |
|   2B     |  1  | RW  |     NA     | Yes      | CMD - Command register(cleared after execution) |
|   2C     |  1  | RW  |    NA      | Yes      | ICMD- Command complement register (Cleared after execution) |


## Status & flags
Status and flags are to bytes that can be readed any time that inform the status of the board. they bitfieds with the following structure:

Status: 76543210
        10IWBERC
Where:
    10 : bits 7&6 are fixed with 10 value. This can be used test if there is a typical I2C sync error that will render 0x00 or 0xFF read outs in the bus.
    I: Falling edge interrupt has been detected.
    B: T


## Command execution
Writting configuration registers is not enought to activate some functionalities. Commands must be issued to the board. In order to issue a boar a i2c transaction must be made writing the registers CMD and ICMD. This can be acomplished writing 3 bytes to the I2C interfaces as follows: 
[ CMD register addr=0x2B, CMD , CMD xor magik number ] where magic number is 0xAA.
With this combination CMD + (CMD xor Magic) the board assure the intetegrity of the command
in case of transmission errors in the i2c line and avoid execution of wrong commands.

**Available commands are:**

| Command               | code | Result                                           | Update CRC | 
| -------               | ---- | ------------------------------------------------ | ---------  |
| NOP                   | 0x00 | No operation                                     |   No       |
| Activate config(ACT)  | 0x41 | Activate all configurations but buzzer and Wake  |   Yes      |
| Boot (BOOT)           | 0x42 | Set internal state to booted                     |   No       |
| Init                  | 0x49 | Set internal state to init                       |   No       |
| Feed watchdog (FEED)  | 0x46 | Feed (reset) High level watchdog                 |   No       |
| Hard shutdown (HARD)  | 0x48 | Hard power-off without grace time                |   No       |
| Shutdown (SHUT)       | 0x53 | Shutdown with grace time configured grace reg.   |   No       |
| Clear Flags (CLEAR)   | 0x43 | Clear status and flags                           |   No       |
| Led set (LED)         | 0x4C | Activates the led with led mode & led value regs.|   Yes      |
| Fan set (FAN)         | 0x4E | Activates the fan with fan speed reg.            |   Yes      |
| Beep (BEEP)           | 0x5A | Activates beep with configured values            |   Yes      |
| Out Set (OUTSET)      | 0x31 | Activate Open collector output                   |   No       |
| Out Clear (OUTCL)     | 0x30 | Activate Open collector output                   |   No       |
| Beep (BEEP)           | 0x5A | Activates beep with configured values            |   Yes      |
| Reset (RESET)         | 0x54 | Resets VPi board                                 |   No       |
| Watchdog Set (WDGSET) | 0x57 | Activate the watchdog using wdg reg.             |   Yes      |
| Watchdog Clear(WDGCL) | 0x56 | Deactivate the watchdog using wdg reg.           |   Yes      |
| AutoWake Enable(WEN)  | 0x45 | Enable auto wake after shutdown with wake reg.   |   Yes      |
| AutoWake Disable(WDI) | 0x44 | Disable auto wake after shutdown with wake reg.  |   Yes      |
| AutoWake IRQ En.(IEN) | 0x65 | Enable auto wake with IRQ falling edge           |   No       |
| AutoWake IRQ Dis.(IDI) | 0x64| Disable auto wake with IRQ falling edge          |   No       |

## CRC computation
CRC is computed with de value of registers 0x18 to 0x2A (inclusive) applying to each register the following function (with inititial value of crc=0):

```C
uint8_t crc8_update(uint8_t data, uint8_t crc) {
    crc ^= data;
    for (uint8_t i = 0; i < 8; i++)
        crc = (crc & 0x80) ? (crc << 1) ^ 0x07 : crc << 1;
    return crc;
}
```
