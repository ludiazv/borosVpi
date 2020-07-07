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
 *      PC3: input pull-up - enable external interrupt both edges
 *      PC4: output push-pull low
 *      PC5: input pull-up - enable external interrupt both edges 
 *      PC6: output pseudo open drain (external pull donw)
 *      PC7: input pull-up - enable external interrupt both edges
 * 
 *   PORTD:
 *      PD1: leave in reset state SWIN will be enabled
 *      PD2: input internal pull-up.  external interrupt falling
 *      PD3: leave at reset state managed by TIM2
 *      PD4: output push-pull low.
 *      PD5 & PD6: Leave in reset state (UART)
 *   
 *             +-----------------------------+ 
 *    Buzz    -| PD4/BEEP          PD3/T2_CH2|-  LED ---R---- |led> ---Vdd
 *    TX      -| PD5/TX            PD2       |-  Fan tachometer (internal pull-up) (EXTI3 - Falling)
 *    RX      -| PD6/RX            PD1/SWIM  |-  To header SWIM (for bootloader programming)
 *    RST     -| RST               PC7       |-  IRQn (input internal pull-up) (EXITI3 - Falling)
 *    X       -| PA1               PC6       |-  Power Switch Mosfet (have 100k pull-down)
 *    X       -| PA2               PC5       |-  Aux Button  (EXTI2  - Both)
 *    GND     -| VSS               PC4       |-  Open colector output -> 1k -> Q  (push pull output)
 *    1uF     -| VCAP              PC3       |-  Power button (EXTI2 - Both)
 *    VDD     -| VDD               PB4/SCL   |-  I2C SCL
 *    FANPWM  -| PA3/T2_CH3        PB5/SDA   |-  I2C SDA
 *             +-----------------------------+ 
 * 
 * 
 *   Buttons signal treatment: 
 *    The two button entries treat are low leve activated and governed by the following parameter
 *    - Dt (Debouncing time): Minimal time of low level to be considered as a button push.
 *                            This parameter is hardcoded in the firmware (DEBOUNCE_MS)
 *    - St (Short time): Maximun time that will be cosidered as a short click.
 *    - Pt (sPace time):
 *    - Ht (Hold time):
 *  
 *  ------                                                                      --------
 *       |                                                                      |
 *       |                                                                      |
 *       ===========|=====================|=====================================|
 *          Dt           St                                                     Ht
 * 
 *    IRQn:
 *    IRQn line is active at low level and detect falling edge on the line. 
 * 
 *  
 */

// Some defines for readibitly
#define PWR_BUT_PIN             (3)
#define AUX_BUT_PIN             (5)
#define IRQ_PIN                 (7)
#define OUTPUT_PIN              (4)

//< Debounce time in milliseconds
#define DEBOUNCE_MS             (55)

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