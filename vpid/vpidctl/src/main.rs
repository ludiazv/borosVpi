use vpi::Vpi;
use std::path::PathBuf;
use ansi_term::Colour::{Green,Red};
use std::process::exit;


use clap::{App, SubCommand};
const VERSION :&'static str= "1.0";

fn show_error(e : &std::error::Error) -> ! {
    eprintln!("{} => {}", Red.paint("ERROR"), e);
    exit(1);
}

fn show_error_str(e : &str) -> ! {
    show_error(&clap::Error::with_description(e,clap::ErrorKind::UnknownArgument))
}

fn show_success(msg: &str,quiet:bool) -> ! {
    if !quiet {
        println!("{} => {}", Green.paint("SUCCESS"),msg);
    }
    exit(0);
}

fn main() {

    let matches = App::new("vpidctl")
                          .version(VERSION)
                          .author("LDV")
                          .about("Command line tool for vpid")
                          .subcommand(SubCommand::with_name("cmd")
                                      .about("Send commands to the vpid service")
                                      .args_from_usage(
                                      "-s, --socket=[socket] 'Socket of vpid service'
                                       -q, --quiet           'Quiet output'
                                       <CMD>                 'Command to send'
                                       [args]...             'Argumments of command'"))
                          .subcommand(SubCommand::with_name("dcmd")
                                      .about("Send commands to the vpi board")
                                      .version(VERSION)
                                      .args_from_usage(
                                      "-d, --device=[dev]    '/dev/i2c-? device path [default:/dev/i2c-1]'
                                       -a, --address=[addr]  'I2C address [default: 0x33]'
                                       -q, --quiet           'Quiet mode'
                                       <CMD>                 'Command to send'
                                       [args]...             'Argumments of command'"))
                          .subcommand(SubCommand::with_name("firmware")
                                      .about("Update vpi board firmware")
                                      .version(VERSION)
                                      .args_from_usage(
                                      "-d, --device=[dev]    '/dev/i2c-? device path [default:/dev/i2c-1]'
                                       -a, --address=[addr]  'I2C address [default: 0x22]'
                                       -r. --rstpin=[pin]    'GPIO number of reset pin [default:4]'
                                       <BINFILE>             '.bin file with the firmware'"))
                          .get_matches();

    
    if let Some(m) = matches.subcommand_matches("firmware") {
        let dev=m.value_of("device").unwrap_or("/dev/i2c-1");
        let addr:u8=m.value_of("address").unwrap_or("0x22").parse().unwrap_or(0x22);
        let pin:u16=m.value_of("rstpin").unwrap_or("4").parse().unwrap_or(4);
        let file=PathBuf::from(m.value_of("BINFILE").unwrap());
        println!("Updating firware with:\n=>I2C Bus:{}\n=>Address: 0x{:x}\n=>Reset Pin:{}",dev,addr,pin);
        println!("Firmware:{}",file.to_str().unwrap());
        match vpi::uploader::upload(addr,&PathBuf::from(dev),&file,pin) {
            Err(e) => show_error(&e) ,
            Ok(_) => show_success("firmware updated.", false)
        }
    }

    if let Some(m)= matches.subcommand_matches("dcmd") {
        let cmd=m.value_of("CMD").unwrap_or("nop");
        let dev=m.value_of("device").unwrap_or("/dev/i2c-1");
        let addr:u8=m.value_of("address").unwrap_or("0x33").parse().unwrap_or(0x33);
        let quiet:bool=m.is_present("quiet");
        let mut vpi=Vpi::new(Some(addr as u16),!quiet);
        vpi.open(&PathBuf::from(dev)).unwrap_or_else(|e| show_error(&e) );

        match cmd {
            "monitor" => {
                loop {

                }
            }
            "boot" => {
                vpi.boot().cmd().unwrap_or_else(|e| show_error(&e) );
                show_success("Booted", quiet);
            },
            //"boot" =>
            //"led" =>
            "fan" => {
                let speedv:Vec<&str>=m.values_of("args").unwrap_or_else(|| show_error_str("missing speed 0-255 argument")).collect();
                let mut speed:u32=speedv[0].parse::<u32>().unwrap_or_else(|e| show_error(&e));
                if speed > 255 { speed=255 } 
                vpi.fan_now(speed as u8).unwrap_or_else(|e| show_error(&e));
                show_success(format!("Set fan to {}",speed).as_str(),quiet);
            },
            _ => show_error_str("Invalid command.") 
        }

    }

}
