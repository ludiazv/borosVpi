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
#include "beep.h"
#include "clock.h"


static uint16_t beep_time,pause_time;
static uint8_t  beep_counter;
static uint8_t  beep_or_pause;
static uint32_t last_time;

/** @brief Initialize beep peripherical */
void init_beep() {
    // Enable alternate function AFR7 for beeper
    //_SFR_(0x4803 + 0x00)= 0x80;
    //_SFR_(0x4803 + 0x01)= ~(0x80);
    // SEL [1:0] | EN | DIV[4:0]
    // SEL  00      0     1E
    BEEP_CSR =  0x1E; // Configure 32 divider. 3 option [2khz,1khz,500Hz]
    beep_counter=0;
}
/** @brief start beeping for n beeps with pause parameters. After calling this function
 *   `update_beep` must be called in the main loop to produce the correct beep sequence.
 * 
 *  @param freq   select beep frequency [0:2Khz,1:1Khz,3:500Hz]
 *  @param count  number of beeps to produce.
 *  @param b_time time (1/10th s) of beep duration.
 *  @param p_time time (1/10th s) of pause duration between beeps.
 *  
 */ 
void start_beep(uint8_t freq,uint8_t count,uint8_t b_time,uint8_t p_time) {
    BEEP_CSR &= 0x3F;        // Clear freq 0011 1111 -> 3F
    BEEP_CSR |= (freq << 6); // Set frequency mode (0,1,2)
    beep_counter = count;
    beep_time    = b_time * 100; // 1/10th s -> ms
    pause_time   = p_time * 100; // 1/10th s -> ms
    BEEP_ENABLE(); // Start beep
    DBG("BEEP:%02X",BEEP_CSR);
    last_time= milis();
    beep_or_pause=1; // beep started.
}

/** @brief Update beep status. This must be called in the main loop. Timing is not critical here
 *   a call every 100ms will suffice.
 */
void update_beep(){
    uint32_t dif;
    if(beep_counter==0) return; // No aditional beep
    dif= milis()- last_time;
    if(beep_or_pause) {  // We are beeping
        if(dif > beep_time) {
            BEEP_DISABLE(); // stop beep
            last_time= milis();
            beep_or_pause = 0; // Go to pause
            beep_counter--; // decreae number of beeps
        }
    } else { // We are paused.
        if(dif > pause_time) {
            BEEP_ENABLE(); // Start next beep
            last_time = milis();
            beep_or_pause = 1;
        }
    }
}
