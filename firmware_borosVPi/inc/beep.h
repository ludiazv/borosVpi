/**
 * @file beep.h
 * @author LDV
 * @brief  Beep controller using BEEP perififerical of the STM8S line.
 * @version 0.1
 * @date 2020-05-08
 * 
 * @copyright Copyright LDV (c) 2019
 * 
 */
#ifndef _VPI_BEEP_H_
#define _VPI_BEEP_H_

#include<stm8s.h>


// Some helpers
#define BEEP_ENABLE()   ( BEEP_CSR |=  (1 << 5) )
#define BEEP_DISABLE()  ( BEEP_CSR &= ~(1 << 5) )

// Prototypes
void init_beep();
void start_beep(uint8_t freq,uint8_t count,uint8_t beep_time,uint8_t pause_time);
void update_beep();

#endif