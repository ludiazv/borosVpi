use vpi::{Vpi};
use vpi::cmd::{VpiCmd,VpiCmdOutput};
use std::path::PathBuf;
use ansi_term::Colour::{Green,Red,Yellow,Blue};
use std::process::exit;
use std::{thread, time};
use std::io::{Write,Read};
use std::os::unix::net::UnixStream;
use serde_json;


use clap::{App, SubCommand};
const VERSION :&'static str= "0.1";

fn show_error(e : & dyn std::error::Error) -> ! {
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
fn show_success_json(j:&String,quiet:bool) -> ! {
    if !quiet { println!("{}",j); }
    exit(0);
}
fn show_error_json(j:&String) -> ! {
    eprintln!("{}",j);
    exit(1);
}

fn main() {

    let matches = App::new("vpidctl")
                          .version(VERSION)
                          .author("LDV")
                          .about("Command line tool for vpid")
                          .subcommand(SubCommand::with_name("cmd")
                                      .about("Send commands to the vpid service")
                                      .version(VERSION)
                                      .args_from_usage(
                                      "-s, --socket=[socket] 'Socket of vpid service'
                                       -q, --quiet           'Quiet output'
                                       <CMD>                 'Command to send'
                                       [args]...             'Argumments of command'"))
                          .subcommand(SubCommand::with_name("dcmd")
                                      .about("Send commands to the vpi board directly")
                                      .version(VERSION)
                                      .args_from_usage(
                                      "-d, --device=[dev]    '/dev/i2c-? device path [default:/dev/i2c-1]'
                                       -a, --address=[addr]  'I2C address [default: 0x33]'
                                       -q, --quiet           'Quiet mode'
                                       -b, --debug           'Debug mode'
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
        let mut args:Vec<&str>=[].to_vec();
        if let Some(ar) = m.values_of("args") {
            args=ar.collect();
        }
        let dev=m.value_of("device").unwrap_or("/dev/i2c-1");
        let addr:u8=m.value_of("address").unwrap_or("0x33").parse().unwrap_or(0x33);
        let quiet:bool=m.is_present("quiet");
        let debug:bool=m.is_present("debug");
        let mut vpi=Vpi::new(Some(addr as u16),debug);
        vpi.open(&PathBuf::from(dev)).unwrap_or_else(|e| show_error(&e) );

        if let Some(basic_command) = VpiCmd::from_cmd_vec(cmd, args) {
            match vpi.run(&basic_command, false) {
                Ok(VpiCmdOutput::Text(msg)) |
                Ok(VpiCmdOutput::Uuid(msg))   => show_success(msg.as_str(), quiet) ,
                Ok(VpiCmdOutput::Stats(st))   => show_success(format!("{:?}",st).as_str(), quiet) ,
                Ok(VpiCmdOutput::Status(st))  => show_success(format!("{:?}",st).as_str(), quiet) ,
                Ok(VpiCmdOutput::Json(s))     => show_success(s.as_str(), quiet),
                Err(e) => show_error(&e)
            }
        } else { // composite commands

            match cmd.to_lowercase().as_str() {
                "dump" => {
                    vpi.read_all().unwrap_or_else(|e| show_error(&e));
                    if !debug && !quiet { vpi.dump_regs(); }
                    show_success("Registers dumped", quiet);
                },
                "rpmtest" => {
                    println!("Set fan to 0");
                    vpi.fan_now(0).unwrap_or_else(|e| show_error(&e));
                    thread::sleep(time::Duration::from_millis(2000));
                    for i in (0..256).step_by(5) {
                        vpi.fan_now(i as u8).unwrap_or_else(|e| show_error(&e));
                        thread::sleep(time::Duration::from_millis(2000));
                        let st=vpi.check_status(0).unwrap_or_else(|e| show_error(&e));
                        println!("{}=>{}",i,st.rpm);
                        thread::sleep(time::Duration::from_millis(100));
                    }
                    vpi.fan_now(0).unwrap_or_else(|e| show_error(&e));
                    show_success("Done", quiet);
                },
                "monitor" => {
                    let mut w=std::io::stdout();
                    let mut cnt:u64 =0;
                    if !quiet { println!("{}", Blue.paint("Starting monitor...")); }
                    loop {
                        let st=vpi.monitor().unwrap_or_else(|e| show_error(&e));
                        cnt+=1;
                        if !quiet {
                            let mut out:String = "\r<".to_string();
                            if st.recover_type > 0 {
                                out+=&format!("({} {})",Red.paint("Recover"),st.recover_type);
                            }
                            if st.has_click {
                                out+=&format!("({} PWR[{},{}] AUX[{},{}])", Green.paint("Buttons"),
                                         st.pwr_short,st.pwr_long,st.aux_short,st.aux_long);
                            }
                            if st.is_wdg_enabled {
                                out+=&format!("({})",Blue.paint("WDG enabled"));
                            }
                            if st.is_wake_enabled {
                                out+=&format!("({})",Blue.paint("Wake Enabled"));
                            }
                            if st.is_wake_irq_enabled {
                                out+=&format!("({})",Blue.paint("IRQ Wake enabled"));
                            }
                            out+=&format!("(out:{})",st.out_value);
                            if st.has_irq { out+=&format!("({})",Yellow.paint("IRQ")); }
    
                            if st.has_rpm {
                                out= out + &format!("({} [{} rpm])",Green.paint("Fan speed"),st.rpm);
                            }
                            if st.is_running {
                                out = out + &format!("({})",Green.paint("Vpi is running"));
                            }
                            if out.len() == 2 {
                                out=  out + &format!("{}>",Yellow.paint("No events"));
                            } else {
                                out+=">";
                            }
                            out += &format!("[{}]",cnt);
                            let _=w.write(out.as_bytes());
                            let _=w.write(b"\x1b[K");
                            let _=w.flush();
                            if st.has_click || st.recover_type>0 { println!("{}",out); }
                            if cnt % 120 ==0 { 
                                let stats=vpi.get_stats();
                                println!("\n{}:{:?}",Blue.paint("Stats"),stats);
                            }
                        }
                        thread::sleep(vpi.poll_time());
                    }
                },
                _ => show_error_str(format!("Invalid command {}.",cmd).as_str()) 
            } 
        }// Composit commands
    }

    // commands to daemon
    if let Some(m) = matches.subcommand_matches("cmd") {
        let cmd=m.value_of("CMD").unwrap_or("nop");
        let quiet : bool=m.is_present("quiet");
        let socket_name=m.value_of("socket").unwrap_or("/var/run/vpid.sock");
        let mut args:String=String::new();
        if let Some(ar) = m.values_of("args") {
            let arv:Vec<&str>= ar.collect();
            args= arv.join(" ");
        }
        match UnixStream::connect(socket_name) {
            Ok(mut stream) => {
                if let Err(e)= stream.write_fmt(format_args!("{} {}\n",cmd,args)) { show_error(&e); }
                if let Err(e)= stream.flush() { show_error(&e); }
                let mut resp=String::new();
                if stream.read_to_string(&mut resp).is_ok() {
                    let objr : serde_json::Result<serde_json::Value>= serde_json::from_str(resp.as_str());
                    if let Ok(obj) = objr {
                        if obj["result"] == "ok" { show_success_json(&resp, quiet); } 
                        else { show_error_json(&resp); }
                    } else {
                        println!("{}",resp);
                        show_error(&objr.err().unwrap());
                    }
                } else {
                    show_error_str("vpid did not responded or timeout occurred");
                }

            },
            Err(e) => show_error(&e)
        }
    } // commands to damon

}
