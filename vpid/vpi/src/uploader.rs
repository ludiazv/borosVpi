//! Uploader submodule to upload new firmware using custom stm8sboot loader.
//! Boot loader code is here: https://github.com/ludiazv/stm8-bootloader
//!
use i2cdev::core::I2CDevice;
use i2cdev::linux::*;
use pbr::ProgressBar;
use std::fs::OpenOptions;
use std::io::ErrorKind;
use std::io::Read;
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;
use sysfs_gpio::{Direction, Pin};

/// BLOCK SIZE IS 64 for stms8 low desity
const BLOCK_SIZE: usize = 64;
const ACK: [u8; 2] = [0xaa, 0xbb];
const NACK: [u8; 2] = [0xde, 0xad];

/// Just a simple crc8 computation compatble with bootloader
fn crc8_update(data: u8, crc_in: u8) -> u8 {
    let mut crc: u8 = crc_in ^ data;
    for _i in 0..8 {
        if crc & 0x80 != 0 {
            crc = (crc << 1) ^ 0x07;
        } else {
            crc <<= 1;
        }
    }
    return crc & 0xFF;
}

/// Compute crc8 of a buffer
pub fn buff_crc(data: &[u8], mut crc_in: u8) -> u8 {
    for i in data.iter() {
        crc_in = crc8_update(*i, crc_in);
    }
    return crc_in;
}

/// Compute the crc of a binary file
/// # Arguments
/// `file` : Path of the file
///
///
fn get_crc(file: &PathBuf) -> Result<(u8, u8), std::io::Error> {
    let mut crc: u8 = 0;
    let mut chunk: [u8; BLOCK_SIZE] = [0xFF; BLOCK_SIZE];
    let mut f = OpenOptions::new().read(true).open(file)?;
    let size = f.metadata()?.len();
    let mut sbl = size / (BLOCK_SIZE as u64);
    sbl = if (size % (BLOCK_SIZE as u64)) > 0 {
        sbl + 1
    } else {
        sbl
    };
    let mut len = f.read(&mut chunk)?;
    while len > 0 {
        crc = buff_crc(&chunk, crc);
        chunk = [0xFF; BLOCK_SIZE]; // Reset chuck
        len = f.read(&mut chunk)?; // Read next chunck
    }
    Ok((crc, sbl as u8))
}

/// Resets the STM8S with High pulse using a pin of the SBC
/// # Arguments
/// `pin`- Pin number
fn reset(pin: u16) -> Result<(), std::io::Error> {
    let pin = Pin::new(pin as u64);

    let block = || -> Result<(), sysfs_gpio::Error> {
        pin.export()?;
        sleep(Duration::from_millis(3500)); // Big delay for export
        pin.set_direction(Direction::Out)?;
        pin.set_value(0)?;
        sleep(Duration::from_millis(500));
        pin.set_value(1)?;
        sleep(Duration::from_millis(500));
        pin.set_value(0)?;
        Ok(())
    };

    match block() {
        Ok(()) => Ok(()),
        Err(e) => {
            println!("{:?}", e);
            Err(std::io::Error::new(
                ErrorKind::PermissionDenied,
                "could not reset vpi board via gpio, please check conection & permissions",
            ))
        }
    }
}

/// Unexport a pin
/// # Arguments
/// `pin` - pin to unexport
pub fn unexport(pin: u16) {
    let pin = Pin::new(pin as u64);
    let _ = pin.unexport();
}

/// Small pause
#[inline]
fn pause() {
    sleep(Duration::from_millis(20));
}
/// Uploads the firmeware using i2c
/// # Arguments
/// `addr` - i2c address of the i2c board
/// `dev_path` - path to i2c devive /dev/i2c-XX
/// `file` - file path of the firmware
/// `rst_pin` - pin used for reset the stm8s chip (rest will be high level)
pub fn upload(
    addr: u8,
    dev_path: &PathBuf,
    file: &PathBuf,
    rst_pin: u16,
) -> Result<(), LinuxI2CError> {
    let mut dev = LinuxI2CDevice::new(dev_path, addr as u16)?;
    let crc: u8;
    let blocks: u8;
    if file.exists() {
        // Compute CRC
        match get_crc(file) {
            Ok((c, l)) => {
                crc = c;
                blocks = l;
            }
            Err(e) => return Err(LinuxI2CError::Io(e)),
        }
        println!("Firmware readed. blocks={},CRC={:x}", blocks, crc);
        // Initiate bootloader sequence
        reset(rst_pin)?;
        let req: [u8; 7] = [0xde, 0xad, 0xbe, 0xef, blocks, crc, crc]; // Activation msg
        let mut resp: [u8; 2] = NACK;
        print!("Sending upload request...");
        pause();
        dev.write(&req)?;
        pause();
        print!("R.");
        dev.read(&mut resp)?;
        if resp != ACK {
            return Err(LinuxI2CError::Io(std::io::Error::new(
                ErrorKind::ConnectionRefused,
                "Bootloader activation response:NACK",
            )));
        }
        println!("Ok! Starting upload...");
        // Upload chucks
        let mut pb = ProgressBar::new(blocks as u64);
        let mut f = OpenOptions::new().read(true).open(file)?;
        for _i in 0..blocks {
            let mut chunk: [u8; BLOCK_SIZE] = [0xFF; BLOCK_SIZE];
            f.read(&mut chunk)?;
            dev.write(&chunk)?;
            pb.inc();
            pause();
        }
        // ACK confirmation of the firmware
        resp = NACK;
        print!("Confirming upload...");
        pause();
        dev.read(&mut resp)?;
        if resp != ACK {
            Err(LinuxI2CError::Io(std::io::Error::new(
                ErrorKind::ConnectionRefused,
                "Bootloader activation response:NACK",
            )))
        } else {
            println!("Ok!");
            Ok(())
        }
    } else {
        Err(LinuxI2CError::Io(std::io::Error::new(
            ErrorKind::NotFound,
            "Firmware file not found",
        )))
    }
}
