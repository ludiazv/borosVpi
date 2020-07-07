/**
 * @file vpi_i2c.h
 * @author your name (you@domain.com)
 * @brief 
 * @version 0.1
 * @date 2019-07-25
 * 
 * @copyright Copyright (c) 2019
 * 
 */
#ifndef _VPI_I2C_H_
#define _VPI_I2C_H_

#include <stm8s.h>
#include "vpi_regs.h"
#include "clock.h"
#include "version.h"

// Some macros to help readeability mostly for internal use
#define _Reg(n)     ( ((uint8_t *)vpi_regs)[(n)]     )
#define _ISREAD()   ( I2C_SR1 & (1 << I2C_SR1_TXE)   )
#define _ISWRITE()  ( I2C_SR1 & (1 << I2C_SR1_RXNE)  )
#define _ISSTOP()   ( I2C_SR1 & (1 << I2C_SR1_STOPF) )
#define _ISMATCH()  ( I2C_SR1 & (1 << I2C_SR1_ADDR)  )
#define _ISNACK()   ( I2C_SR2 & (1 << I2C_SR2_AF)    )
#define _FREQ_CAL() ( (uint8_t)(F_CPU/1000000UL)     )  

#define _HAS_I2C_ERROR()    ( i2c_error !=0 )
#define _CLEAR_I2C_ERROR()  ( i2c_error =0  )
#define _GET_COMMAND()      ( vpi_regs.cmd  )
#define _GET_COMMANDI()     ( vpi_regs.icmd )
#define I2C_RECOVERY_TIME   2

// Prototipes
// ==============
void reset_i2c_regs();				 // Reset i2c_regs to factory values
void init_i2c();				     // init Routine
uint8_t in_transaction();            // check trasaction flag.
void transaction_wait();             // Bussy wait if transaction.		
void i2c_isr() __interrupt(I2C_ISR); // ISR must be included in main.c for ISR assigantion in SDCC
void regs_crc();                       // Compute CRC of registers	

// Globals
extern volatile  uint8_t   i2c_error; // I2C Error global
extern volatile  VPiRegs   vpi_regs;  // I2C Registers
extern volatile  uint16_t  i2c_last_transaction; // Last i2c transacion
extern volatile  uint8_t   i2c_last_reg; 


#endif