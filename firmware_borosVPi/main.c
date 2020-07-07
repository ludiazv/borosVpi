/**
 * @file main.c
 * @author LDV
 * @brief Main VPI firmaware file
 * @version 0.1
 * @date 2019-08-22
 * 
 * @copyright Copyright (c) 2019
 */
#include<dbg.h>
#include<gpio.h>
#include<beep.h>
#include<vpi_i2c.h>
#include<pwm.h>

/* Nacked interrupt required by bootloader */
void dummy_isr() __interrupt(29) /*__naked*/ { ; }

/** @brief Status of the main program automata */
typedef enum VpiStatus {
    BOOTING,                ///< Booting state wating to configuration and boot command.
    RUNNING,                ///< Running state.
    SHUTDOWN,               ///< Shutdown procedure started, now in grace time.
    WDOG,                   ///< High level watchdog trigered, now in grace time.
    OFF,                    ///< System is OFF
} VpiStatus_t;

static VpiStatus_t status;           ///< State machine of the main program
static uint16_t    shutdown_started; ///< Used for shutdonw safe sequence and WDOG

/**
 * @brief Move the state machine to a specific state. The trasisiton will execute
 *        basic changes in gpios and function inline with the tansition logic.
 * 
 * @param newstatus new status of the board.
 */
void transitionTo(VpiStatus_t newstatus) {
    DBG("{%i->%i}",status,newstatus);
    if(newstatus==BOOTING) { 
        PWR_ON();                    // Assure power on
        //update_led(LED_MODE_CY,0); // LED mode cycle for booting updated in main loop
        update_fan(255);             // FAN MAX on booting
        vpi_regs.fan_val=255;        // Set the fan status in the control registers
        shutdown_started =0;         // Clear 
        VPI_CLEARALL(vpi_regs);      // Clear all flags
        VPI_CLEARFLA(vpi_regs);
        reset_buts();                // Cleanr any pending clicks
        BEEP_DISABLE();

    }else if(newstatus==RUNNING) {
        PWR_ON();                    // Assure power on
        shutdown_started=0;
        i2c_last_transaction=seconds();
        vpi_regs.status |= VPI_RUNING_FLAG; // Running
        VPI_CLEAR(vpi_regs);
        reset_buts();
        // Led mode=desired (done in main loop)
        update_led(LED_MODE_ON,0);
        vpi_regs.led_mode=LED_MODE_ON;
        // fan 50% (done in min loop)
        #if defined(VPI_OUT_FOLLOW)
            update_outs(1);
            vpi_regs.flags |= VPI_OUT_FLAG;
        #endif

    } else if(newstatus==SHUTDOWN) {
        PWR_ON(); // Asure poweron
        shutdown_started=seconds();
        update_fan(255);            // FAN MAX speed on shutdown.
        //update_led(LED_MODE_FCY,0);  // LED mode cycle for booting updated in main loop
    } else if(newstatus==WDOG) {
        PWR_OFF();                       // Watchdog bite but still ON
        shutdown_started= seconds();    // Watchdog started period to reset
        update_fan(255);                // FAN MAX speed on WDG
        update_led(LED_MODE_FBLINK,0);  // LED mode fast blink for booting updated in main loop
    } else if(newstatus==OFF) {
        PWR_OFF();                  // turnoff
        update_led(LED_MODE_OFF,0);   // LED OFF
        update_fan(0);              // FAN OFF
        vpi_regs.status=VPI_FIXED_FLAG;          // Clear all flags
        BEEP_DISABLE();
        #if defined(VPI_OUT_FOLLOW)
            update_outs(0);
            vpi_regs.flags &= ~VPI_OUT_FLAG;
        #endif
        reset_buts();
        start_wake();
    }
    status=newstatus;
}

/**
 * @brief Execute commands written in the command register. Each command will have its onw logic. 
 *        Cmd register is cleared after exectuion.
 * @param cmd   desired command to execute. If the command is invalid it will be ignored.
 * @param icmd  check command must be cmd ^ VPI_CMD_NOP to execute
 */
void doCmd(uint8_t cmd,uint8_t icmd) {
     // Clear command before exectuion
    vpi_regs.cmd=VPI_CMD_NOP;
    if(cmd==VPI_CMD_NOP) return; 
    //DBG("C:%02X,I:%02X",cmd,icmd);
    if((cmd ^ VPI_DEVICE_MAGIK) != icmd) return; // do nothing fast
    DBG("{%c}",cmd);

    switch(cmd) {
        case VPI_CMD_BOOT: // Just finished booting
            transitionTo(RUNNING);
            break;
        case VPI_CMD_INIT: // Return to booting
            transitionTo(BOOTING);
            break;
        case VPI_CMD_SHUT: // Shutdown requested
            transitionTo( (status==OFF) ? BOOTING : SHUTDOWN );
            break;
        case VPI_CMD_FEED: // Feed watchdog
            feed_wdg();// Clear WDOG
            break;
        case VPI_CMD_HARD: // Hard shutdown request
            transitionTo(OFF);
            break;
        case VPI_CMD_ACT: // Activate configuration
            //disable_interrupts(); // clean update with interrupts masked
            vpi_regs.rev_divisor=set_rpm_divider(vpi_regs.rev_divisor); // rpm divider update dividir
            vpi_regs.pwm_freq=set_pwm_freq(vpi_regs.pwm_freq);          // activate frequency
            vpi_regs.short_tm=set_short_time(vpi_regs.short_tm);        // Set Short time
            vpi_regs.space_tm=set_space_time(vpi_regs.space_tm);        // Set spacing time
            vpi_regs.hold_tm= set_hold_time(vpi_regs.hold_tm);          // Set Hold time
            update_led(vpi_regs.led_mode,vpi_regs.led_val);             // Update led
            update_fan(vpi_regs.fan_val);                               // Update fan
            wdg_start(vpi_regs.wdg);
            regs_crc();
            //enable_interrupts();
            break;
        case VPI_CMD_CLEAR:
            VPI_CLEAR(vpi_regs);
            vpi_regs.err_count = 0;
            break;
        case VPI_CMD_LED:
            update_led(vpi_regs.led_mode,vpi_regs.led_val);
            regs_crc();
            break;
        case VPI_CMD_FAN:
            update_fan(vpi_regs.fan_val);
            regs_crc();
            break;
        case VPI_CMD_OUTSET:
        case VPI_CMD_OUTCL:
            icmd=(cmd==VPI_CMD_OUTSET); // Reuse icmd stack var for tmp
            update_outs(icmd);
            if(icmd) vpi_regs.flags |= VPI_OUT_FLAG; 
            else vpi_regs.flags &= ~VPI_OUT_FLAG;
            break;
        case VPI_CMD_BEEP:
            start_beep(vpi_regs.buzz_freq,vpi_regs.buzz_count,vpi_regs.buzz_b_tm,vpi_regs.buzz_p_tm);
            regs_crc();
            break;
        case VPI_CMD_RESET:
            SOFTWARE_RESET();
            break;
        case VPI_CMD_WDGSET:
            vpi_regs.status |= VPI_WDG_FLAG;
            regs_crc();
            wdg_start(vpi_regs.wdg);
            break;
        case VPI_CMD_WDGRST:
            vpi_regs.status &= ~(VPI_WDG_FLAG);
            feed_wdg();
            regs_crc();
            break;
        case VPI_CMD_WEN:
            vpi_regs.flags |= VPI_WAKEEN_FLAG;
            break;
        case VPI_CMD_WDI:
            vpi_regs.flags  &= ~(VPI_WAKEEN_FLAG);
            break;
        case VPI_CMD_IEN:
            vpi_regs.flags  |= VPI_WAKEENI_FLAG;
            break;
        case VPI_CMD_IDI:
            vpi_regs.flags  &= ~(VPI_WAKEENI_FLAG);
            //break;
        //default:
            // Other case interpreted as NOP
        //    return;
    }
   
}

/**
 * @brief Recover I2C interface when conection is lost
 *  
 *  @param timeout boolean to to indicate if timeout have been id  
 *     
 */
void recover_i2c(uint8_t timeout) {
        DBG("[I2CR E:%02X 1:%02X 2:%02X 3:%02X]",i2c_error,I2C_SR1,I2C_SR2,I2C_SR3);
        if(timeout) {
            if(I2C_SR3 & 0b110) SOFTWARE_RESET(); // Hard reset if TRA or MLS flags are set
            i2c_last_transaction=seconds();
        } else {
            disable_interrupts();
            // TODO: check i2_error
            init_i2c();             // Reinit i2c interface
            enable_interrupts();
        }     
}

/* Main program */
void main() {
    
    // Initilization phase
    disable_interrupts();       // Dont interrupt in init
    init_clocks();              // Configure clocks and timebase
    DBG_INIT();                 // Init uart for debuggin
    init_gpio();                // Configure GPIO with init value
    reset_i2c_regs();           // Reset values to registers
    init_i2c();                 // Configure i2c slave.
    init_pwm();                 // Configure pwm outputs
    init_beep();                // Enable beep
    
    transitionTo(BOOTING);      // On boot the stat
    
    enable_interrupts();        // Go
    
    DBG("[BOOT:%i]",VPI_VERSION); // Boot banner
    
    // Main Loop
    uint8_t tmp; // temporal byte used in the loop
#if VPI_DEBUG
    uint16_t s=seconds();
#endif

    while(1) {
        transaction_wait();    // Wait if i2c_transaction is in progress.
        doCmd(_GET_COMMAND(),_GET_COMMANDI()); // First execute commands

#if VPI_DEBUG
        if(s!=seconds()) {
            DBG(".");
            s=seconds();
            //if(VPI_HAS_CLICK(vpi_regs)) {
            //    DBG("Click reg:%04X , %04X\n\r", *((uint16_t *)&vpi_regs.buts[0][0]),
            //                                     *((uint16_t *)&vpi_regs.buts[1][0]) );
            //}
        }
#endif
        // State Machine
        if(status==BOOTING) {
            update_led(LED_MODE_CY,0);          // Udapdate shutdow led
            tmp=update_buts((uint8_t *)vpi_regs.buts,&vpi_regs.status); // Process buton status
            if(tmp==2) transitionTo(OFF);       // hard shutdown in booting
        
        }else if(status==RUNNING) {    
            u_led(); // update led
            update_beep();
            tmp=update_buts((uint8_t *)vpi_regs.buts,&vpi_regs.status);
            if(tmp==2) transitionTo(OFF); // HARD detection
            if(wdg_check()) transitionTo(WDOG);
            if((seconds() - i2c_last_transaction) > I2C_RECOVERY_TIME) recover_i2c(1);

        } else if(status==SHUTDOWN) {
            
            update_led(LED_MODE_FCY,0); // Udapdate shutdow led
            if((seconds()-shutdown_started)>vpi_regs.grace_tm) transitionTo(OFF); // Shutdown

        }else if(status==OFF){
            // start detection -> BOOTING
            tmp=update_buts((uint8_t *)vpi_regs.buts,&vpi_regs.status);
            if(tmp==1 && vpi_regs.buts[PWR_BUT][BUT_LONG]>0) transitionTo(BOOTING);
            // Wake process
            if(VPI_HAS_WAKEEN(vpi_regs)) {
                tmp=wake_check(vpi_regs.wake) || (VPI_HAS_WAKEENI(vpi_regs) && VPI_HAS_IRQ(vpi_regs));
                if(tmp) transitionTo(BOOTING);
            }

        } else if(status==WDOG){
            //update_led(LED_MODE_FBLINK,0); // udate led
            u_led();
            if((seconds()-shutdown_started)>5) transitionTo(BOOTING); // Watchdog will reboot after 5 seconds

        } //else transitionTo(BOOTING);// falback

        // Err handling
        if(_HAS_I2C_ERROR()) { // Restart the I2C Error interface
            vpi_regs.err_count++;   // Add Error Count
            vpi_regs.status |= VPI_ERROR_FLAG;
            recover_i2c(0);
            _CLEAR_I2C_ERROR();
        }
        wfi();              // Wait for next irq: idle will restart every ms or IO interrupts (I2C/click/tachometer)

    }

}