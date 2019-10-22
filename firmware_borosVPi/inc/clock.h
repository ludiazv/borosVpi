#ifndef _VPI_CLOCK_H_
#define _VPI_CLOCK_H_

#include<stm8s.h>
#include<delay.h>
#include<vpi_i2c.h>


extern volatile uint32_t     _milis;         ///< Milis counter
#define milis()     (_milis)
extern volatile uint16_t     _seconds;       ///< Second counter
#define seconds()   (_seconds)

void init_clocks();
uint8_t set_rpm_divider(uint8_t d);
void feed_wdg();
uint8_t wdg_check(uint8_t limit);
void delay(uint16_t ms);
void timer_isr(void)        __interrupt(TIM4_ISR);  // Must be declared and included into main
void tachometer_isr(void)   __interrupt(EXTI3_ISR); // Must be declared and included into main


#endif