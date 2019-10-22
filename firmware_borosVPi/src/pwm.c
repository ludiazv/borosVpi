#include<stdint.h>
#include<pwm.h>
#include<util.h>
#include<dbg.h>


/**
 * @brief Simple PWM implementation with timer 2 and some extras
 *  
 *   Signal generation using TIM2:
 *      - Resolution: 8bit resolution.
 *      - Freq: Use a prescaler 1 will have a range from 244Hz to 62.5kHz (with 8bit relution) caped to 250Hz->65.2Kh
 *              as it use no prescaler on a 16Mhz frecuency.
 *          Frequency is is computed using this formula:
 *              
 *      - Polarity is Active high.
 *       
 * 
 *    Channels: 
 *       -Pin use PD3 TIM2 CH2 for led  (do not require opt byte as is defult output of the channel)
 *       -Fan use PA3 TIM2 CH3 for fan  (do not require opt byte as is default otput of the channel)
 *   
 *                                                                  
 * 
 */

static uint32_t _led_time;
static uint8_t  _curr_led_val,_curr_fan_val,_prsc,_curr_led_mode;
static int8_t   _led_dir;
static uint16_t _arr;


void init_pwm() {
    // Set GPIOS as ouputs push pull default high
    /* PD_ODR |= (1<<3);
    PD_DDR |= (1<<3);
    PD_CR1 |= (1<<3); 
    
    PA_ODR &= ~(1<<3);
    PA_DDR |= (1<<3);
    PA_CR1 |= (1<<3); */
    
    //Configure the timer
    TIM2_CR1= 0;    // Disable counter
    TIM2_PSCR = 0;  // No prescaler
    _prsc=0;
    _arr=640;
    TIM2_ARRH = (_arr >> 8);  // Default at 25Khz H must be load first
    TIM2_ARRL = _arr & 0x00FF;
    TIM2_CCR2H  = 0;   // Duty 0% H byte must be loaded first
    TIM2_CCR2L  = 0;
    TIM2_CCR3H  = 0;
    TIM2_CCR3L  = 0;


    // Configure channels
    //TIM2_CCER1 = (0<< 5) | ( 1<< 4); // Polarity High & channel as output for channel 2
    //TIM2_CCER2 = (0<< 1) | ( 1<< 0); // Polarity High & channel as output for channel 3 
    TIM2_CCER1 = ( 1<< 4); // Polarity High & channel as output for channel 2
    TIM2_CCER2 = ( 1<< 0); // Polarity High & channel as output for channel 3 
    TIM2_CCMR2 = (0b110 << 4);       // Mode PWM-1 for channel 2
    TIM2_CCMR3 = (0b110 << 4);       // Mode PWM-1 for channel 3


    set_bit(TIM2_CR1,TIM2_CR1_CEN); // enable timer
    _led_time=0;      // init control state
    _curr_led_val=0;
    _curr_led_mode=LED_MODE_OFF;
    _curr_fan_val=0;
    _led_dir=1;
    DBG("PWM started\n\r");
}

// TODO: Refactor to change prescaler and accept lower frequencies to 250Hz
uint16_t set_pwm_freq(uint16_t freq) {
    
    uint16_t tarr;
    uint8_t  prsc;

    // Limits
    if(freq< 2) freq=2;
    if(freq> 62500) freq=62500;

    // Adjust prescaler ranges
    if(freq> 31000) { // Freq > 31Khz
        tarr=F_CPU/freq;
        prsc=0;
    } else if(freq> 15000) { // Freq> 15 Khz
        tarr=(F_CPU/2)/freq;
        prsc=1;
    } else if(freq > 7000) { // Freq> 7Khz
        tarr=(F_CPU/4)/freq;
        prsc=2;
    } else if(freq > 3000 ) { // Freque > 3Khz
        tarr=(F_CPU/8)/freq;
        prsc=3;
    } else if(freq > 1500) { // Freq > 1.5Khz
        tarr=(F_CPU/16)/freq;
        prsc=4;
    } else if(freq > 900) { // Freq > 900 Hz 
        tarr=(F_CPU/32)/freq;
        prsc=5;
    } else if(freq > 450) { //Freq > 450 Hz
        tarr=(F_CPU/64)/freq;
        prsc=6;
    } else {  // Freq <= 450Hz
        tarr=(F_CPU/128)/freq;
        prsc=7;
    }

   
    if(tarr!=_arr || prsc!=_prsc) {
        _arr=tarr;
        _prsc=prsc;
        clear_bit(TIM2_CR1,TIM2_CR1_CEN); // Disable timer
        TIM2_PSCR = _prsc;
        TIM2_ARRH = (_arr >> 8);          //  H must be load first
        TIM2_ARRL = _arr & 0x00FF;
        set_bit(TIM2_CR1,TIM2_CR1_CEN);  // enable timer
    }

    DBG("PWM freq=%i [prsc=%i,arr=%i]\n\r",freq,_prsc,_arr);
    return freq;
}

inline uint16_t adj_pwm(uint8_t d) {
    uint32_t cc;
    //cc= (d==255) ? _arr+1 : ((_arr+1)*d)/255;
    //SDDC did not liked this so we help the compiler
    if(d==255) {
        cc=_arr+1;
    } else if(d==0) {
        return 0;
    } else {
        cc= (uint32_t)(_arr+1)*(uint32_t)d;
        cc/=255;
    }
    return (uint16_t) cc;
}

void set_led_val(uint8_t d) {
    uint16_t cc;
    if(_curr_led_val==d) return;
    cc=adj_pwm(d);
    TIM2_CCR2H  = (((uint16_t)cc) >> 8);   // Duty  H byte must be loaded first
    TIM2_CCR2L  = ((uint16_t)cc) & 0x00FF;
    _curr_led_val=d;
}

void set_fan_val(uint8_t d) {
    uint16_t cc;
    cc= adj_pwm(d);
    TIM2_CCR3H  = (cc >> 8);   // Duty  H byte must be loaded first
    TIM2_CCR3L  = cc & 0x00FF;
    _curr_fan_val=d;
}

uint8_t update_led(uint8_t mode,uint8_t val) {
    uint32_t now=milis();
    uint16_t t=0;

    if(mode==LED_MODE_BLINK)  t=550;
    if(mode==LED_MODE_FBLINK) t=175;
    if(mode==LED_MODE_CY)     t=8;
    if(mode==LED_MODE_FCY)    t=2;   

    // Modes ON -> 100% duty
    switch(mode) {
        case LED_MODE_ON:
            val=0;
            set_led_val(val);
            break;
        case LED_MODE_OFF:
            val=255;
            set_led_val(val);
            break;
        case LED_MODE_BLINK:
        case LED_MODE_FBLINK:
            if((now-_led_time)>t) { // change estatus
                val = (_curr_led_val == 0 ) ? 255 : 0;
                set_led_val(val);
                _led_time=now;
            }
            val=_curr_led_val; 
            break;
        case LED_MODE_CY:
        case LED_MODE_FCY:
            if((now-_led_time)>t) { // change estatus
                if(_curr_led_val==0) _led_dir=1;
                if(_curr_led_val==255) _led_dir=-1;
                set_led_val(_curr_led_val+_led_dir);
                _led_time=now;
            }
            val=_curr_led_val; 
            break;
        //case LED_MODE_CUSTOM:
        default:
            set_led_val(val);
            break;
    }
    _curr_led_mode=mode;
    return val;
}
void u_led() {
    update_led(_curr_led_mode,_curr_fan_val);
}

void update_fan(uint8_t val) {
    if(val!=_curr_fan_val) set_fan_val(val);
}