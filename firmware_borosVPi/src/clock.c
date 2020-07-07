/**
 * @file clock.c
 * @author LDV
 * @brief   Bare Metal clock control for STM8S: Timebase, High-level watchdog & tachometer.
 * @details Relevant implementation details:
 *      - Configure the mcu to run from internal clock high speed HSI (default at boot)
 *      - 16Mhz master clock:No CPU clock divider F_CPU = 16 Mhz
 *      - Disable clock for unused perifericals: TIM1,SPI,ADC & AWU (reduce powe consumption)
 *      - Generate an arduino style time base using the following:
 * 
 *          Formula: freq=16Mhz / (2 * prescaler * (ARR+1) ).
 *          with prescaler 64=>  1000=16000000/(128*(ARR+1)) 
 *                               (ARR+1)=16000000/128000 = 125 => ARR=124.
 *
 *      - Tachometer is implemente via a interrupt counting the pulses of the fan.
 *        Every 1s the number of pulses are divided by the reolution divisor and extrapolated to RPM.
 *        RPM= (tick in 1s/ticks per rolution)*60
 *      - High level watch dog is implemented incrementing _wdg_counter each second. wdg_feed() & wgd_check(ms) can be used to
 *        reset and test the watchdog status.
 *      - High level wake control is implemented incr
 * @version 0.1
 * 
 * 
 * @copyright Copyright LDV (c) 2019
 * 
 */
#include "clock.h"

volatile uint32_t                   _milis;         ///< Milis counter
volatile uint16_t                   _seconds;       ///< Second counter
volatile uint16_t                   _minutes;       ///< Minute counter
volatile static uint16_t            _rpmtick;       ///< Counter of tachometer pulses
static uint8_t                      _rpmdivider;    ///< RPM divider proxied from registers
static uint8_t                      _wdg_limit;     ///< limit set for wdg
volatile static uint8_t             _wdg_counter;   ///< WDG counter
volatile static uint16_t            _wake_counter;  ///< Wake counter

/**
 * @brief Set the rpm divider for rpm computation. Controls Div by 0 potential error by assinging 2 to the divider
 *        if 0 is passed.
 * 
 * @param d uint8_t  divider to use.
 * @return  uint8_t divider set
 */
uint8_t set_rpm_divider(uint8_t d){ 
    _rpmdivider= (d==0) ? 2 : d;
    return _rpmdivider;
}
/**
 * @brief Resets watchdog counter by setting to 0 the _wdg_counter variable.
 */
void feed_wdg() {
    _wdg_counter=0;
}
/**
 * @brief Start the watchdog
 * 
 * @param limit seconds, 0 -> disabled
 */
void wdg_start(uint8_t limit) {
    _wdg_limit=limit;
    _wdg_counter=0;
    DBG("[W:%i]",limit);
}
/**
 * @brief Check if the elapsed time since last call to feed_wdg()
 * 
 * @return uint8_t boolean if limit time has been reached.
 * @retval 0 watchdog did not reach the limit
 * @retval 1 watchdog did reach the limit.
 */
uint8_t wdg_check() {
    /*if(_wdg_limit>0) {
        DBG("{W%i,%i}",_wdg_limit,_wdg_counter);
        return VPI_HAS_WDG(vpi_regs) && _wdg_counter >  _wdg_limit;
    }*/
    return VPI_HAS_WDG(vpi_regs) && _wdg_limit> 0 && _wdg_counter >  _wdg_limit;
    //return VPI_HAS_WDG(vpi_regs) && limit>0 && _wdg_counter>limit;
}
/** @brief starts wake counter
 */
void start_wake() {
    _wake_counter = 0;
}
/** @brief Check if wake counter elapsed 
 *  @param limit minutes to test againts the wake counter
 *  @return uint8_t boolean if limit has been reached.
 *  @retval 0 wake limit han't been reached 
 *  @retval 1 wake limit has been reached
 */
uint8_t wake_check(uint16_t limit) {
    return limit>0 && VPI_HAS_WAKEEN(vpi_regs) && _wake_counter > limit;
}
/**
 * @brief bussy delay ms.
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
 *      - Set ISR for timer 4 to priority 2. This interrupt will be prioritary.
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
    // (R,R,R,R,ADC,AWU,R,R)  AWU is needed for BEEP
    //  0 0 0 0  0   0  0 0
    CLK_PCKENR2 = 0b100;

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
    _minutes=0;
    _rpmtick=0;
    _rpmdivider=2;  // default 2 ticks per rpm
    _wdg_counter=0; // wdg_counter
    _wake_counter=0; // wakecounter 
    _wdg_limit=0;    // wdg limit
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
 *   intettupts masked calling set_rpm_divider(uint8_t d).
 * 
 */
#pragma opt_code_speed
void timer_isr(void) __interrupt(TIM4_ISR) {

    //PC_ODR ^= (1<<4); 
    _milis++;
    if((_milis % 1000)==0) {
        _seconds++;
        _wdg_counter++;
        if(_rpmtick>0 && (_seconds & 1)) { // compute rpm every 2 seconds
            vpi_regs.rpm = (_rpmtick/_rpmdivider)*30; // Compute rpm
            vpi_regs.status |= VPI_RPM_FLAG; // Set flags
            _rpmtick = 0; // reset counter
        }
        if((_seconds % 60)==0) {
            _minutes++;
            _wake_counter++;
        }
    }
    TIM4_SR &= ~(1 << TIM4_SR_UIF); // Clear update flag
}
/**
 * @brief EXTI configured in PD port simply add to the counter of tachometer ticks.
 *    Tick counter is reset every 1 second in the timer_isr ISR rutine
 */
#pragma opt_code_speed
void tachometer_isr(void) __interrupt(EXTI3_ISR) {
     _rpmtick++;
}







