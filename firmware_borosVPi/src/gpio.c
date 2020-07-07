/**
 * @file gpio.c
 * @author your name (you@domain.com)
 * @brief 
 * @version 0.1
 * 
 * @copyright Copyright (c) 2020
 * 
 */
#include "dbg.h"
#include "gpio.h"
#include "clock.h"
#include "vpi_i2c.h"
#include <string.h>


// Control variables
static uint8_t  _prev;                          // Previous values of the PC port
static uint8_t  _tmp_clicks[2][2];              // Button countrer clicks
volatile static uint32_t _started[2];           // Started time
volatile static uint32_t _last_started;         // Last started click
volatile static uint32_t _last_finished;        // Last finished click

static uint16_t short_time,space_time;
static uint32_t hold_time;

const  uint8_t  but_pin[2]={PWR_BUT_PIN,AUX_BUT_PIN};

// Helpers
#define READ_BUT()          (  PC_IDR                )
#define IS_LOW(s,i)         ( !( (s) & (1 << but_pin[(i)] ) )   )   
#define IS_HIGH(s,i)        (  ( (s) & (1 << but_pin[(i)] ) )   )
#define HAS_CHANGED(s,i)    (  ( (s) & (1 << but_pin[(i)] ) )   )

/**
 * @brief Init GPIOs and external interrupts using control registers
 *   Key registers:
 *    DDR (Direction) 0 - Input
 *        CR1 CR2
 *          0   0    -  Floating input
 *          0   1    -  Floating with IRQ
 *          1   0    -  Pull up without IRQ
 *          1   1    -  Pull up with IRQ
 *    DDR 1 - Output
 *        CR1 CR2
 *          0   0    -  Pseudo Open drain
 *          0   1    -  Pseudo  drain fast speed
 *          1   0    -  Push-Pull low speed
 *          1   1    -  Push-Pull fast speed
 */
void init_gpio() {
    // Port C
    //    0,1,2 -> Not exposed (ignored)
    //    3 -> Power button input, pull-up, EXTI
    //    4 -> Output 1     output, PP, low speed (default:0)
    //    5 -> Aux Button   input, pull-up, EXTI
    //    6 -> Power Switch output, OD , Low speed (default:0)
    //    7 -> IRQn         input, pull-up, EXTI          
    //         76543210
    PC_ODR = 0b01000000;    // all outputs will be 0 if configured in ddr
    PC_DDR = 0b01010000;    // PC6,PC4 output the rest inputs
    PC_CR1 = 0b10111000;    // PC7:PU  PC6:OD PC5:PU PC4:PP PC3:PU
    PC_CR2 = 0b10101000;    // PC7:EX  PC6:LS PC5:EX PC4:LS PC3:EX
   
    // Port B => not used leave at reset state
    // configured in i2c module @see vpi_i2c.c

    // Port A 
    //         76543210
    PA_ODR = 0b00000000;    // all outputs will be 0 if configured in ddr
    PA_DDR = 0b00000000;    // All inputs 
    PA_CR1 = 0b00000110;    // with push-pull  (unsed pins)

    // Port D
    //         76543210
    PD_ODR = 0b00000000;    // all outputs will be 0 if configured in ddr
    PD_DDR = 0b00010000;    // PD4 output low
    PD_CR1 = 0b00010100;    // PD4 push-pull, PD2 internal pull-up
    PD_CR2 = 0b00000100;    // PD2 EXIT enabled. Ouputs: Low speed IO.
   

    // EXIT type selection.
    //           76543210
    //           DDCCBBAA
    EXTI_CR1 = 0b10110000;   // 11-> both in port C, 10-> failing in port D
    
    //Seting software Interrupt priorities.
    // to 1 (lowest priority) b01
    // EXTI2 & EXTI3 are ISR 5 & 6 => ITC_SPR2(7,6,5,4)
    ITC_ISPR2 &= 0b11000011;
    ITC_ISPR2 |= 0b00010100;


    // Load initial values of variables.
    reset_buts();
    //DBG("GPIO started\n\r");
} 





uint16_t set_hold_time(uint16_t hold) {
    hold_time= 1000UL * hold; // TODO: Set limits
    //DBG("Set hold time:%i s\n\r",hold);
    return hold;
}
uint16_t set_space_time(uint16_t space) {
    space_time = space; // TODO: Set limits
    //DBG("Set space time:%i ms\n\r",space_time);
    return space_time;
}
uint16_t set_short_time(uint16_t sht){
    short_time = (sht > DEBOUNCE_MS*2) ? sht : DEBOUNCE_MS*2; // short time should be at less twice of the debounce time
    //DBG("Set short time:%i ms\n\r",short_time);
    return short_time; 
}

uint8_t update_buts(uint8_t *buts,uint8_t *status){
    uint8_t r=0;
    //disable_interrupts();
    if(_last_finished>0 && (milis()-_last_finished)>space_time) { // Transfer clicks
        //memcpy(buts,_tmp_clicks,4);
        *(uint32_t *)buts=*((uint32_t *)_tmp_clicks);
        *status |= 1; // set peding clean in status
        reset_buts();
        DBG("[TR]");
        r=1;
    } else {        
        if(_started[PWR_BUT]>0 && (milis()-_started[PWR_BUT]) > hold_time) {
            DBG("[H:%lu]",hold_time);
            if(IS_LOW(READ_BUT(),PWR_BUT)) r=2;
            else _started[PWR_BUT]=0;
        } 
    }
    //enable_interrupts();
    return r;
}
void reset_buts() {

    memset(_tmp_clicks,0,4*sizeof(uint8_t));
    memset(_started,0,2*sizeof(uint32_t));
    _last_started=_last_finished=0;
    _prev=READ_BUT();

}

void update_outs(uint8_t o) {
    // PP output on PC 4
    if(o) PC_ODR |= (1<< OUTPUT_PIN); else PC_ODR &= ~(1<< OUTPUT_PIN);   
}

#pragma opt_code_balanced
void but_isr() __interrupt(EXTI2_ISR) {
    uint8_t     s=READ_BUT();
    uint8_t     chg= s ^ _prev; // chg bit will 1 if changed
    //uint32_t    now=milis();
    uint32_t    t;

    // IRQ PIN change
    if( (chg & (1 << IRQ_PIN )) && (s & (1<< IRQ_PIN) ) ) vpi_regs.status |= VPI_IRQ_FLAG; 
    // Itereate over two buttons
    for(uint8_t i=0;i<2;i++) { 
        // Check events
        if(!HAS_CHANGED(chg,i)) continue;
        if(IS_LOW(s,i)) { // click Started
            DBG("D:%i,%lu",i,_milis);
            _started[i]=milis();
            //_last_started=milis();
        } else if(IS_HIGH(s,i)) {
            DBG("U:%i,%lu",i,_milis);
            if(_started[i]>0 && (t=(uint32_t)(_milis-_started[i]))>DEBOUNCE_MS) { // if time > debounce
                if(t <= short_time) _tmp_clicks[i][BUT_SHORT]++; else _tmp_clicks[i][BUT_LONG]++;
                _last_finished=milis();
            } 
            _started[i]=0; // reset click start
        } 
    }
    _prev=s; // Save acutal reg
}