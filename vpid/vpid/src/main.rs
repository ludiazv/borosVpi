
// External Crates
#[macro_use]
extern crate log;
extern crate simple_logger;
#[macro_use]
extern crate crossbeam_channel;

use std::path::PathBuf;
use std::str::FromStr;
use clap::App;
use std::thread;
use std::time::{Duration,Instant};
use crossbeam_channel::{bounded,tick,never,Sender,Receiver};
use signal_hook::{iterator::Signals, SIGTERM, SIGHUP, SIGINT};
use std::sync::{Arc, Mutex};
use std::process::exit;

// Internal
use vpi::{Vpi,VpiStatus,VpiTimes};
use vpi::cmd::{VpiCmd};
use cmd::{VpiCommand,VpiCommandBody};
use config::VpiConfig;
//use fan::VpiFanConfig;
use engine::{Engine};
use crate::error::{Result,ResultExt,I2cOpen,VpiConfigureError};

// Modules declaration
mod error;
mod config;
mod fan;
mod sock;
mod engine;
mod cmd;

// Constant
const VPID_VERSION :&'static str = "0.1.0";

fn main() -> ! {
    let matches = App::new("vpid")
                          .version(VPID_VERSION)
                          .author("LDV")
                          .about("vpi daemon")
                          .args_from_usage(
                            "-s, --socket=[socket] 'Socket of vpid service, default:/var/run/vpid.sock'
                             -c, --config=[file]   'Config file, default:/etc/vpid/vpid.yml'
                             -d, --device=[i2cdev] 'i2c-dev path, default:/dev/i2c-1'
                             -a, --address=[addr]  'i2c address, default:0x33'")
                          .get_matches();

    let socket_path=     matches.value_of("socket").unwrap_or("/var/run/vpid.sock");
    let cfg_file=   matches.value_of("config").unwrap_or("/etc/vpid/vpid.yml");
    let device_s=   matches.value_of("device").unwrap_or("/dev/i2c-1");
    let device  =   PathBuf::from_str(device_s).unwrap();
    let address_s=  matches.value_of("address").unwrap_or("0x33");
    let address  =  vpi::from_str_address(address_s).unwrap_or(0x33u8);

    // Init log
    simple_logger::init_by_env();
    info!("Vpid daemon init {}",VPID_VERSION);
    info!("Parameters => socket:{}, config file path:{}, device:{}, address:0x{:02X}",socket_path,cfg_file,device_s,address);
    // Check device
    if !device.as_path().exists() {
        error!("I2C device {} not found. Aboring",device_s);
        exit(1);
    } else {
        info!("I2C device {} found",device_s);
    }
    // Check configuration
    let cfg_path= PathBuf::from(cfg_file);
    if !cfg_path.as_path().exists() {
        error!("Configuration file {} do not exist. Aborting",cfg_file);
        exit(1);
    } 
    info!("Validating configuration file...");
    if let Err(e) = VpiConfig::load(&cfg_path) {
        error!("Configuration file invalid [{}] Aborting",e);
        exit(1);
    }
    info!("Configuration validated!");
    // socket server
    let (command_sender,command_receiver) = bounded::<VpiCommand>(15);
    info!("Starting socket server");
    if let Err(e) = sock::run_socket(&PathBuf::from(socket_path),&command_sender) {
        error!("Could not start socket server [{}] Aborting.",e);
        exit(2);
    }
    info!("Socket server started!");
    
    // Signal manager
    info!("Init signal manager");
    let signals = Signals::new(&[SIGTERM,SIGHUP,SIGINT]).unwrap();
    let signal_sender=command_sender.clone();
    thread::spawn(move || {
        for sig in signals.forever() {
            info!("OS signal received [code:{}]",sig);
            if sig==SIGHUP {
                let _=signal_sender.send(VpiCommand::new_nbc(VpiCommandBody::ReloadConfig));
            } else {
                let _=signal_sender.send(VpiCommand::new_nbc(VpiCommandBody::Signal(sig)));
            }
        }
    });
    info!("Signal manager started!");
    // Launch server daemon
    info!("Activating daemon");
    let mut return_code:i32=0;
    loop {
        let res = serve(&cfg_path,&device,address,&command_sender,&command_receiver);
        if let Ok(ret) =  res {
            if ret == RET_CODE_EXIT { 
                break;
            } // exit loop to exit
        } else {
            error!("Fatal error: Aborting vpid exectuion: {:?}",res);
            return_code=3;
            break;
        }
    }
    info!("Shuting down service gracefully...");
    sock::close_socket(&PathBuf::from(socket_path));
    exit(return_code);
}

const RET_CODE_RELOAD:i32 =0i32;
const RET_CODE_EXIT:i32   =1i32;

fn serve(cfg_file : &PathBuf,
         device : &PathBuf,
         addr: u8,
         command_sender: &Sender<VpiCommand>,
         command_receiver : &Receiver<VpiCommand>) -> Result<i32>{

    // Reload the confing
    let cfg=config::VpiConfig::load(cfg_file)?;
    // Init i2c
    let mut vpi=Vpi::new(Some(addr as u16),false);
    vpi.open(device).context( I2cOpen { dev: device, addr: vpi.get_addr() } )?;
    let mut last_status = vpi.check_status(0).context(I2cOpen { dev: device, addr: vpi.get_addr() } )?;
    let mut last_stats = vpi.get_stats();
    // apply config and boot
    //let timing=VpiCmd::from_string(format!("timing {} {} {} {}",cfg.short_time,cfg.space_time,cfg.hold_time,cfg.grace_time)).unwrap();
    //command_sender.send(VpiCommand::new_nbc(timing));
    
    
    // Set up timers
    let monitor= tick(cfg.get_poll_time()); // tick(Duration::from_secs(1));
    let mut auto_feed : Receiver<Instant>=never::<Instant>(); // initialy off
    let mut fan_control: Receiver<Instant>=never::<Instant>(); // intially off

    // Fan configuration
    let mut fan_controller =cfg.fan.clone(); // Clone the fan to avoid as cfg will be borrowed.
    if let Some(ref fan) = cfg.fan {
        info!("Configure pwm frequency {} Hz and fan divisor to {} turns",fan.get_pwmfreq(),fan.get_divisor());
        vpi.pwm_freq(fan.get_pwmfreq()).rev_divisor(fan.get_divisor());
        fan_control = tick(Duration::from_secs(3));  
    }
    // WDG 
    

    // Times
    vpi.timings(&VpiTimes::new(cfg.short_time, cfg.space_time, cfg.hold_time, cfg.grace_time));
    info!("Set button timmings short:{} ms space:{} ms hold:{} s",cfg.short_time,cfg.space_time,cfg.hold_time);
    info!("Set shutdown grace time to {} s",cfg.grace_time);
   
    vpi.config().context(VpiConfigureError {} )?;
    // Send te boot command & WDG & Wake & Wake IRQ
    let _=command_sender.send(VpiCommand::new_nbc(VpiCommandBody::Basic(VpiCmd::Boot) ));
    let _=command_sender.send(VpiCommand::new_nbc(VpiCommandBody::Basic(VpiCmd::Wdg(cfg.watchdog)) ));
    let _=command_sender.send(VpiCommand::new_nbc(VpiCommandBody::Basic(VpiCmd::Wake(cfg.wake))   ));
    let _=command_sender.send(VpiCommand::new_nbc(VpiCommandBody::Basic(VpiCmd::IrqWake(cfg.wake_irq)) ));
    // Rule & exectution engine
    let mut engine=Engine::new(&cfg,&command_sender);

    // Main Loop
    loop {
        select! {
            // Monitor
            recv(monitor) -> _ => {
                engine.test_childs(false);
                engine.test_lua_childs(false);
                last_stats=vpi.get_stats();
                if let Ok(st) = vpi.monitor() {
                    if st.has_changed(&last_status) {
                        let _=engine.run_rules(&st,&last_stats);
                    }
                    last_status=st;
                } else {
                    warn!("Failed to monitor");
                }
            },
            // Auto feed 
            recv(auto_feed) -> _ => { let _=vpi.feed().cmd(); },
            // Fan control
            recv(fan_control) -> _ => {
                let fvalue=fan_controller.as_mut().unwrap().regulate();
                if fvalue != vpi.get_fan_value() {
                    let _=vpi.fan_now(fvalue);
                    trace!("Adjusted fan value to {}",fvalue);
                }
            },
            recv(command_receiver) -> cmdr => {
                if cmdr.is_err() {
                    error!("Error receiving command {:?}",cmdr);
                }
                let cmd=cmdr.unwrap_or(VpiCommand::new_nbc(VpiCommandBody::Basic(VpiCmd::Nop)));
                match cmd.body {
                    VpiCommandBody::ReloadConfig => {
                        let _=cmd.send_ok();
                        return Ok(RET_CODE_RELOAD);
                    },
                    VpiCommandBody::Signal(_) => {
                        let _=cmd.send_ok();
                        return Ok(RET_CODE_EXIT);
                    },
                    VpiCommandBody::Basic(ref basic_command) => {
                        match vpi.run(basic_command,true) {
                            Ok(output) =>  { 
                                cmd.send_output(&output);
                                info!("Command executed:{}",output);
                                if let VpiCmd::Wdg(wdg) = basic_command  {
                                    if *wdg == 0u8 || !cfg.watchdog_autofeed {
                                        info!("Disabling watchdog autofeed");
                                        auto_feed=never::<Instant>();
                                    } else {
                                        info!("Enable watchdog autofeed to {} ms",(cfg.watchdog as u64*1000u64)/2u64);
                                        auto_feed=tick(Duration::from_millis( (cfg.watchdog as u64*1000u64)/2u64 ) );
                                    }
                                }
                            },
                            Err(e) => error!("Command {:?} failed",e)
                        }
                    },
                }
            } // match 
        } //Select   
    }//loop
    //Ok(0i32)
}