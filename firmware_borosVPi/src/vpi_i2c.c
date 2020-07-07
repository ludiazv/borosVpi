#include<vpi_i2c.h>
#include<string.h>
#include<dbg.h>
#include<clock.h>
#include<crc.h>



// Globals
volatile uint8_t   i2c_error;               /// Error notificator
volatile VPiRegs   vpi_regs;                /// The I2C internal registers
volatile uint16_t  i2c_last_transaction;    /// Last transaction seconds
volatile uint8_t   i2c_last_reg;            /// Last requested register

// Private
volatile static uint8_t i2c_reg_index;      // pointer to registers
volatile static uint8_t i2c_just_match;     // state machine status for just match
volatile static uint8_t i2c_transaction;    // i2c transaction in progress.

/**
 * @brief load the default values to I2C registers
 * 
 */
void reset_i2c_regs() {
    memset(&vpi_regs,0,sizeof(VPiRegs)); // Reset to 0.
    vpi_regs.id=VPI_DEVICE_MAGIK; // ID
    vpi_regs.v=VPI_VERSION;     // Version
    vpi_regs.pwm_freq=25000;     // default pwm frqu
    vpi_regs.short_tm=200;       // short click
    vpi_regs.space_tm=1200;    // Long click
    vpi_regs.hold_tm=8;          // Hold type
    vpi_regs.grace_tm=15;        // grace time
    vpi_regs.rev_divisor=2;     // divisor to 2 as default
    memcpy(vpi_regs.uuid,(uint8_t*)0x4865,12); // Copy Chip unique ID.
    vpi_regs.cmd=VPI_CMD_ACT;   // Force default values on startup
    vpi_regs.icmd=VPI_CMD_ACT ^ VPI_DEVICE_MAGIK; // Complement
}

/** @brief Init I2c Slave interface
 * 
 */
void init_i2c() {
   
    // Reset the peri and configure it
    I2C_CR1 =0;  // Disable periphericas (allso enable stretch & disable general call)
    //PB_ODR|=(1<<5 | 1<< 5);
    //PB_CR1&=~(1<<4 | 1 << 5);   // Set PB4/PB5 float input 
    PB_DDR|=(1<<4 | 1 << 5);   // Set PB4/PB5 
    PB_ODR|=(1<<4 | 1 << 5);
    PB_CR2|=(1<<4 | 1 << 5);
    //CLK_PCKENR1 &=~(1);
    //CLK_PCKENR1 |=1;
    //I2C_CR2 |= (1<<7);

    I2C_FREQR = 16;                     //  Set the internal clock frequency (MHz)=> 16Mhz
    //I2C_CCRL = 0xa0;                    //  SCL clock speed is 100Khz. (will have no efect)
    //I2C_CCRH = 0;                       //  Reset I2C mode and duty cycle

    I2C_OARL = (VPI_I2C_ADDR << 1);     // Set Slave address set Addrress + ADD0. ADD0=0 7-bit addrs
    I2C_OARH = 0b01000000;              // bit 6 must be 1 as of RM0016
    // Unactive tweeks
    //I2C_TRISER = 17;
    I2C_ITR= 0b111; // Enable all I2C events (BUFFER, EVENT & ERROR)


    // Set I2C interrupt with priority Level3 (b11) i2c interrupt
    // ISR  for I2C is 19 -> ITC_ISPR5 (19,18,17,16)
    ITC_ISPR5 |= ( 0b11 << 6 ); // set priority 0=0b10
    
    //I2C_CR2 &= ~(1<<7);
    //CLK_PCKENR1 |=1;
    I2C_CR1 |= (1<<I2C_CR1_PE);   // Enable periphal
    I2C_CR2 |= (1<<I2C_CR2_ACK);  // Enable ACK respond
    //if(I2C_SR3 & 1) I2C_CR2 |= (1<<7);
    

    i2c_reg_index=0;   // Reset state control flags (reg pointer & just_match)
    i2c_just_match=0;  // No match
    i2c_transaction=0; // No transaction
    i2c_last_transaction=seconds(); // Reset last transacion time
    DBG("[2CS]");

}

uint8_t  in_transaction() {
    return i2c_transaction;
}

void transaction_wait() {
    uint32_t now=milis();
    while(i2c_transaction && (milis()-now)<5); // Bussy wait
}

/** @brief compute vpi configuration registers crc
 */
void regs_crc() {
    vpi_regs.crc=compute_crc( ((uint8_t *)vpi_regs) + VPI_FIRST_WREG , VPI_CONFIG_LEN);
}

/**
 * @brief ISR for I2C is hing priority autonumous routine that reads and write register.
 *        no logic is implemented here in order to be fast an respond as quick as possible
 *        this is necesarry as RPI has faulty clockstreching capabilitis and alos enable
 *        the device to respond to fast I2C.
 * 
 *  Implementation:
 *      - If address is match: Clear flags and set just match flag and transaction in progress.
 *      - If master wants to read: send byte pointed by reg_index.
 *      - If master wants write: if just match flag set the write is considerd to be the register to write
 *                               if not write to register pointer.
 *      - register index is incremented to bulk read or writes. If index is out of boundaries the index will rotate.
 *      - Stop condition & NACK flag ar also managed and cleared.
 *      - Trasaction control: When address is match the transaccion flag is set. It is cleared if:
 *          * Stop condition detected.
 *          * NACK condition detected.
 *          * Error condition detected.
 * 
 */
#pragma opt_code_speed
void i2c_isr() __interrupt(I2C_ISR) {
	
  uint8_t dummy;//=I2C_SR1; // required for reading
   
  // Match event : Match , write or read (exclusive)
  if(_ISMATCH()) {
       //DBG2_ON();
       dummy=I2C_SR1; // To clear flag as RM0016
       dummy=I2C_SR3; // To clear Flag
       I2C_CR2 |= (1 << I2C_CR2_ACK); // Assure ACK is activated (possible not necesary)
       i2c_just_match=1;   // Flag that just matched address. Next write event will be treated as firs byte.
       i2c_transaction=1;  // Transaction just begin.
       i2c_last_transaction=seconds(); // Set last transaction
       i2c_error= 0;       // Clear all errors
       //DBG2_OFF();
       //return;      
   } // Master has writes
   else if(_ISWRITE()) {
         //DBG2_ON();
         if(i2c_just_match) { // The first write is the register
             i2c_last_reg = I2C_DR;
             i2c_reg_index= i2c_last_reg; // set register
             i2c_just_match=0; // Clear the register
         } else {
            if(i2c_reg_index>VPI_LAST_REG || i2c_reg_index<VPI_FIRST_WREG) i2c_reg_index=VPI_FIRST_WREG; // check boudaries
            _Reg(i2c_reg_index) = I2C_DR; // Load the value
            i2c_reg_index++; // inc pointer
         }
         //DBG2_OFF();
    } // Master reads
    else if(_ISREAD()) {
        //DBG2_ON();
        if(i2c_reg_index>VPI_LAST_REG) i2c_reg_index=0; //check boundaries
        I2C_DR = _Reg(i2c_reg_index);
        i2c_reg_index++; // inc register pointer
        //DBG2_OFF();
    }

    // Master stop!
    if(_ISSTOP()) {
        dummy=I2C_SR1; // required by RM0016
        I2C_CR2 = 0b100; // Required by RM0016 to clear the stop flag (here we write ACK activeted)
        i2c_just_match=0; // assure state flag is cleared
        i2c_transaction=0; // clear any pending trasaction
    }

    // Master nacked. Ignore it and do not treat it as an error.
    if(_ISNACK()) {
        I2C_SR2 &= ~( 1 << I2C_SR2_AF); // clear AF Flag
        i2c_just_match=0; // Assure state flag is cleared.
        i2c_transaction=0; // Clear any pending transaction
    }

    // Other wise treat event as error
    // Error handling cases
    // -----------------------
    //DBG2_ON();
    i2c_error = I2C_SR2 & 0x0F;
    if(i2c_error) {
        dummy=I2C_SR1; // To clear flag as RM0016
        dummy=I2C_SR3; // To clear Flag
        i2c_just_match=0;  // Also clear internal flag
        i2c_transaction=0; // Clear any transaction
        I2C_SR2 &= ~(0x0F);  // clear al errors
    }
    //DBG2_OFF();
}


