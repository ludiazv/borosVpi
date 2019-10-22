//#[cfg(test)]
//mod tests {
//    #[test]
//    fn it_works() {
//        assert_eq!(2 + 2, 4);
//    }
//}

// VPI module for low level control of the vpi firmware
//extern crate i2cdev;
#[macro_use]
extern crate memoffset;

use i2cdev::core::I2CDevice;
use i2cdev::linux::*;
use std::path::PathBuf;
use std::io::ErrorKind;
use std::mem;
use std::{thread, time};


pub mod uploader;


#[repr(C,packed)]
#[derive(Debug,Default,Copy,Clone)]
pub struct VpiTimes {
  short_tm      :u16,       ///< [RW] Short click time ms (max)
  space_tm      :u16,       ///< [RW] Spacing time ms
  hold_tm       :u8,        ///< [RW] Hold Time s //< [RW] Shutdown grace time s
  grace_tm      :u8,        
} 

impl VpiTimes {
    pub fn new(short:u16,space:u16,hold:u8,grace:u8) -> VpiTimes {
        VpiTimes { short_tm: short,
                   space_tm: space,
                   hold_tm: hold,
                   grace_tm : grace
        }
    }
    
    pub fn from_string(args:String) -> VpiTimes {
        let p:Vec<i32>=args.split_whitespace().map( |e| e.parse::<i32>().unwrap_or(-1)).collect();
        let mut r=VpiTimes::default();

        r
    }

}

#[repr(C,packed)]
#[derive(Debug,Default,Copy,Clone)]
struct VpiRegs {
  // Read only section
  id            :u8,            ///< [RO] ID of the chip usefull to check if present
  v             :u8,            ///< [RO] Version sequential
  status        :u8, 	        ///< [RO] Status register    7 6 5 4 3 2 1 0
                                ///<                         x x x x x E R C
                                ///<                         E: Error present.
                                ///<                         R: RPM updated.
                                ///<                         C: Clicks pending.
  buts          :[[u8;2];2],	///< [RO] Button clicks
  rpm           :u16,           ///< [RO] Fan estimated RPM
  err_count     :u16,           ///< [RO] Error count flags (for debuggin)
  uuid          :[u8;12],       ///< [RO] 96bit Unique ID.

  // RW section (Configuration and commands)
  pwm_freq      :u16,       ///< [RW] desired pwm frequency for fan and led 250-625000Hz
  rev_divisor   :u8,        ///< [RW] Number of pulses per revolution divisor.
  wdg           :u8,        ///< [RW] Time in seconds for highlevel watchdog 0=deactivated.
  times         :VpiTimes,  ///< [RW] times reg sections
  led_mode      :u8,        ///< [RW] Led mode <normal, on , off, blink, cycle>
  led_val       :u8,        ///< [RW] Lad value
  buzz_freq     :u8,        ///< [RW] Buzzer frequency <off, 2khz, 1Khz, 500Hz >
  buzz_b_tm     :u8,        ///< [RW] Buzzer beep time. centi seconds.
  buzz_p_tm     :u8,        ///< [RW] Buzzer pause time. centi seconds.
  buzz_count    :u8,        ///< [RW] Buzzer number of beeps
  outs          :u8,        ///< [RW] Ouputs
  fan_val       :u8,        ///< [RW] Fan value 0-255  
  cmd           :u8         
  
}

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
const VPI_CMD_NOP    :u8 =  0x00;
const VPI_CMD_ACT    :u8 =  'A' as u8;
const VPI_CMD_BOOT   :u8 =  'B' as u8;
const VPI_CMD_FEED   :u8 =  'F' as u8;
const VPI_CMD_HARD   :u8 =  'H' as u8;
const VPI_CMD_SHUT   :u8 =  'S' as u8;
const VPI_CMD_CLEAR  :u8 =  'C' as u8;
const VPI_CMD_FAN    :u8 =  'N' as u8;
const VPI_CMD_LED    :u8 =  'L' as u8;
const VPI_CMD_BEEP   :u8 =  'Z' as u8;
const VPI_CMD_CLEARE :u8 =  'X' as u8;
const VPI_CMD_OUTS   :u8 =  'O' as u8;

/// BUTTONS
pub const BUT_PWR   : u8 = 0;
pub const BUT_AUX   : u8 = 1;
pub const BUT_SHORT : u8 = 0;
pub const BUT_LONG  : u8 = 1;

/// MAGIK ID
const VPI_DEVICE_MAGIK : u8  =0xAA;

// // BOUNDARIES
// #define VPI_LAST_REG	    (offsetof(VPiRegs,cmd))
// #define VPI_FIRST_WREG    (offsetof(VPiRegs,pwm_freq))

// // FLAGS
// #define VPI_HAS_CLICK(r)   ( (r).status & 1 )
// #define VPI_HAS_RPM(r)     ( (r).status & 2 )
// #define VPI_HAS_ERROR(r)   ( (r).status & 4 )
// #define VPI_CLEAR(r)       ( (r).status &= ~0x03   )
// #define VPI_CLEARE(r)      ( (r).status &= ~0x04   )



// Default I2C Address ChangeMe to chose other address
pub const VPI_I2C_ADDR: u16 = 0x33;

// The object
pub struct Vpi {
    address : u16,
    regs    : VpiRegs,
    sregs   : VpiRegs,
    dev     : Option<LinuxI2CDevice>,
    debug   : bool,
}



// TODO relocate
pub enum VpiCommands {
    PowerOff,
    Feed,
    Fan(u8),
    Beep(u8),

}

pub struct VpiEvent {
    pwr_short: u8,
    pwr_long:  u8,
    aux_short: u8,
}



impl Vpi {
    /// New object with default values
    /// dbg: show vervose in stdout
    pub fn new(addr:Option<u16> ,dbg:bool) -> Vpi {
        
        Vpi { 
            address: addr.unwrap_or(VPI_I2C_ADDR), 
            regs:   Default::default(),
            sregs:  Default::default(),
            dev:    None,
            debug:  dbg,
        }

    }
    /// Prive funcions
    /// Produce a Not found error
    #[inline]
    fn err(s: &str) -> LinuxI2CError {
        LinuxI2CError::Io(std::io::Error::new(ErrorKind::NotFound,s))
    }
    // manage Endianness of u16
    #[inline]
    fn swap_u16(n:u16) -> u16 {
        let tmp  = n >> 8;
        (n<< 8) | tmp 
    }
    // Syn sregs & regs correcting endianness
    #[cfg(target_endian = "little")]
    fn sync(&mut self,read: bool) {
        if read {
            self.regs = self.sregs;
            self.regs.rpm= Vpi::swap_u16(self.regs.rpm);
            self.regs.err_count= Vpi::swap_u16(self.regs.err_count);
            self.regs.pwm_freq = Vpi::swap_u16(self.regs.pwm_freq);
            self.regs.times.short_tm = Vpi::swap_u16(self.regs.times.short_tm);
            self.regs.times.space_tm = Vpi::swap_u16(self.regs.times.space_tm);
        } else {
            self.sregs = self.regs;
            self.sregs.rpm= Vpi::swap_u16(self.sregs.rpm);
            self.sregs.err_count= Vpi::swap_u16(self.sregs.err_count);
            self.sregs.pwm_freq = Vpi::swap_u16(self.sregs.pwm_freq);
            self.sregs.times.short_tm = Vpi::swap_u16(self.sregs.times.short_tm);
            self.sregs.times.space_tm = Vpi::swap_u16(self.sregs.times.space_tm);
        }
    }
    #[cfg(target_endian = "big")]
    fn sync(&mut self,read: bool) {
        if read {
            self.regs = self.sregs;
        } else {
            self.sregs = self.regs;
        }
    }

    /// Open device
    pub fn open(&mut self,dev_path: &PathBuf) -> Result<u8,LinuxI2CError> {
        self.dev = Some(LinuxI2CDevice::new(dev_path,VPI_I2C_ADDR)?);
        let id= self.read_id()?;
        if id != VPI_DEVICE_MAGIK {
            self.dev = None;
            Err(Vpi::err("I2C device ID not match")) 
        } else {
            self.read_all()?; 
            Ok(id)
        }
    }
    /// Read registers starting in reg with len
    fn read(&mut self,reg : u8, len: u8) -> Result<(),LinuxI2CError>{
        if let Some(ref mut dev) = self.dev {
            // Check boundaries at is unsafe
            let r= reg as usize;
            let re= r+(len as usize);
            if re > mem::size_of::<VpiRegs>() {
                   return Err(LinuxI2CError::Io(std::io::Error::new(ErrorKind::InvalidInput,"I2C regs out of bounds")));
            }
            let addr : [u8;1] = [reg]; // First write the register
            dev.write(&addr)?;         // 
            // UNSAFE conversion 
            // 1st get a mutable reference from sregs
            // 2nd cast as as mutable raw pointer
            // 3rd re cast the raw point as potiner to bytes.
            let ptr = (&mut(self.sregs) as *mut VpiRegs) as *mut u8;
            let as_buff = unsafe { std::slice::from_raw_parts_mut(ptr.add(r), len as usize) };
            dev.read(as_buff)?;
            self.sync(true); // sync shadow registers and fronte
            if self.debug {
                println!("I2C-RD reg:{} len:{}",reg,len);
            }
            Ok(())
        } else {
            Err(Vpi::err("I2C not available/opened")) 
        }
    
    }
    /// write registers at reg with len u:
    fn write(&mut self,reg : u8, len:u8) -> Result<(),LinuxI2CError> {
        self.sync(false); // sync front registers to shawdow
        if let Some(ref mut dev) = self.dev {
            // Check boundaries as it is unsafe operation
            let r= reg as usize;
            let re= r+(len as usize);
            if r < offset_of!(VpiRegs,pwm_freq) || re > (offset_of!(VpiRegs,cmd)+1) {
                   return Err(LinuxI2CError::Io(std::io::Error::new(ErrorKind::InvalidInput,"WR - I2C regs out of bounds")));
            }
            let mut total : Vec<u8> = vec![reg];// First write the register
            // UNSAFE conversion 
            // 1st get a const reference from sregs
            // 2nd cast as as const raw pointer
            // 3rd re cast the raw point as potiner to bytes.
            let ptr = (&(self.sregs) as *const VpiRegs) as *const u8;
            let as_buff = unsafe { std::slice::from_raw_parts(ptr.add(r), len as usize) };
            total.extend_from_slice(as_buff);
            dev.write(total.as_slice())?;
            if self.debug {
                println!("I2C-WR reg:{} len:{} 0:{}",reg,as_buff.len(),as_buff[0] as char);
            }
            Ok(())
        } else {
            Err(Vpi::err("I2C not available/opened")) 
        }
    }
    pub fn cmd(&mut self) -> Result<(),LinuxI2CError> {
        //self.sregs.cmd=self.regs.cmd;
        self.write(offset_of!(VpiRegs,cmd) as u8,1)?;
        thread::sleep(time::Duration::from_millis(2));
        if self.regs.cmd == VPI_CMD_ACT {
            thread::sleep(time::Duration::from_millis(8));
        }
        self.regs.cmd=VPI_CMD_NOP;
        self.sregs.cmd=VPI_CMD_NOP;
        Ok(())
    }

    // Read Board ID (MAGIK and version)
    pub fn read_id(&mut self) -> Result<u8,LinuxI2CError> {
        self.read(0,2)?;
        Ok(self.sregs.id)
    }

    /// Read all registers from vpi
    pub fn read_all(&mut self) -> Result<(),LinuxI2CError> {
        self.read(0,mem::size_of::<VpiRegs>() as u8)?;
        if self.debug {
            println!("Device addr:{}",self.address);
            println!("Readed all:\n {:#?}",self.regs);
        }
        Ok(())
    }
    /// Sends all configuration registers to Vpi
    pub fn config(&mut self) -> Result<(),LinuxI2CError>  {
        self.regs.cmd=VPI_CMD_ACT;
        let first = offset_of!(VpiRegs,pwm_freq) as u8;
        let len = offset_of!(VpiRegs,cmd) as u8 - first; 
        self.write(first,len)?;
        Ok(())
    }

    // Configure options
    /// Set pwm_freq
    pub fn pwm_freq(&mut self,pwmf:u16) -> &mut Self {
        self.regs.pwm_freq=pwmf;
        self
    }
     /// Set revolution divisor
    pub fn rev_divisor(&mut self,d:u8) -> &mut Self {
        self.regs.rev_divisor=d;
        self
    }
    pub fn wdg(&mut self,wdg:u8) -> &mut Self {
        self.regs.wdg=wdg;
        self
    }
    /// Set timings
    pub fn timings(&mut self,short_tm:u16, space_tm:u16, hold_tm:u8) -> &mut Self {
        self.regs.times.short_tm=short_tm;
        self.regs.times.space_tm=space_tm;
        self.regs.times.hold_tm=hold_tm;
        self
    }
    /// Set gace time
    pub fn grace(&mut self,grace_tm:u8) -> &mut Self {
        self.regs.times.grace_tm=grace_tm;
        self
    }
    /// Set gace time
    pub fn led(&mut self,m:u8,v:u8) -> &mut Self {
        self.regs.led_mode=m;
        self.regs.led_val=v;
        self
    }
     /// Set fan  speed
    pub fn fan(&mut self,speed:u8) -> &mut Self {
        self.regs.cmd=VPI_CMD_FAN;
        self.regs.fan_val= speed;
        self
    }
    /// Set the commando to feed wdg
    pub fn feed(&mut self) -> &mut Self {
        self.regs.cmd=VPI_CMD_FEED;
        self
    }

    pub fn boot(&mut self) -> &mut Self {
        self.regs.cmd=VPI_CMD_BOOT;
        self
    }
   

    /// There are some imnediate action

    /// Set fan speed and send to device
    pub fn fan_now(&mut self,speed:u8) -> Result<(),LinuxI2CError> {
        self.fan(speed);
        let first = offset_of!(VpiRegs,fan_val) as u8;
        let len = offset_of!(VpiRegs,cmd) as u8 - first+1;
        self.write(first,len)?;
        thread::sleep(time::Duration::from_millis(2));
        self.regs.cmd=0;
        self.sregs.cmd=0;
        Ok(())
    } 
    


    // Geting information


}





