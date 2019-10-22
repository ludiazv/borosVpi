#include "beep.h"


static uint32_t beep_time,pause_time;
static uint8_t  beep_counter;


void init_beep(){
    BEEP_CSR =  0x1E | ( 1 << 6); // Configure 32 divider. At 16Mhz have 3 option [2khz,1khz,500Hz] and set 500Hzb
    beep_counter=0;
}


/*void update_beep(){

}*/