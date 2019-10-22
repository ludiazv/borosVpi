#include "clock.h"

/// =================================================================
/**
 * @brief Clock selection, tunning timebase generation and tachometer.
 *      - Run from internal clock high speed HSI (default at boot)
 *      - 16Mhz master clock
 *      - No CPU clock divider F_CPU = 16 Mhz
 *      - Disable clock for unused perifericals: TIM1,SPI,ADC & AWU
 *
 *   Time Base generation with arduino like milis
 * -------------------------------------------------
 *   Time base = 16Mhz / (2*prescaler*(ARR+1)) = 1000Hz (1ms period)
 *               16000000/2000 = p * (ARR+1)
 *               8000 = p * a
 *        p= 128 => a= 8000 / 128 = 62.5 ! not integer
 *        p= 64  => a= 8000 / 64 = 125 -> ARR = 124 (chosen) 
 *        p= 32  => a= 8000 / 32 = 250 -> ARR = 249
 * 
 *   Tachometer fan measurement is estimation counting the ticks 
 *   every second and extrapolate mesure perminute. 
 *      RPM= (tick in 1s/ticks per rolution)*60                 
 */

volatile uint32_t                   _milis;         ///< Milis counter
//#define  milis()                    (_milis)
volatile uint16_t                   _seconds;       ///< Second counter
//#define seconds()                   (_seconds)
volatile static uint16_t            _rpmtick;       ///< Counter of tachometer pulses
static uint8_t                      _rpmdivider;    ///< RPM divider proxied from registers
volatile static uint8_t             _wdg_counter;   ///< WDG counter

/**
 * @brief Set the rpm divider for rpm computation.
 * 
 * @param d u8  divider to use.
 * @return uint8_t divider set
 */
uint8_t set_rpm_divider(uint8_t d){ 
    _rpmdivider= (d==0) ? 2 : d;
    return _rpmdivider;
}
void feed_wdg() {
    _wdg_counter=0;
}
uint8_t wdg_check(uint8_t limit) {
    return limit>0 && _wdg_counter>limit;
}

/**
 * @brief delay
 * 
 * @param ms u16 time to delay in miliseconds
 */
void delay(uint16_t ms){
    if(ms==0) return;
    if(ms<=2) delay_ms(ms);
    uint32_t d=milis();
    while((milis()-d)<ms);
}

/**
 * @brief init_clocks: Initilize clock and time base using timer 4.
 *  Implementation:
 *      - Use HSI with no divisor for system bus master and CPU (16Mhz)
 *      - Disable unused perifericals (will save some power)
 *      - Program TIM4 tor produce a tick signal every 1ms => 1Khz
 *          Formula: freq=16Mhz / (2 * prescaler * (ARR+1) ).
 *          with prescaler 64=>  1000=16000000/(128*(ARR+1)) 
 *                               (ARR+1)=16000000/128000 = 125 => ARR=124.
 * 
 *      - Se
 */
void init_clocks() {

    CLK_ICKR = 0;                       //  Reset the Internal Clock Register.
    CLK_ICKR |= (1<< CLK_ICKR_HSIEN);   //  Enable the HSI.
    CLK_ECKR = 0;                       //  Disable the external clock.
    while ( !(CLK_ICKR & (1<<CLK_ICKR_HSIRDY)) );  //  Wait for the HSI to be ready for use.
    CLK_CKDIVR = 0;                     // No HSI divider & and CPU clock = Master = 16mHZ

    // Clock enable to periphericals (disable all )
    // (TIM1,TIM3,TIM2,TIM4,UART1,UART2,SPI,I2C)
    //   0    0    1    1    1      1    0   1
    //CLK_PCKENR1 = 0xff;
    CLK_PCKENR1 = 0b00111101;
    // (R,R,R,R,ADC,AWU,R,R)
    //  0 0 0 0  0   0  0 0
    CLK_PCKENR2 = 0;

        // In CCO mode output Master clock to CCO output
#if defined(VPI_CCO)
    // (R,R,BUSY,READY,CCOSEL[3:0], ENABLE)
    //  0 0  0     0     1100         1
    CLK_CCOR = 0b11001;
#else
    CLK_CCOR = 0; // disable CCO
#endif

    // Automatic clock switch procedure
    CLK_HSITRIMR = 0;                   //  Turn off any HSIU trimming.
    CLK_SWIMCCR = 0;                    //  Set SWIM to run at clock / 2.
    CLK_SWR = 0xe1;                     //  Use HSI as the clock source.
    CLK_SWCR = 0;                       //  Reset the clock switch control register.
    CLK_SWCR |= ( 1 << CLK_SWCR_SWEN);  //  Enable switching.
    while ( CLK_SWCR & (1 << CLK_SWCR_SWBSY) );        //  Pause while the clock switch is busy.


    // Time base
    /* Prescaler 64 & ARR=124 */
    TIM4_PSCR = 0b00000110;
    TIM4_ARR = 124*2; 
    TIM4_IER |= (1 << TIM4_IER_UIE); // Enable Update Interrupt
    TIM4_CR1 |= (1 << TIM4_CR1_CEN); // Enable TIM4

    // Set TIMER interrupt with priority 2 (b00) TIM4 interrupt
    // ISR  for TIM4 is 23 -> ITC_ISPR6 (23,22,21,120)
    ITC_ISPR6 &= ~( 0b00 << 6); // clear
    
     // Init all control variables
    _milis=0;
    _seconds=0;
    _rpmtick=0;
    _rpmdivider=2;  // default 2 ticks per rpm
    _wdg_counter=0; // wdg_counter
}

/**
 * @brief Time base iterruput rutine.
 *   Timer4 overflow every 1ms and millis tick variable is incremented.
 *   a second variable _seconds is also incremented also every 1000 ms.
 *   each second fan speed is computed counting the ticks of the tackomenter
 *   in one second.
 * 
 *   Timer ISR have more priority than EXTI interrupt rpmtick will not be modified.
 *   vpi_reg divider can be modified in the I2C interrupt rutine therefore captured with 
 *   intettupts masked.
 * 
 */
void timer_isr(void) __interrupt(TIM4_ISR) {

    //PC_ODR ^= (1<<4); 
    _milis++;
    if((_milis % 1000)==0) {
        _seconds++;
        _wdg_counter++;
        if(_rpmtick>0) {
            vpi_regs.rpm = (_rpmtick/_rpmdivider)*60; // Compute rpm
            _rpmtick = 0; // reset counter
        }
    }
    TIM4_SR &= ~(1 << TIM4_SR_UIF); // Clear update flag
}

/**
 * @brief EXTI configured in PD port simply add to the counter of tachometer ticks.
 *    Tick counter is reset every 1 second in the timer_isr ISR rutine
 * 
 */
void tachometer_isr(void) __interrupt(EXTI3_ISR) {
     _rpmtick++;
}
/*
 * =============== END CLOCK ====================
 */







