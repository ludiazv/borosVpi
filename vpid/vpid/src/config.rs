/// Configuration module for vpid
/// Abstract & parse configuration options
use serde::Deserialize;
use serde_piecewise_default::DeserializePiecewiseDefault;
use std::path::{PathBuf};
use std::fs;
use vpi::VpiTimes;

// Crate used
use crate::fan::VpiFanConfig;
use crate::error::{Result,ResultExt,ReadConfig,ParseConfig};

/// Rule types
#[derive(Deserialize, PartialEq, Eq, Debug)]
pub enum VpiRuleType {
    /// Execute system shutdown (lauch command)
    Shutdown,
    /// Execute system Reboot
    Reboot,
    /// Run shell script
    Shell,
    /// Run Lua script
    Lua,
    // No operation rule
    Nop
}

/// Definiton of a Vpid Rule
#[derive(DeserializePiecewiseDefault, PartialEq, Eq, Debug)]
pub struct VpiRule {
    /// Name of the rule for reference
    pub name:    String,
    /// Condition of the rule (expression)
    pub when:    String, 
    /// Kind of the rule see *VpiRuleType*
    pub kind:    VpiRuleType,
    /// Script to execute (Lua or shell)
    pub script:  Option<String>,
    /// Run shell script asyncronous (only apply to shellscripts)
    pub asyncr:  bool,
    /// Script execution timeout
    pub timeout: u32,
}

/// Default trait implementation for VpiRule
impl Default for VpiRule {
    fn default() -> VpiRule {
        VpiRule {
            name: "no name".to_string(),
            when: "".to_string(),
            kind: VpiRuleType::Nop,
            script: None,
            asyncr: false,
            timeout: 0,
        }
    }
}

/// Minservice definition in lua
#[derive(DeserializePiecewiseDefault, PartialEq, Eq, Debug)]
pub struct VpiMiniService {
    pub name: String,
    pub script: String,
}
///Default implementa for VpiMiniService
impl Default for VpiMiniService {
    fn default() -> Self {
        Self {
            name: "no name-miniservice".to_string(),
            script: r#"print("Empty mini service - Hi from lua")"#.to_string(),
        }
    }
}
/// Configurarion file structure 
#[derive(DeserializePiecewiseDefault, Debug)]
pub struct VpiConfig {
    pub shutdown_command:   String,
    pub reboot_command:     String,
    pub shell:              String,
    pub short_time:         u16,
    pub space_time:         u16,
    pub grace_time:         u8,
    pub hold_time:          u8,
    pub poll_time:          Option<u32>,
    pub wake:               u16,
    pub wake_irq:           bool,
    pub watchdog:           u8,
    pub watchdog_autofeed:  bool,
    pub rules:              Vec<VpiRule>,
    pub fan:                Option<VpiFanConfig>,
    pub services:           Vec<VpiMiniService>,
}

// Just retrun default values
impl Default for VpiConfig {
    fn default() -> Self {
        VpiConfig {
            shutdown_command: "/sbin/shutdown -P -t 1 now \"Vpid is shutting down the system\"".to_string(),
            reboot_command: "/sbin/shutdown -r -t 1 now \"Vpid is rebooting the system\"".to_string(),
            shell: "/bin/sh -c".to_string(),
            short_time: 250,
            space_time: 1250,
            hold_time:  7,
            grace_time: 15,
            poll_time:  None,
            watchdog:   0,
            watchdog_autofeed: true,
            rules:      vec!(),
            fan:        None,
            wake:       0u16,
            wake_irq:   false,
            services:   vec!(),

        }
    }
}

impl VpiConfig {

    pub fn load(cfile : &PathBuf) -> Result<VpiConfig> {
        //let yml = fs::read_to_string(cfile).and_then(|yml| serde_yaml::from_str );
        let yml = fs::read_to_string(cfile).context(ReadConfig { filename: cfile })?;
        let vpi = serde_yaml::from_str(yml.as_str()).context(ParseConfig { filename:cfile})?;
        Ok(vpi)
    }
    pub fn times(&self) -> VpiTimes {
        VpiTimes::new(self.short_time,self.space_time,self.hold_time,self.grace_time)
    }
    pub fn get_poll_time(&self) -> std::time::Duration {
        if let Some(p) = self.poll_time {
            std::time::Duration::from_millis(std::cmp::max(500u32,p) as u64 )
        } else {
            std::time::Duration::from_millis( (self.space_time as u64)/3u64 )
        }
    } 

}
