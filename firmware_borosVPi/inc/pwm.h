#ifndef _VPI_PWM_H_
#define _VPI_PWM_H_

#include<stm8s.h>
#include<vpi_regs.h>
#include<clock.h>  // Require milis


/**
 * @brief PWM module to manage LED & FAN
 * 
 *  Features:
 *    - 8bit resolution PWM 0-255 (0 off )
 *    - Inverted polarity to be used with pmosfet and leds to vcc.
 *    - Frequecy configurable from 250 Hz to 62.5 Khz
 * 
 *   Implmentation:
 *      - Use TIM2 to generate time base and pwm pulse.
 *      - Prescaler = 0 . To archive up to 62.5 Khz (fized)
 *      - Frequency geneated by ARR counter computed as ARR = Fmaster/desired frequency.
 *      - 62,5 Khz => ARR = 255,  250 Hz = 64000 (by default 25Khz ARR=640 or 0x0280)
 *      - Duty cycle computation: (ARR/255) * 8bit value => comparator register. (forced upper bound)
 * 
 *    Channels: 
 *       -Pin use PD3 TIM2 CH2 for led  (do not require opt byte as is defult output of the channel)
 *       -Fan use PA3 TIM2 CH1 for fan  (do not require opt byte as is default otput of the channel)
 *        
 * 
 */

#define LED_MODE_OFF        0
#define LED_MODE_ON         1
#define LED_MODE_CY         2
#define LED_MODE_FCY        3
#define LED_MODE_BLINK      4
#define LED_MODE_FBLINK     5
#define LED_MODE_CUSTOM     6

void init_pwm();
uint16_t set_pwm_freq(uint16_t freq);
uint8_t  update_led(uint8_t mode,uint8_t val);
void     update_fan(uint8_t val);
void     u_led();

#endif