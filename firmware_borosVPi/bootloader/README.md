# Custom bootloader
Custom bootloader for i2c posted (here)[https://github.com/ludiazv/stm8s-bootloader].

## Features
* Size: 707 bytes . Reservered: 14*64bytes blocks => 896 bytes
* Bootloader delay activation aprox 300ms
* Bootloader in I2C mode with address 0x22
* Bootloader led in PD3
* ITV is relocated.
* Code location: 0x8300

## Compilation defines:
__I2C_ADDR__     0x22
__DELAY_COUNT__  0xFFFF
__BOOT_ADDR__    0x8300
__FLASH_PIN__    3
__FLASH_PIN_DDR__      PD_DDR
__FLASH_PIN_ODR__      PD_ODR
__FLASH_PIN_CR1__      PD_CR1

## Init code .s

´´´
.macro jump addr
    ;jp 0x8280 + addr
    jp  0x8300 + addr
    .ds 1
.endm

´´´