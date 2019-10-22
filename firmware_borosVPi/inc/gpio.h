#ifndef _VPI_GPIO_H_
#define _VPI_GPIO_H_

#include<stm8s.h>

/**
 * @brief GPIO Mapping for 20-pin versions of STM8SX03F3
 * 
 *   All GPIOs have reset input floating state
 * 
 *   PORTA: Is not used for GPIO PA1,PA2 are unconnected, PA3 is manged by timer 2.
 *       PA1: output push-pull low
 *       PA2: output push-pull low
 *       PA3: Leave in reset state
 *            
 *   PORTB: Not used as PB4&5 ara used by I2C peri.
 *        leave at reset state 
 * 
 *   PORTC: 
 *      PC3: input floating (external filter circuit )    - enable external interrupt both edges
 *      PC4: output push-pull low
 *      PC5: input pull-up (no filter circuit implemented) - enable external interrupt both edges 
 *      PC6: output pseudo open drain (external pull donw)
 *      PC7: output push-pull low
 * 
 *   PORTD:
 *      PD1: leave in reset state SWIN will be enabled
 *      PD2: input internal pull-up.  external interrupt falling
 *      PD3: leave at reset state managed by TIM2
 *      PD4: output push-pull low.
 *      PD5 & PD6: Leave in reset state (UART)
 *   
 *             +-----------------------------+ 
 *    Buzz    -| PD4/BEEP          PD3/T2_CH2|-  LED ---R---- |> ---|
 *    TX      -| PD5/TX            PD2       |-  Fan tachometer (internal pull-up) (EXTI3 - Falling)
 *    RX      -| PD6/RX            PD1/SWIM  |-  To header
 *    RST     -| RST               PC7       |-  Boot select boot loader/ in program will be output low (header)
 *    X       -| PA1               PC6       |-  Power Switch Mosfet (have 100k pull-down)
 *    X       -| PA2               PC5       |-  Aux Button  (EXTI2  - Both)
 *    GND     -| VSS               PC4       |-  Test point (debug pin 2)
 *    1uF     -| VCAP              PC3       |-  Power button (EXTI2 - Both)
 *    VDD     -| VDD               PB4/SCL   |-  I2C SCL
 *    FANPWM  -| PA3/T2_CH3        PB5/SDA   |-  I2C SDA
 *             +-----------------------------+ 
 * 
 * 
 *   Buttons signal treatment:
 * 
 *  
 *  ------                                                                      --------
 *       |                                                                      |
 *       |                                                                      |
 *       ===========|=====================|=====================================|
 *          Dt           St                                                     Ht
 * 
 *    
 * 
 *  
 */

// Some defines for readibitly
#define PWR_BUT_PIN             (3)
#define AUX_BUT_PIN             (5)

#define DEBOUNCE_MS             (25)

// Button index
#define PWR_BUT                 (0)
#define AUX_BUT                 (1)

// Drive power mosfet (is a p type) therfore is inverted
#define PWR_ON()        ( PC_ODR |=  (1 << 6) )
#define PWR_OFF()       ( PC_ODR &= ~(1 << 6) )
#define DBG1_ON()       ( PC_ODR |=  (1 << 7) )
#define DBG1_OFF()      ( PC_ODR &= ~(1 << 7) )
#define DBG1_INV()      ( PC_ODR ^=  (1 << 7) )
#define DBG2_ON()       ( PC_ODR |=  (1 << 4) )
#define DBG2_OFF()      ( PC_ODR &= ~(1 << 4) )
#define DBG2_INV()      ( PC_ODR ^=  (1 << 4) )

// prototypes
void init_gpio();
uint16_t set_hold_time(uint16_t hold);
uint16_t set_space_time(uint16_t space);
uint16_t set_short_time(uint16_t sht);
uint8_t update_buts(uint8_t *buts,uint8_t *status);
void    update_outs(uint8_t o);
void    reset_buts();

// ISR
void but_isr() __interrupt(EXTI2_ISR); // Need to be delcared and included in main.c

#endif