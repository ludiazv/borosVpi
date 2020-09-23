//! VPI module for low level access via I2C
//! Enable full control of the VPi harware using the I2C protocol of the board.
//! The module mimics the commands and behaviour expected by the firmware.
//! but introducing realibility meachanims such retrys, pauses, etc....
//!
//! 

// ----- External references ---
extern crate i2cdev;
#[macro_use]
extern crate memoffset;

//Core imports
use i2cdev::core::I2CDevice;
use i2cdev::linux::*;
use serde::Serialize;
use std::io::ErrorKind;
use std::mem;
use std::path::PathBuf;
use std::{thread, time};

// Local imports
use cmd::{VpiCmd, VpiCmdOutput};

// Define
pub mod cmd;
pub mod uploader;


// --- CONSTANTS ----
/// Minimun wait time between i2c transfers
const MIN_I2C_XFER_TIME: time::Duration = time::Duration::from_millis(3);
/// Maximun retry on I2C failure
const I2C_RETRIES: u32 = 3;

/// Simple C structure for packed times
#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct VpiTimes {
    /// [RW] Short click time ms (max)
    pub short_tm: u16,
    /// [RW] Spacing time ms       
    pub space_tm: u16,
    /// [RW] Hold Time s
    pub hold_tm: u8,
    /// [RW] Shutdown grace time s
    pub grace_tm: u8,
}
/// Iplements some default values.
impl Default for VpiTimes {
    fn default() -> Self {
        VpiTimes {
            short_tm: 200u16,
            space_tm: 1200u16,
            hold_tm: 8u8,
            grace_tm: 15u8,
        }
    }
}
impl VpiTimes {
    pub fn new(short: u16, space: u16, hold: u8, grace: u8) -> VpiTimes {
        let s = format!("{} {} {} {}", short, space, hold, grace);
        VpiTimes::from_string(s)
    }
    // constructor VpiTimes from string <short> <space> <hold> <grace>
    pub fn from_string(args: String) -> VpiTimes {
        let p: Vec<i32> = args
            .split_whitespace()
            .map(|e| e.parse::<i32>().unwrap_or(-1))
            .collect();
        let mut r = VpiTimes::default();
        if p.len() >= 1 && p[0] > 20 && p[0] <= u16::max as i32 {
            r.short_tm = p[0] as u16
        }
        if p.len() >= 2 && p[1] > 100 && p[1] <= u16::max as i32 {
            r.space_tm = p[1] as u16
        }
        if p.len() >= 3 && p[2] > 0 && p[2] <= u8::max as i32 {
            r.hold_tm = p[2] as u8
        }
        if p.len() >= 4 && p[3] > 0 && p[3] <= u8::max as i32 {
            r.grace_tm = p[3] as u8
        }
        r
    }
}

/// Simple C structure for packed times
#[repr(C, packed)]
#[derive(Debug, Default, Copy, Clone)]
pub struct VpiLed {
    ///< [RW] Led mode on , off, blink, cycle,....
    pub led_mode: u8,
    ///< [RW] Lad value
    pub led_val: u8,
}
impl VpiLed {
    pub fn new(mode: u8, val: u8) -> VpiLed {
        VpiLed {
            led_mode: mode,
            led_val: val,
        }
    }
}

#[repr(C, packed)]
#[derive(Debug, Default, Copy, Clone)]
pub struct VpiBuzz {
    ///< [RW] Buzzer frequency <off, 2khz, 1Khz, 500Hz >
    pub buzz_freq: u8,
    ///< [RW] Buzzer beep time. centi seconds.
    pub buzz_b_tm: u8,
    ///< [RW] Buzzer pause time. centi seconds.
    pub buzz_p_tm: u8,
    ///< [RW] Buzzer number of beeps
    pub buzz_count: u8,
}
impl VpiBuzz {
    fn new(f: u8, c: u8, b_t: u8, p_t: u8) -> VpiBuzz {
        VpiBuzz {
            buzz_freq: f,
            buzz_count: c,
            buzz_b_tm: b_t,
            buzz_p_tm: p_t,
        }
    }
}

/// Status in more usable form
#[derive(Debug, Default, Copy, Clone, Serialize)]
pub struct VpiStatus {
    pub has_click: bool,
    pub has_rpm: bool,
    pub has_error: bool,
    pub has_irq: bool,
    pub is_running: bool,
    pub is_wdg_enabled: bool,
    pub is_wake_enabled: bool,
    pub is_wake_irq_enabled: bool,
    pub out_value: bool,
    pub integrity: bool,
    pub pwr_short: i32,
    pub pwr_long: i32,
    pub aux_short: i32,
    pub aux_long: i32,
    pub rpm: i32,
    pub error_count: i32,
    pub recover_type: u8,
    pub crc: u8,
}

impl VpiStatus {
    pub fn has_changed(&self, &prev: &VpiStatus) -> bool {
        self.has_click != prev.has_click
            || self.has_error != prev.has_error
            || self.has_irq != prev.has_irq
            || self.is_running != prev.is_running
            || self.is_wdg_enabled != prev.is_wdg_enabled
            || self.is_wake_enabled != prev.is_wake_enabled
            || self.is_wake_irq_enabled != prev.is_wake_irq_enabled
            || self.out_value != prev.out_value
    }
}

#[repr(C, packed)]
#[derive(Debug, Default, Copy, Clone)]
struct VpiRegs {
    // Read only section
    id: u8,
    ///< [RO] ID of the chip usefull to check if present
    v: u8,
    ///< [RO] Version sequential
    status: u8,
    ///< [RO] Status register    7 6 5 4 3 2 1 0
    ///<                         1 0 I W B E R C
    ///<                         E: Error present.
    ///<                         R: RPM updated.
    ///<                         C: Clicks pending.
    ///<                         B: Running
    ///<                         W: Watchdog enabled
    ///<                         I: Interrupt pending
    flags: u8,
    ///< [RO] additional status flags 7 6 5 4 3 2 1 0
    ///<                              1 0 X X X O W I
    ///<                              I: Wake by IRQ enabled
    ///<                              W: Wake enabled
    ///<                              O: Ouput state
    crc: u8,
    ///< [RO] CRC8 of cofiguration registers
    buts: [[u8; 2]; 2],
    ///< [RO] Button clicks
    rpm: u16,
    ///< [RO] Fan estimated RPM
    err_count: u8,
    ///< [RO] Error count flags (for debuggin)
    uuid: [u8; 12],
    ///< [RO] 96bit Unique ID.
    // RW section (Configuration and commands)
    pwm_freq: u16,
    ///< [RW] desired pwm frequency for fan and led 250-62500Hz
    rev_divisor: u8,
    ///< [RW] Number of pulses per revolution divisor.
    wdg: u8,
    ///< [RW] Time in seconds for highlevel watchdog 0=deactivated.
    wake: u16,
    ///< [RW] Autowake in minutes 0=no autowake
    times: VpiTimes,
    ///< [RW] times reg sections
    led: VpiLed,
    ///< [RW] led regs section
    buzz: VpiBuzz,
    ///< [RW] buzz regs section
    fan_val: u8,
    ///< [RW] Fan value 0-255
    cmd: u8,
    icmd: u8,
}

// VPI commands constants
const VPI_CMD_NOP: u8 = 0x00;
const VPI_CMD_ACT: u8 = 'A' as u8;
const VPI_CMD_BOOT: u8 = 'B' as u8;
const VPI_CMD_INIT: u8 = 'I' as u8;
const VPI_CMD_FEED: u8 = 'F' as u8;
const VPI_CMD_HARD: u8 = 'H' as u8;
const VPI_CMD_SHUT: u8 = 'S' as u8;
const VPI_CMD_CLEAR: u8 = 'C' as u8;
const VPI_CMD_FAN: u8 = 'N' as u8;
const VPI_CMD_LED: u8 = 'L' as u8;
const VPI_CMD_BEEP: u8 = 'Z' as u8;
const VPI_CMD_OUTSET: u8 = '1' as u8;
const VPI_CMD_OUTCL: u8 = '0' as u8;
const VPI_CMD_RESET: u8 = 'T' as u8;
const VPI_CMD_WDGSET: u8 = 'W' as u8;
const VPI_CMD_WDGRST: u8 = 'V' as u8;
const VPI_CMD_WEN: u8 = 'E' as u8;
const VPI_CMD_WDI: u8 = 'D' as u8;
const VPI_CMD_IEN: u8 = 'e' as u8;
const VPI_CMD_IDI: u8 = 'd' as u8;

// Button index
const BUT_PWR: usize = 0;
const BUT_AUX: usize = 1;
const BUT_SHORT: usize = 0;
const BUT_LONG: usize = 1;

/// MAGIK ID
const VPI_DEVICE_MAGIK: u16 = 0xAA;

// // BOUNDARIES
// #define VPI_LAST_REG	    (offsetof(VPiRegs,cmd))
// #define VPI_FIRST_WREG    (offsetof(VPiRegs,pwm_freq))

/// FLAGS
const VPI_HAS_CLICK: u8 = 1;
const VPI_HAS_RPM: u8 = 2;
const VPI_HAS_ERROR: u8 = 4;
const VPI_IS_RUNNING: u8 = 8;
const VPI_HAS_WDG: u8 = 16;
const VPI_HAS_IRQ: u8 = 32;
const VPI_HAS_WAKEENI: u8 = 1;
const VPI_HAS_WAKEEN: u8 = 2;
const VPI_HAS_OUT_FLA: u8 = 4;

/// Default I2C Address ChangeMe to chose other address
pub const VPI_I2C_ADDR: u16 = 0x33;

/// Stats for Vpi
#[derive(Debug, Copy, Clone, Serialize)]
pub struct VpiStats {
    pub retries: u32,
    pub recovers: u32,
    pub i2c_errors: u32,
    pub status_checks: u64,
    pub crc_errors: u32,
    #[serde(skip)]
    last_read: time::Instant,
    #[serde(skip)]
    last_write: time::Instant,
}

/// Vpi object
pub struct Vpi {
    address: u16,
    /// Address    
    regs: VpiRegs,
    /// registers
    sregs: VpiRegs,
    /// shawdow registers (in endianness of the device)
    dev: Option<LinuxI2CDevice>, // Device
    debug: bool,
    stats: VpiStats,
}

/// Error type for the module
pub type Error = LinuxI2CError;
/// Result type for the module
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// convert hex string byte to u8 (format DD or 0xDD)
pub fn from_str_address(s: &str) -> std::result::Result<u8, std::num::ParseIntError> {
    if s.len() != 2 && s.len() != 4 {
        u8::from_str_radix("uu", 16)
    } else {
        if s.len() == 2 {
            u8::from_str_radix(s, 16)
        } else {
            u8::from_str_radix(&s[2..], 16)
        }
    }
}

impl Vpi {
    /// New object with default values. The commication with the device is not started here
    /// dbg: show vervose in stdout
    pub fn new(addr: Option<u16>, dbg: bool) -> Self {
        Vpi {
            address: addr.unwrap_or(VPI_I2C_ADDR),
            regs: Default::default(),
            sregs: Default::default(),
            dev: None,
            debug: dbg,
            stats: VpiStats {
                retries: 0,
                recovers: 0,
                i2c_errors: 0,
                crc_errors: 0,
                status_checks: 0,
                last_read: time::Instant::now(),
                last_write: time::Instant::now(),
            },
        }
    }
    /// Get I2C i2c address
    pub fn get_addr(&self) -> u16 {
        self.address
    }
    /// Get the current fan value
    pub fn get_fan_value(&self) -> u8 {
        self.regs.fan_val
    }
    /// Produce a Not found error for an arbitrary string
    #[inline]
    fn err(s: &str) -> Error {
        LinuxI2CError::Io(std::io::Error::new(ErrorKind::NotFound, s))
    }
    /// Helper for sleep ms
    #[inline]
    fn sleep_ms(n: u64) {
        thread::sleep(time::Duration::from_millis(n));
    }
    /// helper for Manage Endianness of u16 
    #[inline]
    fn swap_u16(n: u16) -> u16 {
        let tmp = n >> 8;
        (n << 8) | tmp
    }
    /// Check the integrity of the status register in line with firmware rules
    #[inline]
    fn status_integrity(s: u8) -> bool {
        (s & 0x80) == 0x80u8 && (s & 0b01000000) == 0
    }
    /// Get instant of the last i2c transfer
    #[inline]
    pub fn last_xfer(&self) -> time::Instant {
        self.stats.last_read.max(self.stats.last_write)
    }
    /// Get current stats
    #[inline]
    pub fn get_stats(&self) -> VpiStats {
        self.stats
    }
    /// Syn sregs & regs correcting endianness
    /// This function will change endianness of u16 registers.
    /// # Arguments
    /// * `read` - true -> copy shawdow regs to regs / false -> copy regs to shadow regs
    #[cfg(target_endian = "little")]
    fn sync(&mut self, read: bool) {
        if read {
            self.regs = self.sregs;
            self.regs.rpm = Vpi::swap_u16(self.regs.rpm);
            self.regs.wake = Vpi::swap_u16(self.regs.wake);
            //self.regs.err_count= Vpi::swap_u16(self.regs.err_count);
            self.regs.pwm_freq = Vpi::swap_u16(self.regs.pwm_freq);
            self.regs.times.short_tm = Vpi::swap_u16(self.regs.times.short_tm);
            self.regs.times.space_tm = Vpi::swap_u16(self.regs.times.space_tm);
        } else {
            self.regs.icmd = self.regs.cmd ^ VPI_DEVICE_MAGIK as u8;
            self.sregs = self.regs;
            self.sregs.rpm = Vpi::swap_u16(self.sregs.rpm);
            self.sregs.wake = Vpi::swap_u16(self.sregs.wake);
            //self.sregs.err_count= Vpi::swap_u16(self.sregs.err_count);
            self.sregs.pwm_freq = Vpi::swap_u16(self.sregs.pwm_freq);
            self.sregs.times.short_tm = Vpi::swap_u16(self.sregs.times.short_tm);
            self.sregs.times.space_tm = Vpi::swap_u16(self.sregs.times.space_tm);
        }
    }
    #[cfg(target_endian = "big")]
    fn sync(&mut self, read: bool) {
        if read {
            self.regs = self.sregs;
        } else {
            self.regs.icmd = self.regs.cmd ^ VPI_DEVICE_MAGIK as u8;
            self.sregs = self.regs;
        }
    }

    fn config_crc(&mut self) -> u8 {
        let first = offset_of!(VpiRegs, pwm_freq);
        let len = offset_of!(VpiRegs, cmd) - first;
        let ptr = (&mut (self.sregs) as *mut VpiRegs) as *mut u8;
        let as_buff = unsafe { std::slice::from_raw_parts_mut(ptr.add(first), len) };
        uploader::buff_crc(as_buff, 0)
    }

    /// Open the device
    /// # Arguments
    ///
    /// * `dev_path` - full path to /dev/i2c-xx device
    ///
    /// # Return
    ///   `device_id` or Err
    pub fn open(&mut self, dev_path: &PathBuf) -> Result<u8> {
        self.dev = Some(LinuxI2CDevice::new(dev_path, VPI_I2C_ADDR)?);
        let id = self.read_id();
        if id > 255 {
            self.dev = None;
            return Err(Vpi::err("Contact with Vpi board"));
        }
        if id != VPI_DEVICE_MAGIK {
            self.dev = None;
            Err(Vpi::err("I2C device ID not match"))
        } else {
            self.read_all()?;
            Ok(self.regs.id)
        }
    }

    /// Retry function helper function. Retries the function until success or
    /// I2C_RETRIES is reached.
    /// # Arguments
    ///   * f: function or clouse to execute.
    ///   * debug: print a debug trace in stdout
    /// # Retun
    ///    number of retries pending or Err
    fn retry<F>(mut f: F, debug: bool) -> Result<u32>
    where
        F: FnMut() -> Result<()>,
    {
        let mut retries: u32 = I2C_RETRIES;
        let mut res: Result<()> = Ok(());
        while retries > 0 {
            res = f();
            if res.is_ok() {
                return Ok(I2C_RETRIES - retries);
            }
            if debug {
                println!("Failed {:?} try {}", res, retries);
            }
            Vpi::sleep_ms(100 + 100 * (I2C_RETRIES - retries) as u64);
            retries -= 1;
        }
        Err(res.err().unwrap())
    }

    /// Perform a simple I2C read transaction in to buffer `buff`
    fn buff_read(&mut self, reg: u8, buff: &mut [u8]) -> Result<()> {
        let dev = self.dev.as_mut().unwrap();
        let addr: [u8; 1] = [reg]; // First write the register
        dev.write(&addr)?;
        Vpi::sleep_ms(1); // give some time to set the register in the device safetly
        dev.read(buff)?;
        if self.debug {
            let u: Vec<String> = buff.into_iter().map(|b| format!("{:02X}", b)).collect();
            let c = u.join(" ");
            println!("I2C-RD reg:0x{:02X} len:{}, values:{}", reg, buff.len(), c);
        }
        self.stats.last_read = time::Instant::now();
        Ok(())
    }

    fn atomic_read(&mut self, reg: u8, len: u8) -> Result<()> {
        // Check boundaries at is unsafe
        let r = reg as usize;
        let re = r + (len as usize);
        if re > mem::size_of::<VpiRegs>() {
            return Err(LinuxI2CError::Io(std::io::Error::new(
                ErrorKind::InvalidInput,
                "I2C regs out of bounds",
            )));
        }
        // UNSAFE conversion
        // 1st get a mutable reference from sregs
        // 2nd cast as as mutable raw pointer
        // 3rd re cast the raw point as potiner to bytes.
        let ptr = (&mut (self.sregs) as *mut VpiRegs) as *mut u8;
        let as_buff = unsafe { std::slice::from_raw_parts_mut(ptr.add(r), len as usize) };
        thread::sleep(time::Duration::from_micros(500)); // Give some time to read.
        self.buff_read(reg, as_buff)?; // Read in to as_buff
        self.sync(true); // sync shadow registers and fronte
        Ok(())
    }
    /// Read registers starting in reg with len with retries.
    fn read(&mut self, reg: u8, len: u8) -> Result<()> {
        if self.dev.is_none() {
            Err(Vpi::err("I2C not available/opened"))
        } else {
            let d = self.debug;
            if self.last_xfer().elapsed() < MIN_I2C_XFER_TIME {
                thread::sleep(MIN_I2C_XFER_TIME);
            }
            let res = Vpi::retry(|| self.atomic_read(reg, len), d);
            if let Ok(retries) = res {
                self.stats.retries += retries;
                Ok(())
            } else {
                Err(res.err().unwrap())
            }
        }
    }
    /// Write registers starting in reg with len
    fn write(&mut self, reg: u8, len: u8) -> Result<()> {
        if self.dev.is_none() {
            Err(Vpi::err("I2C not available/opened"))
        } else {
            let d = self.debug;
            if self.last_xfer().elapsed() < MIN_I2C_XFER_TIME {
                thread::sleep(MIN_I2C_XFER_TIME);
            }
            let res = Vpi::retry(|| self.atomic_write(reg, len), d);
            if let Ok(retries) = res {
                self.stats.retries += retries;
                Ok(())
            } else {
                Err(res.err().unwrap())
            }
        }
    }

    /// write registers at reg with len
    fn atomic_write(&mut self, reg: u8, len: u8) -> Result<()> {
        self.sync(false); // sync front registers to shawdow
        let dev = self.dev.as_mut().unwrap();
        // Check boundaries as it is unsafe operation
        let r = reg as usize;
        let re = r + (len as usize);
        if r < offset_of!(VpiRegs, pwm_freq) || re > (offset_of!(VpiRegs, icmd) + 1) {
            return Err(LinuxI2CError::Io(std::io::Error::new(
                ErrorKind::InvalidInput,
                "WR - I2C regs out of bounds",
            )));
        }
        let mut total: Vec<u8> = vec![reg]; // First write the register
                                            // UNSAFE conversion
                                            // 1st get a const reference from sregs
                                            // 2nd cast as as const raw pointer
                                            // 3rd re cast the raw point as potiner to bytes.
        let ptr = (&(self.sregs) as *const VpiRegs) as *const u8;
        let as_buff = unsafe { std::slice::from_raw_parts(ptr.add(r), len as usize) };
        total.extend_from_slice(as_buff);
        dev.write(total.as_slice())?;
        if self.debug {
            let u: Vec<String> = as_buff.into_iter().map(|b| format!("{:02X}", b)).collect();
            let c = u.join(" ");
            println!(
                "I2C-WR reg:0x{:02X} len:{} values:{}",
                reg,
                as_buff.len(),
                c
            );
        }
        self.stats.last_write = time::Instant::now();
        Ok(())
    }
    /// Send the the defined command to the device
    /// Implement minimal delays to give time to the hw to execute the command.
    pub fn cmd(&mut self) -> Result<()> {
        self.write(offset_of!(VpiRegs, cmd) as u8, 2)?;
        Vpi::sleep_ms(25);
        self.regs.cmd = VPI_CMD_NOP;
        self.sregs.cmd = VPI_CMD_NOP;
        Ok(())
    }

    // Read Board ID (MAGIK and version)
    pub fn read_id(&mut self) -> u16 {
        match self.read(0, 2) {
            Ok(_) => self.sregs.id as u16,
            Err(_) => 256u16,
        }
    }
    /// Read all registers from vpi board
    pub fn read_all(&mut self) -> Result<()> {
        self.read(0, mem::size_of::<VpiRegs>() as u8)?;
        if self.debug {
            self.dump_regs()
        }
        Ok(())
    }
    // Print registers
    pub fn dump_regs(&self) {
        println!("Device addr:0x{:02X}", self.address);
        println!("Registers:\n {:#?}", self.regs);
    }
    /// Get UUID
    pub fn get_uuid(&self) -> String {
        let u: Vec<String> = self
            .regs
            .uuid
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect();
        u.join("")
    }
    /// Sends all configuration registers to Vpi and sends actulization command.
    pub fn config(&mut self) -> Result<()> {
        self.regs.cmd = VPI_CMD_ACT;
        let first = offset_of!(VpiRegs, pwm_freq) as u8;
        let len = offset_of!(VpiRegs, icmd) as u8 - first + 1;
        self.write(first, len)?;
        self.regs.cmd = VPI_CMD_NOP;
        self.sregs.cmd = VPI_CMD_NOP;
        Vpi::sleep_ms(20);
        self.wdg_enable(self.regs.wdg > 0).cmd()?;
        self.wake_enable(self.regs.wake > 0 || (self.regs.flags & VPI_HAS_WAKEENI != 0)).cmd()?;
        Ok(())
    }
    pub fn poll_time(&self) -> std::time::Duration {
        let mut p_time: u64 = 500;
        if self.dev.is_some() {
            p_time = (self.regs.times.space_tm as u64) / 2u64;
        }
        time::Duration::from_millis(p_time)
    }

    // Configure options
    /// Set pwm frequency. To activate require a call to `config()`
    pub fn pwm_freq(&mut self, pwmf: u16) -> &mut Self {
        let mut pwm = pwmf;
        if pwm < 2 {
            pwm = 2;
        }
        if pwm > 62500 {
            pwm = 62500;
        }
        self.regs.pwm_freq = pwm;
        self
    }
    /// Set revolution divisor. To activate require a call to `config()`
    pub fn rev_divisor(&mut self, di: u8) -> &mut Self {
        let mut d = di;
        if d == 0 {
            d = 2;
        }
        self.regs.rev_divisor = d;
        self
    }
    /// Set watchdog time. To activate require a call to `config()`
    pub fn wdg(&mut self, wdg: u8) -> &mut Self {
        self.regs.wdg = wdg;
        self
    }
    /// Set wake time. To activate require a call to `config()`
    pub fn wake(&mut self, w: u16) -> &mut Self {
        self.regs.wake = w;
        self
    }
    /// Set timing configuration. To activate require a call to `config()`
    pub fn timings(&mut self, t: &VpiTimes) -> &mut Self {
        /*self.regs.times.short_tm=t.short_tm;
        self.regs.times.space_tm=t.space_tm;
        self.regs.times.hold_tm=t.hold_tm;
        self.regs.times.grace_tm=t.grace_tm;*/
        self.regs.times = *t;
        self
    }
    /// Set led value & mode
    pub fn led(&mut self, l: VpiLed) -> &mut Self {
        self.regs.cmd = VPI_CMD_LED;
        self.regs.led = l;
        self
    }
    /// Set fan  speed
    pub fn fan(&mut self, speed: u8) -> &mut Self {
        self.regs.cmd = VPI_CMD_FAN;
        self.regs.fan_val = speed;
        self
    }
    pub fn buzz(&mut self, bp: VpiBuzz) -> &mut Self {
        self.regs.buzz = bp;
        self.regs.cmd = VPI_CMD_BEEP;
        self
    }
    /// Set the commando to feed wdg
    pub fn feed(&mut self) -> &mut Self {
        self.regs.cmd = VPI_CMD_FEED;
        self
    }
    /// Set the comands to boot indication
    pub fn boot(&mut self) -> &mut Self {
        self.regs.cmd = VPI_CMD_BOOT;
        self
    }
    /// Set the borad in init(booting state)
    pub fn init(&mut self) -> &mut Self {
        self.regs.cmd = VPI_CMD_INIT;
        self
    }
    /// Set command to hard shudown
    pub fn hard_shutdown(&mut self) -> &mut Self {
        self.regs.cmd = VPI_CMD_HARD;
        self
    }
    /// Set command to shutdown
    pub fn shutdown(&mut self) -> &mut Self {
        self.regs.cmd = VPI_CMD_SHUT;
        self
    }
    /// Set command to clear
    pub fn clear(&mut self) -> &mut Self {
        self.regs.cmd = VPI_CMD_CLEAR;
        self
    }
    //pub fn clear_error(&mut self) -> &mut Self {
    //    self.regs.cmd=VPI_CMD_CLEARE;
    //    self
    //}
    /// Reset the VpiBoard
    pub fn reset(&mut self) -> &mut Self {
        self.regs.cmd = VPI_CMD_RESET;
        self
    }
    /// set wdg
    pub fn wdg_enable(&mut self, en: bool) -> &mut Self {
        if en {
            self.regs.cmd = VPI_CMD_WDGSET;
        } else {
            self.regs.cmd = VPI_CMD_WDGRST;
        }
        self
    }
    /// Start a beep sequencie configured
    pub fn beep(&mut self) -> &mut Self {
        self.regs.cmd = VPI_CMD_BEEP;
        self
    }
    /// Enable wake configured
    pub fn wake_enable(&mut self, en: bool) -> &mut Self {
        if en {
            self.regs.cmd = VPI_CMD_WEN;
        } else {
            self.regs.cmd = VPI_CMD_WDI;
        }
        self
    }
    /// Enable wake configured
    pub fn wake_irq(&mut self, en: bool) -> &mut Self {
        if en {
            self.regs.cmd = VPI_CMD_IEN;
        } else {
            self.regs.cmd = VPI_CMD_IDI;
        }
        self
    }
    /// Enable wake configured
    pub fn output(&mut self, en: bool) -> &mut Self {
        if en {
            self.regs.cmd = VPI_CMD_OUTSET;
        } else {
            self.regs.cmd = VPI_CMD_OUTCL;
        }
        self
    }

    // ---- Inmediate actions ----
    /// Set fan speed and send to device
    pub fn fan_now(&mut self, speed: u8) -> Result<()> {
        self.fan(speed);
        let first = offset_of!(VpiRegs, fan_val) as u8;
        let len = offset_of!(VpiRegs, icmd) as u8 - first + 1;
        self.write(first, len)?;
        Vpi::sleep_ms(5);
        self.regs.cmd = VPI_CMD_NOP;
        self.sregs.cmd = VPI_CMD_NOP;
        Ok(())
    }
    /// Buzz inmeditaly
    pub fn buzz_now(&mut self, bp: &VpiBuzz) -> Result<()> {
        self.buzz(*bp);
        let first = offset_of!(VpiRegs, buzz);
        self.write(first as u8, mem::size_of::<VpiBuzz>() as u8)?;
        Vpi::sleep_ms(5);
        self.cmd()
    }
    /// Configure and change led
    pub fn led_now(&mut self, l: VpiLed) -> Result<()> {
        self.led(l);
        let first = offset_of!(VpiRegs, led) as u8;
        self.write(first, 2)?;
        Vpi::sleep_ms(2);
        self.cmd()
    }

    // Geting information
    pub fn check_status(&mut self, recover_flag: u8) -> Result<VpiStatus> {
        self.stats.status_checks += 1;
        self.read(offset_of!(VpiRegs, status) as u8, 3)?;
        let mut s = VpiStatus::default();
        s.integrity =
            Vpi::status_integrity(self.regs.status) && Vpi::status_integrity(self.regs.flags);
        s.recover_type = recover_flag;
        if !s.integrity {
            Vpi::sleep_ms(10);
            self.read(offset_of!(VpiRegs, status) as u8, 3)?;
            s.integrity =
                Vpi::status_integrity(self.regs.status) && Vpi::status_integrity(self.regs.flags);
            if !s.integrity {
                return Ok(s);
            }
        }
        s.crc = self.regs.crc;
        let config_crc = self.config_crc();
        s.has_click = self.regs.status & VPI_HAS_CLICK != 0;
        s.has_rpm = self.regs.status & VPI_HAS_RPM != 0;
        s.has_error = self.regs.status & VPI_HAS_ERROR != 0;
        s.is_running = self.regs.status & VPI_IS_RUNNING != 0;
        s.is_wdg_enabled = self.regs.status & VPI_HAS_WDG != 0;
        s.has_irq = self.regs.status & VPI_HAS_IRQ != 0;
        s.is_wake_enabled = self.regs.flags & VPI_HAS_WAKEEN != 0;
        s.is_wake_irq_enabled = self.regs.flags & VPI_HAS_WAKEENI != 0;
        s.out_value = self.regs.flags & VPI_HAS_OUT_FLA != 0;

        if s.has_rpm || s.has_click || s.has_error {
            let mut first = offset_of!(VpiRegs, buts) as u8;
            let mut len = 2u8;
            if s.has_click {
                len += 2;
            }
            if s.has_rpm && !s.has_click {
                first = offset_of!(VpiRegs, rpm) as u8;
            }
            if s.has_rpm && s.has_click {
                len += 3;
            }
            if s.has_error {
                len += 2;
            }
            Vpi::sleep_ms(5);
            self.read(first, len)?;
            if s.has_click {
                s.pwr_short = self.regs.buts[BUT_PWR][BUT_SHORT] as i32;
                s.pwr_long = self.regs.buts[BUT_PWR][BUT_LONG] as i32;
                s.aux_short = self.regs.buts[BUT_AUX][BUT_SHORT] as i32;
                s.aux_long = self.regs.buts[BUT_AUX][BUT_LONG] as i32;
            }
            if s.has_rpm {
                s.rpm = self.regs.rpm as i32;
            }
            if s.has_error {
                s.error_count = self.regs.err_count as i32;
                self.stats.i2c_errors += s.error_count as u32;
            }
            Vpi::sleep_ms(5);
            self.clear().cmd()?;
            self.regs.buts = [[0, 0], [0, 0]];
            self.regs.status &= !(VPI_HAS_CLICK | VPI_HAS_RPM | VPI_HAS_IRQ);
        } else if s.has_irq {
            Vpi::sleep_ms(5);
            self.clear().cmd()?;
            self.regs.status &= !(VPI_HAS_IRQ);
        }
        if s.crc != config_crc {
            // Configuration CRC mistmatch
            if self.debug {
                println!(
                    "CRC config mistmatch [Local:0x{:02X},Board:0x{:02X}] Sync launched",
                    config_crc, s.crc
                );
            }
            self.stats.crc_errors += 1;
            self.config()?;
            self.boot().cmd()?;
        }
        Ok(s)
    }

    pub fn recover(&mut self) -> Result<()> {
        let mut retries = 250;
        self.stats.recovers += 1;
        while retries > 0 {
            Vpi::sleep_ms(10);
            let id = self.read_id();
            if id == VPI_DEVICE_MAGIK {
                Vpi::sleep_ms(1);
                let res2 = self.config();
                if res2.is_ok() {
                    return self.boot().cmd();
                }
            }
            retries -= 1;
        }
        Err(Vpi::err(
            "Fatal error: Unable to recover connection with VPi",
        ))
    }

    pub fn monitor(&mut self) -> Result<VpiStatus> {
        let res = self.check_status(0);
        match res {
            Err(LinuxI2CError::Io(_)) => {
                let rec = self.recover();
                if rec.is_ok() {
                    self.check_status(1)
                } else {
                    Err(Vpi::err(
                        "Fatal error: Unable to recover connection with VPi",
                    ))
                }
            }
            Ok(mut st) => {
                st.recover_type = 0;
                if !st.is_running {
                    st.recover_type = 2u8;
                    let _ = self.recover();
                }
                Ok(st)
            }
            _ => res,
        }
    }

    pub fn run(&mut self, cmd: &VpiCmd, js: bool) -> Result<VpiCmdOutput> {
        match cmd {
            VpiCmd::Nop => Ok(VpiCmdOutput::t_or_j("Nop", js)),
            VpiCmd::Boot => {
                self.boot().cmd()?;
                Ok(VpiCmdOutput::t_or_j("Booted", js))
            }
            VpiCmd::Init => {
                self.init().cmd()?;
                Ok(VpiCmdOutput::t_or_j("Initialized", js))
            }
            VpiCmd::Shutdown => {
                self.shutdown().cmd()?;
                Ok(VpiCmdOutput::t_or_j("Shutdown started", js))
            }
            VpiCmd::HardShutdown => {
                self.hard_shutdown().cmd()?;
                Ok(VpiCmdOutput::t_or_j("Hard Shutdown", js))
            }
            VpiCmd::Config => {
                self.config()?;
                Ok(VpiCmdOutput::t_or_j("Configuration activated", js))
            }
            VpiCmd::Feed => {
                self.feed().cmd()?;
                Ok(VpiCmdOutput::t_or_j("Watchdog updated", js))
            }
            VpiCmd::Status => {
                let st = self.check_status(0)?;
                let ret = VpiCmdOutput::Status(st);
                if js {
                    Ok(ret.to_json())
                } else {
                    Ok(ret)
                }
            }
            VpiCmd::Reset => {
                self.reset().cmd()?;
                Ok(VpiCmdOutput::t_or_j("Board reset done", js))
            }
            VpiCmd::Recover => {
                self.recover()?;
                Ok(VpiCmdOutput::t_or_j("Board recover done", js))
            }
            VpiCmd::Stats => {
                let s = self.get_stats();
                let ret = VpiCmdOutput::Stats(s);
                if js {
                    Ok(ret.to_json())
                } else {
                    Ok(ret)
                }
            }
            VpiCmd::Wake(mins) => {
                self.wake(*mins).config()?; // Config will enable if needed.
                Ok(VpiCmdOutput::t_or_j(
                    format!("Wake enabled for {} minutes after power off", mins).as_str(),
                    js,
                ))
            }
            VpiCmd::IrqWake(en) => {
                self.wake_irq(*en).cmd()?;
                Ok(VpiCmdOutput::t_or_j(
                    format!("Wake irq enabled={}", en).as_str(),
                    js,
                ))
            }
            VpiCmd::Uuid => {
                let ret = VpiCmdOutput::Uuid(self.get_uuid());
                if js {
                    Ok(ret.to_json())
                } else {
                    Ok(ret)
                }
            }
            VpiCmd::Wdg(secs) => {
                self.wdg(*secs).config()?;
                Ok(VpiCmdOutput::t_or_j(
                    format!("Watchog set to {} seconds", secs).as_str(),
                    js,
                ))
            }
            VpiCmd::Led(led) => {
                self.led_now(*led)?;
                Ok(VpiCmdOutput::t_or_j(
                    format!("Led set to [mode={},value={}]", led.led_mode, led.led_val).as_str(),
                    js,
                ))
            }
            VpiCmd::Fan(speed) => {
                self.fan_now(*speed)?;
                Ok(VpiCmdOutput::t_or_j(
                    format!("Fan set to speed={}", speed).as_str(),
                    js,
                ))
            }
            VpiCmd::Beep(buzz_pars) => {
                self.buzz_now(buzz_pars)?;
                Ok(VpiCmdOutput::t_or_j(
                    format!(
                        "Issued {} beeps [mode:{}-{}-{}]",
                        buzz_pars.buzz_count,
                        buzz_pars.buzz_freq,
                        buzz_pars.buzz_b_tm,
                        buzz_pars.buzz_p_tm
                    )
                    .as_str(),
                    js,
                ))
            }
            VpiCmd::Timing(tim) => {
                self.timings(tim).config()?;
                unsafe {
                    Ok(VpiCmdOutput::t_or_j(
                        format!(
                            "Button timming set to [short={}ms,space={}ms,hold={}s,grace={}s]",
                            tim.short_tm, tim.space_tm, tim.hold_tm, tim.grace_tm
                        )
                        .as_str(),
                        js,
                    ))
                }
            }
            VpiCmd::Divisor(div) => {
                self.rev_divisor(*div).config()?;
                Ok(VpiCmdOutput::t_or_j(
                    format!("Fan RPM divisor set to {} per turn", div).as_str(),
                    js,
                ))
            }
            VpiCmd::PwmFreq(pwmfreq) => {
                self.pwm_freq(*pwmfreq).config()?;
                Ok(VpiCmdOutput::t_or_j(
                    format!("PWM frequency set to {} Hz", pwmfreq).as_str(),
                    js,
                ))
            }
            VpiCmd::Output(val) => {
                self.output(*val).cmd()?;
                Ok(VpiCmdOutput::t_or_j(
                    format!("Output value changed to {}", val).as_str(),
                    js,
                ))
            }
        }
    }
}
