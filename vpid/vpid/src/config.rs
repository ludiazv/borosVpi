//! # Configuration module
//!  Load configuration file and 
//extern crate serde_yaml;
//extern crate serde_piecewise_default_derive;
//extern crate strum;
//#[macro_use]
//extern crate strum_macros;


use serde::Deserialize;
use serde_piecewise_default::DeserializePiecewiseDefault;

use std::path::{PathBuf,Path};
use std::fs;
use failure::Error;
use std::result;
use vpi::VpiTimes;



#[derive(Deserialize, PartialEq, Eq, Debug)]
pub enum VpiRuleType {
    Shutdown,
    Reboot,
    Script,
    Nop
}

#[derive(Deserialize, PartialEq, Eq, Debug)]
pub struct VpiRule {
    short:Option<u8>,
    long:Option<u8>,
    or: bool,
    kind: VpiRuleType,
    cmd:Option<String>
}

impl Default for VpiRule {
    fn default() -> VpiRule {
        VpiRule {
            short:None,
            long:None,
            or: false,
            kind: VpiRuleType::Nop,
            cmd: None
        }
    }
}

#[derive(EnumString,Eq,PartialEq,Debug,Deserialize)]
pub enum VpiFanMode {
    Off,
    On,
    Pid,
    Linear
}

#[derive(Deserialize,PartialEq,Eq,Debug)]
pub struct VpiFanConfig {
     pins: u8,
     divisor: u8,
     thermal_path: String,
     mode: VpiFanMode,
     linear_max_temp: Option<i32>,
     linear_min_temp: Option<i32>,
     pid_desired_temp: Option<i32>
}

impl Default for VpiFanConfig {
    fn default() -> Self {
        VpiFanConfig {
            pins: 2,
            divisor: 2,
            thermal_path: String::from("/sys/class/thermal/thermal_zone0/temp"),
            mode: VpiFanMode::Off,
            linear_max_temp: Some(65*1000),
            linear_min_temp: Some(35*1000),
            pid_desired_temp:Some(45*1000),
        }

    }
}


//#[derive(Deserialize, PartialEq, Eq, Debug)]
//pub struct VpiWeb {
//    bind:String
//} 

#[derive(Deserialize, PartialEq, Eq, Debug)]
pub struct VpiMqtt {
    host:String,
    port:u16,
    id: String,
    cmd_topic: String,
    evt_topic: Option<String>,
    user: Option<String>,
    passw: Option<String>
}

#[derive(DeserializePiecewiseDefault, PartialEq, Eq, Debug)]
pub struct VpiConfig {
    device:             PathBuf,
    shutdown_command:   String,
    reboot_command:     String,
    short_time:         u16,
    space_time:         u16,
    grace_time:         u8,
    hold_time:          u8,
    poll_time:          u32,
    watch_dog:          u8,
    rules:              Vec<VpiRule>,
    fan:                Option<VpiFanConfig>,
    //web:                Option<VpiWeb>,
    mqtt:               Option<VpiMqtt>
}

// Just retrun default values
impl Default for VpiConfig {
    fn default() -> Self {
        VpiConfig {
            device : PathBuf::from("/dev/i2c-1"),
            shutdown_command: "/sbin/shutdown -P -t 1 now \"Vpid is shutting down the system\"".to_string(),
            reboot_command: "/sbin/shutdown -r -t 1 now \"Vpid is rebooting the system\"".to_string(),
            short_time: 250,
            space_time: 1250,
            hold_time:  7,
            grace_time: 15,
            poll_time:  250,
            watch_dog:  0,
            rules: vec!(),
            //web: None,
            fan:None,
            mqtt: None
        }
    }
}

impl VpiConfig {

    pub fn load(cfile : PathBuf) -> result::Result<VpiConfig,Error> {
        let yml = fs::read_to_string(cfile)?;
        let vpi = serde_yaml::from_str(yml.as_str())?;
        Ok(vpi)
    }

    pub fn dev(&self) -> &Path {
        self.device.as_path()
    }
    pub fn fan(&self) -> &Option<VpiFanConfig> {
        &self.fan
    }
    pub fn times(&self) -> VpiTimes {
        let v=VpiTimes::new(self.short_time,self.space_time,self.hold_time,self.grace_time);
        v
    }

}
