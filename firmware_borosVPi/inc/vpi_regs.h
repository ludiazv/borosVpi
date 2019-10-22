/**
 *
 */
#ifndef _VPI_REGS_H_
#define _VPI_REGS_H_

#include<stddef.h>
#include<stdint.h>

typedef struct VPIREGS {
  // Read only section
  uint8_t	  id;		        ///< [RO] ID of the chip usefull to check if present
  uint8_t   v;		        ///< [RO] Version sequential
  uint8_t   status; 	    ///< [RO] Status register    7 6 5 4 3 2 1 0
                          ///<                         x x x x x E R C
                          ///<                         E: Error present.
                          ///<                         R: RPM updated.
                          ///<                         C: Clicks pending.
  uint8_t   buts[2][2];	  ///< [RO] Button clicks
  uint16_t  rpm;          ///< [RO] Fan estimated RPM
  uint16_t  err_count;    ///< [RO] Error count flags (for debuggin)
  uint8_t   uuid[12];     ///< [RO] 96bit Unique ID.

  // RW section (Configuration and commands)
  uint16_t  pwm_freq;     ///< [RW] desired pwm frequency for fan and led 250-625000Hz
  uint8_t   rev_divisor;  ///< [RW] Number of pulses per revolution divisor.
  uint8_t   wdg;          ///< [RW] Time in seconds for highlevel watchdog 0=deactivated.
  uint16_t  short_tm;     ///< [RW] Short click time ms (max)
  uint16_t  space_tm;     ///< [RW] Spacing time ms
  uint8_t   hold_tm;      ///< [RW] Hold Time s
  uint8_t   grace_tm;     ///< [RW] Shutdown grace time s
  uint8_t   led_mode;     ///< [RW] Led mode <normal, on , off, blink, cycle>
  uint8_t   led_val;      ///< [RW] Lad value
  uint8_t   buzz_freq;    ///< [RW] Buzzer frequency <off, 2khz, 1Khz, 500Hz >
  uint8_t   buzz_b_tm;    ///< [RW] Buzzer beep time. centi seconds.
  uint8_t   buzz_p_tm;    ///< [RW] Buzzer pause time. centi seconds.
  uint8_t   buzz_count;   ///< [RW] Buzzer number of beeps
  uint8_t   outs;         ///< [RW] Ouputs
  uint8_t   fan_val;      ///< [RW] Fan value 0-255
  
  uint8_t cmd;            ///< [RW] Inmediate command register [cleared after execution]
  
  
} VPiRegs;

/**
 * @brief VPI Commands
 *  NOP:  Do nothing.
 *  ACT:  Activate configuration: Will configure times, wdg, pwm frequency, rev_divisor.
 *  BOOT: Special action to notify the board that the Vpid has been started up correctly and the system is booted.
 *  FEED: Feed watchdog
 *  SHUT: Shutdown power
 *  HARD: Hard shutdown power
 *  CLEAR: Clear clicks & RPM flag
 *  FAN:   Update fan.
 *  LED:   Update Led.
 *  BEEP:  Update beep.
 *  CLEARE: Clears error and reset error count.
 *  OUTS:   Change digiatal outputs
 */
#define VPI_CMD_NOP     (0x00)
#define VPI_CMD_ACT     ('A')
#define VPI_CMD_BOOT    ('B')
#define VPI_CMD_FEED    ('F')
#define VPI_CMD_HARD    ('H')
#define VPI_CMD_SHUT    ('S')
#define VPI_CMD_CLEAR   ('C')
#define VPI_CMD_FAN     ('N')
#define VPI_CMD_LED     ('L')
#define VPI_CMD_BEEP    ('Z')
#define VPI_CMD_CLEARE  ('X')
#define VPI_CMD_OUTS    ('O')

// BUTTONS
#define BUT_PWR         (0)
#define BUT_AUX         (1)
#define BUT_SHORT       (0)
#define BUT_LONG        (1)

// BOUNDARIES
#define VPI_LAST_REG	    (offsetof(VPiRegs,cmd))
#define VPI_FIRST_WREG    (offsetof(VPiRegs,pwm_freq))

// FLAGS
#define VPI_HAS_CLICK(r)   ( (r).status & 1 )
#define VPI_HAS_RPM(r)     ( (r).status & 2 )
#define VPI_HAS_ERROR(r)   ( (r).status & 4 )
#define VPI_CLEARALL(r)    ( (r).status = 0; )
#define VPI_CLEAR(r)       ( (r).status &= ~0x03   )
#define VPI_CLEARE(r)      ( (r).status &= ~0x04   )



// Default I2C Address ChangeMe to chose other address
#define VPI_I2C_ADDR      (0x33)
#define VPI_DEVICE_MAGIK  (0xAA)

#endif
