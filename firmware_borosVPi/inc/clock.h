/**
 * @file clock.h
 * @author LDV
 * @brief  Bare Metal clock control for STM8S: Timebase, High-level watchdog & tachometer.
 * @version 0.1
 * @date 2019-05-08
 * 
 * @copyright Copyright (c) 2019
 * 
 */
#ifndef _VPI_CLOCK_H_
#define _VPI_CLOCK_H_

#include<stm8s.h>
#include<delay.h>
#include<vpi_i2c.h>
#include<dbg.h>

extern volatile uint32_t     _milis;         ///< Milis counter
#define milis()     (_milis)                 ///< Arduino-like milis exposed to avoid using _milis variable
extern volatile uint16_t     _seconds;       ///< Second counter
#define seconds()   (_seconds)               ///< Arduinio-like seconds exposed to avoid using _seconds variable
extern volatile uint16_t     _minutes;       ///< minute counter
#define minutes()   (_minutes)               ///< Arduino-like minutes exposed to avoid uing _minutes variable

/// Do a software reset using windows wdg
#define SOFTWARE_RESET()        (WWDG_CR = 0x80)

// Proptotipes (Documentation in .c)
void init_clocks();
uint8_t set_rpm_divider(uint8_t d);
void feed_wdg();
uint8_t wdg_check();
void    wdg_start(uint8_t limit);
void start_wake();
uint8_t wake_check(uint16_t limit);
void delay(uint16_t ms);
void timer_isr(void)        __interrupt(TIM4_ISR);  // Must be declared and included into main
void tachometer_isr(void)   __interrupt(EXTI3_ISR); // Must be declared and included into main


#endif