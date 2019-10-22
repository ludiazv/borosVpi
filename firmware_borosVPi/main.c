/**
 * @file main.c
 * @author LDV
 * @brief Main VPI firmawara
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
void dummy_isr() __interrupt(29) __naked { ; }

/** MAIN Progam block */
typedef enum VpiStatus {
    BOOTING,                ///< Booting state wating to configuration and boot command.
    RUNNING,                ///< Running state.
    SHUTDOWN,               ///< Shutdown procedure started, now in grace time.
    WDOG,                   ///< High level watchdog trigered, now in grace time.
    OFF,                    ///< System is OFF
} VpiStatus_t;

static VpiStatus_t status;           ///< State machine of the main program
static uint16_t    shutdown_started; ///< Used for shutdonw safe sequence and WDOG

/*volatile VPiRegs vpi_regs;*/

/**
 * @brief Move the state machine to a specific state. The trasisiton will execute 
 * 
 * @param newstatus new status of the board 
 */
void transitionTo(VpiStatus_t newstatus) {

    if(newstatus==BOOTING) { 
        PWR_ON();                    // Assure power on
        //update_led(LED_MODE_CY,0); // LED mode cycle for booting updated in main loop
        update_fan(255);             // FAN MAX on booting
        vpi_regs.fan_val=255;        // Set the fan status in the control registers
        shutdown_started =0;         // Clear 
        vpi_regs.status=0;           // Clear all flags
        reset_buts();                // Cleanr any pending clicks

    }else if(newstatus==RUNNING) {
        PWR_ON();                    // Assure power on
        shutdown_started=0;
        VPI_CLEAR(vpi_regs);
        reset_buts();
        // Led mode=desired (done in main loop)
        update_led(LED_MODE_ON,0);
        vpi_regs.led_mode=LED_MODE_ON;
        // fan 50% (don in min loop)
    }else if(newstatus==SHUTDOWN) {
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
        vpi_regs.status=0;          // Clear all flags
        reset_buts();
    }
    status=newstatus;
    DBG("STATUS:%i\n\r",newstatus);
}

/**
 * @brief 

#define VPI_CMD_BEEP    ('Z')
 */
void doCmd(uint8_t cmd) {
    if(cmd==VPI_CMD_NOP) return; // do nothing fast
    DBG("CMD:%i",cmd);
    switch(cmd) {
        case VPI_CMD_BOOT: // Just finished booting
            transitionTo(RUNNING);
            break;
        case VPI_CMD_SHUT: // Shutdown requested
            transitionTo(SHUTDOWN);
            break;
        case VPI_CMD_FEED: // Feed watchdog
            DBG("WDGF\n\r");
            feed_wdg();// Clear WDOG
            break;
        case VPI_CMD_HARD: // Hard shutdown request
            transitionTo(OFF);
            break;
        case VPI_CMD_ACT: // Activate configuration
            disable_interrupts(); // clean update
            vpi_regs.rev_divisor=set_rpm_divider(vpi_regs.rev_divisor); // rpm divider update dividir
            vpi_regs.pwm_freq=set_pwm_freq(vpi_regs.pwm_freq);          // activate frequency
            vpi_regs.short_tm=set_short_time(vpi_regs.short_tm);        // Set Short time
            vpi_regs.space_tm=set_space_time(vpi_regs.space_tm);        // Set spacing time
            vpi_regs.hold_tm= set_hold_time(vpi_regs.hold_tm);          // Set Hold time
            update_led(vpi_regs.led_mode,vpi_regs.led_val);             // Update led
            update_fan(vpi_regs.fan_val);                               // Update fan
            // set beep // TODO
            // set wgd 
            enable_interrupts();
            break;
        case VPI_CMD_CLEAR:
            VPI_CLEAR(vpi_regs);
            break;
        case VPI_CMD_CLEARE:
            VPI_CLEARE(vpi_regs);
            break;
        case VPI_CMD_LED:
            update_led(vpi_regs.led_mode,vpi_regs.led_val);
            break;
        case VPI_CMD_FAN:
            DBG("F %i",vpi_regs.fan_val);
            update_fan(vpi_regs.fan_val);
            break;
        case VPI_CMD_OUTS:
            update_outs(vpi_regs.outs);
            break;
        case VPI_CMD_BEEP:
            // TODO
            
            break;
        //default:
            // Other case interpreted as NOP
        //    return;
    }
    // Clear command after exectuion
    //disable_interrupts();
    vpi_regs.cmd=VPI_CMD_NOP;
    //enable_interrupts();
}


void main() {
    
    // Initilization phase
    disable_interrupts();       // Dont interrupt in init
    init_clocks();              // Configure clocks and timebase
    DBG_INIT();                 // Init uart for debuggin
    DBG(" \n\rVPI Booting...\n\r");     // Boot banner.
    init_gpio();                // Configure GPIO with init value
    reset_i2c_regs();           // Reset values to registers
    init_i2c();                 // Configure i2c slave.
    init_pwm();                 // Configure pwm outpust
    
    transitionTo(BOOTING);      // On boot the stat
    
    enable_interrupts();    // Go
    
    DBG("VPI booted version:%i\n\r",VPI_VERSION);
    
    // Main Loop
    uint8_t tmp;
    uint16_t s=seconds();

    while(1) {
        transaction_wait();    // Wait if i2c_transaction is in progress.
        doCmd(_GET_COMMAND()); // First execute commands
        #if VPI_DEBUG
        if(s!=seconds()) {
            DBG(".");
            s=seconds();
            if(VPI_HAS_CLICK(vpi_regs)) {
                DBG("Click reg:%04X , %04X\n\r", *((uint16_t *)&vpi_regs.buts[0][0]),
                                                 *((uint16_t *)&vpi_regs.buts[1][0]) );
            }
        }
        #endif
        // State Machine
        if(status==BOOTING) {
            update_led(LED_MODE_CY,0);          // Udapdate shutdow led
            tmp=update_buts((uint8_t *)vpi_regs.buts,&vpi_regs.status); // Process buton status
            if(tmp==2) transitionTo(OFF);       // hard shutdown in booting
        
        }else if(status==RUNNING) {    
            u_led(); // update led
            tmp=update_buts((uint8_t *)vpi_regs.buts,&vpi_regs.status);
            if(tmp==2) transitionTo(OFF); // HARD detection
            if(wdg_check(vpi_regs.wdg)) transitionTo(WDOG);

        }else if(status==SHUTDOWN) {
            
            update_led(LED_MODE_FCY,0); // Udapdate shutdow led
            if((seconds()-shutdown_started)>vpi_regs.grace_tm) transitionTo(OFF); // Shutdown

        }else if(status==OFF){
            // start detection -> BOOTING
            tmp=update_buts((uint8_t *)vpi_regs.buts,&vpi_regs.status);
            if(tmp==1 && vpi_regs.buts[PWR_BUT][BUT_LONG]>0) transitionTo(BOOTING);

        } else if(status==WDOG){
            //update_led(LED_MODE_FBLINK,0); // udate led
            u_led();
            if((seconds()-shutdown_started)>5) transitionTo(BOOTING); // Watchdog will reboot after 5 seconds

        } else transitionTo(BOOTING);// falback

        // Err handling
        if(_HAS_I2C_ERROR()) { // Restart the I2C Error interface
            vpi_regs.err_count++;   // Add Error Count
            disable_interrupts();
            init_i2c();             // Reinit i2c interface
            enable_interrupts();
        }
        wfi();              // Wait for next irq: idle will restart every ms.

    }

}