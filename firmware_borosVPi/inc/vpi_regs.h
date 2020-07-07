/**
 * @file vpi_regs.h
 * @author LDV
 * @brief Definition of VPI registers and helper macros.
 * @version 0.1
 * @date 2019-05-01
 * 
 * @copyright Copyright (c) 2019
 * 
 */
#ifndef _VPI_REGS_H_
#define _VPI_REGS_H_

#include<stddef.h>
#include<stdint.h>

typedef struct VPIREGS {
  // Read only section
  uint8_t	  id;		        ///< [RO] ID of the chip usefull to check if present. Value defined in MAGIK
  uint8_t   v;		        ///< [RO] Version sequential
  uint8_t   status; 	    ///< [RO] Status register    7 6 5 4 3 2 1 0
                          ///<                         1 0 I W B E R C
                          ///<                         E: Error present.
                          ///<                         R: RPM updated.
                          ///<                         C: Clicks pending.
                          ///<                         B: Running
                          ///<                         W: Watchdog enabled
                          ///<                         I: Interrupt pending
  uint8_t   flags;        ///< [RO] additional status flags 7 6 5 4 3 2 1 0
                          ///<                              1 0 X X X O W I
                          ///<                              I: Wake by IRQ enabled
                          ///<                              W: Wake enabled
                          ///<                              O: Ouput state
  uint8_t   crc;          ///< [RO] CRC8 of configuration registers
  uint8_t   buts[2][2];	  ///< [RO] Button clicks
  uint16_t  rpm;          ///< [RO] Fan estimated RPM
  uint8_t   err_count;    ///< [RO] Error count flags (for debugging)

  uint8_t   uuid[12];     ///< [RO] 96bit Unique ID.

  // RW section (Configuration and commands)
  uint16_t  pwm_freq;     ///< [RW] desired pwm frequency for fan and led 250-625000Hz
  uint8_t   rev_divisor;  ///< [RW] Number of pulses per revolution divisor.
  uint8_t   wdg;          ///< [RW] Time in seconds for highlevel watchdog (Only used if )
  uint16_t  wake;         ///< [RW] Autowake in minutes 0=no autowake
  uint16_t  short_tm;     ///< [RW] Short click time ms (max)
  uint16_t  space_tm;     ///< [RW] Spacing time ms
  uint8_t   hold_tm;      ///< [RW] Hold Time s
  uint8_t   grace_tm;     ///< [RW] Shutdown grace time s
  uint8_t   led_mode;     ///< [RW] Led mode <normal, on , off, blink, cycle>
  uint8_t   led_val;      ///< [RW] Led value
  uint8_t   buzz_freq;    ///< [RW] Buzzer frequency <0=500Hz , 1=1Khz & 2=2khz>
  uint8_t   buzz_b_tm;    ///< [RW] Buzzer beep time. 1/10th seconds.
  uint8_t   buzz_p_tm;    ///< [RW] Buzzer pause time. 1/10th seconds.
  uint8_t   buzz_count;   ///< [RW] Buzzer number of beeps
  uint8_t   fan_val;      ///< [RW] Fan value 0-255
  
  uint8_t   cmd;          ///< [RW] Inmediate command register [cleared after execution]
  uint8_t   icmd;         ///< [RW] Integrity command should be cmd ^ magick
  
} VPiRegs;

/**
 * @brief VPI Commands
 *  NOP:  Do nothing.
 *  ACT:  Activate configuration: Will configure times, wdg, pwm frequency, rev_divisor.
 *  BOOT: Special action to notify the board that the Vpid has been started up correctly and the system is booted.
 *  INIT: Special action to notify the board to return to booting state.
 *  FEED: Feed watchdog. Will reset the wdg counter
 *  SHUT: Shutdown power with grace time configured in grace_tm
 *  HARD: Hard shutdown power inmediatly
 *  CLEAR: Clear clicks,RPM, Error and IRQ flag
 *  FAN:   Update fan with value in the register fan_val.
 *  LED:   Update Led with the values in registers
 *  BEEP:  Update beep.
 *  OUTSET: Set digital ouput
 *  OUTCL:  Clear digital ouput
 *  WAKE_EN:  Enable wake after minutes configured wake (minutes)
 *  WAKE_DI:  Disable wake.
 *  WAKE_IEN: Enable wake by low edge in IRQ line.
 *  WAKE_IDI: Disable wake by IRQ line.
 */
#define VPI_CMD_NOP     (0x00)
#define VPI_CMD_ACT     ('A')
#define VPI_CMD_BOOT    ('B')
#define VPI_CMD_INIT    ('I')
#define VPI_CMD_FEED    ('F')
#define VPI_CMD_HARD    ('H')
#define VPI_CMD_SHUT    ('S')
#define VPI_CMD_CLEAR   ('C')
#define VPI_CMD_FAN     ('N')
#define VPI_CMD_LED     ('L')
#define VPI_CMD_BEEP    ('Z')
#define VPI_CMD_OUTSET  ('1')
#define VPI_CMD_OUTCL   ('0')
#define VPI_CMD_RESET   ('T')
#define VPI_CMD_WDGSET  ('W')
#define VPI_CMD_WDGRST  ('V')
#define VPI_CMD_WEN     ('E')
#define VPI_CMD_WDI     ('D')
#define VPI_CMD_IEN     ('e')
#define VPI_CMD_IDI     ('d') 

// BUTTONS
#define BUT_PWR         (0)
#define BUT_AUX         (1)
#define BUT_SHORT       (0)
#define BUT_LONG        (1)

// BOUNDARIES
#define VPI_LAST_REG	    ( offsetof(VPiRegs,icmd)     )
#define VPI_FIRST_WREG    ( offsetof(VPiRegs,pwm_freq) )
#define VPI_CONFIG_LEN    ( offsetof(VPiRegs,cmd) - VPI_FIRST_WREG )

// FLAGS
#define VPI_CLICK_FLAG     ( 1 )
#define VPI_RPM_FLAG       ( 2 )
#define VPI_ERROR_FLAG     ( 4 )
#define VPI_RUNING_FLAG    ( 8 )
#define VPI_WDG_FLAG       ( 16)
#define VPI_IRQ_FLAG       ( 32)
#define VPI_FIXED_FLAG     ( 0x80 )
#define VPI_WAKEENI_FLAG   ( 1 )
#define VPI_WAKEEN_FLAG    ( 2 )
#define VPI_OUT_FLAG       ( 4 )
#define VPI_HAS_CLICK(r)   ( (r).status & VPI_CLICK_FLAG  )
#define VPI_HAS_RPM(r)     ( (r).status & VPI_RPM_FLAG    )
#define VPI_HAS_ERROR(r)   ( (r).status & VPI_ERROR_FLAG  )
#define VPI_IS_RUNING(r)   ( (r).status & VPI_RUNING_FLAG )
#define VPI_HAS_WDG(r)     ( (r).status & VPI_WDG_FLAG    )
#define VPI_HAS_IRQ(r)     ( (r).status & VPI_IRQ_FLAG    )
#define VPI_HAS_WAKEEN(r)  ( (r).flags  & VPI_WAKEEN_FLAG )
#define VPI_HAS_WAKEENI(r) ( (r).flags  & VPI_WAKEENI_FLAG )
#define VPI_CLEARALL(r)    ( (r).status = VPI_FIXED_FLAG   )
#define VPI_CLEARFLA(r)    ( (r).flags  = VPI_FIXED_FLAG   )
#define VPI_CLEAR(r)       ( (r).status &= ~(VPI_CLICK_FLAG | VPI_RPM_FLAG | VPI_IRQ_FLAG | VPI_ERROR_FLAG) )
#define VPI_CLEARE(r)      ( (r).status &= ~(VPI_ERROR_FLAG) )
#define VPI_CLEARI(r)      ( (r).status &= ~(VPI_IRQ_FLAG) )


// Default I2C Address ChangeMe to chose other address
#define VPI_I2C_ADDR      (0x33)
#define VPI_DEVICE_MAGIK  (0xAA)

#endif
