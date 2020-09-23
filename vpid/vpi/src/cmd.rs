//! Vpi Commands abstraction module.
//! This module abstracts VPI command definitions and 
use crate::{VpiBuzz, VpiLed, VpiStats, VpiStatus, VpiTimes};
use serde_json::json;
use std::fmt;

/// Available VPI commands
#[derive(Debug, Copy, Clone)]
pub enum VpiCmd {
    /// No operation
    Nop,
    /// Notify VPI that the system has booted. 
    Boot,
    /// Restore VPI to Booting state
    Init,
    /// Apply configurations on the VPI board
    Config,
    /// Feed VPI high-level watchdog
    Feed,
    /// Get current board status
    Status,
    /// Hard reset of the board
    Reset,
    /// Recover bords I2C interface by reinit
    Recover,
    /// Get last statistics
    Stats,
    /// Powerof the 
    Shutdown,
    /// Poweroff the boad inmedialty
    HardShutdown,
    /// Set wake time in minutes -> 0 wake is is disbled.
    Wake(u16),
    /// Enable wake by IRQ line (low level)
    IrqWake(bool),
    /// Get boards 96-bit UUIDE
    Uuid,
    /// Configure the high-level 
    Wdg(u8),
    /// Change led
    Led(VpiLed),
    /// Change fan value
    Fan(u8),
    /// Beep 
    Beep(VpiBuzz),
    /// Set configuration times
    Timing(VpiTimes),
    /// Set rpm fan divisor
    Divisor(u8),
    /// Set fan pwm frequency fan
    PwmFreq(u16),
    /// Sets output
    Output(bool),
}

#[derive(Debug)]
pub enum VpiCmdOutput {
    Stats(VpiStats),
    Status(VpiStatus),
    Uuid(String),
    Text(String),
    Json(String),
}

impl fmt::Display for VpiCmdOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stats(s) => write!(f, "{:?}", s),
            Self::Status(s) => write!(f, "{:?}", s),
            Self::Uuid(s) | Self::Text(s) | Self::Json(s) => write!(f, "{}", s),
        }
    }
}

impl VpiCmdOutput {
    pub fn t_or_j(s: &str, js: bool) -> Self {
        if js {
            VpiCmdOutput::json(s)
        } else {
            VpiCmdOutput::Text(s.to_string())
        }
    }
    pub fn json(s: &str) -> Self {
        let js = json!({
            "result": true,
            "data": s
        });
        VpiCmdOutput::Json(js.to_string())
    }

    pub fn to_json(&self) -> Self {
        match self {
            VpiCmdOutput::Stats(s) => {
                let js = json!({
                    "result": true ,
                    "data": s
                });
                VpiCmdOutput::Json(js.to_string())
            }
            VpiCmdOutput::Status(s) => {
                let js = json!({
                    "result": true,
                    "data": s
                });
                VpiCmdOutput::Json(js.to_string())
            }
            VpiCmdOutput::Text(t) | VpiCmdOutput::Uuid(t) => VpiCmdOutput::json(t.as_str()),
            VpiCmdOutput::Json(j) => VpiCmdOutput::Json(j.to_string()),
        }
    }
}

impl VpiCmd {
    pub fn from_vec(v: Vec<&str>) -> Option<Self> {
        if v.len() == 0 {
            return None;
        }
        match v[0].to_lowercase().as_str() {
            "nop" => Some(VpiCmd::Nop),
            "boot" => Some(VpiCmd::Boot),
            "init" => Some(VpiCmd::Init),
            "config" => Some(VpiCmd::Config),
            "feed" => Some(VpiCmd::Feed),
            "status" => Some(VpiCmd::Status),
            "uuid" => Some(VpiCmd::Uuid),
            "reset" => Some(VpiCmd::Reset),
            "recover" => Some(VpiCmd::Recover),
            "stats" => Some(VpiCmd::Stats),
            "shutdown" => Some(VpiCmd::Shutdown),
            "hardshutdown" => Some(VpiCmd::HardShutdown),
            "irqwake" => {
                if v.len() >= 2 && (v[1] == "on" || v[1] == "off") {
                    Some(VpiCmd::IrqWake(v[1] == "on"))
                } else {
                    None
                }
            }
            "watchdog" => {
                if v.len() >= 2 && v[1].parse::<i32>().is_ok() {
                    let v = v[1].parse::<i32>().unwrap();
                    if v >= 0 && v <= u8::max as i32 {
                        Some(VpiCmd::Wdg(v as u8))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            "wake" => {
                if v.len() >= 2 && v[1].parse::<i32>().is_ok() {
                    let v = v[1].parse::<i32>().unwrap();
                    if v >= 0 && v <= u16::max as i32 {
                        Some(VpiCmd::Wake(v as u16))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            "led" => {
                let mut r: Option<VpiCmd> = None;
                let mut val: u8 = 0;
                if v.len() >= 2 {
                    let m = match v[1].to_lowercase().as_str() {
                        "on" => Some(1u8),
                        "off" => Some(0u8),
                        "cycle" => Some(2u8),
                        "fast_cycle" => Some(3u8),
                        "blink" => Some(4u8),
                        "fast_blink" => Some(5u8),
                        "custom" => Some(6u8),
                        _ => None,
                    };

                    if v.len() >= 3 {
                        let tst = v[2].parse::<i32>().unwrap_or(-1);
                        if tst >= 0 && tst <= u8::max as i32 {
                            val = tst as u8;
                        }
                    }
                    if let Some(mo) = m {
                        r = Some(VpiCmd::Led(VpiLed::new(mo, val)));
                    }
                }
                r
            }
            "fan" => {
                if v.len() >= 2 && v[1].parse::<i32>().is_ok() {
                    let tst = v[1].parse::<i32>().unwrap();
                    if tst >= 0 && tst <= u8::max as i32 {
                        Some(VpiCmd::Fan(tst as u8))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            "beep" => {
                let mut r: Option<VpiCmd> = None;
                if v.len() >= 2 {
                    let mut count: u8 = 1;
                    let mut b_ms: u32 = 1000;
                    let mut p_ms: u32 = 1000;
                    let f = match v[1].to_lowercase().as_str() {
                        "low" => Some(0u8),
                        "medium" => Some(1u8),
                        "high" => Some(2u8),
                        _ => None,
                    };

                    if v.len() >= 3 {
                        let tst = v[2].parse::<i32>().unwrap_or(count as i32);
                        if tst <= u8::max as i32 {
                            count = tst as u8;
                        }
                    }
                    if v.len() >= 4 {
                        let tst = v[3].parse::<i32>().unwrap_or(b_ms as i32);
                        if tst >= 100 && tst <= 2550 as i32 {
                            b_ms = tst as u32;
                        }
                    }
                    if v.len() >= 5 {
                        let tst = v[3].parse::<i32>().unwrap_or(p_ms as i32);
                        if tst >= 100 && tst <= 25500 as i32 {
                            p_ms = tst as u32;
                        }
                    }
                    if let Some(fe) = f {
                        let t = VpiBuzz::new(fe, count, (b_ms / 100) as u8, (p_ms / 100) as u8);
                        r = Some(VpiCmd::Beep(t));
                    }
                }
                r
            }
            "divisor" => {
                if v.len() >= 2 && v[1].parse::<i32>().is_ok() {
                    let v = v[1].parse::<i32>().unwrap();
                    if v >= 0 && v <= u8::max as i32 {
                        Some(VpiCmd::Divisor(v as u8))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            "pwmfreq" => {
                if v.len() >= 2 && v[1].parse::<i32>().is_ok() {
                    let v = v[1].parse::<i32>().unwrap();
                    if v >= 2i32 && v <= 62500i32 {
                        Some(VpiCmd::PwmFreq(v as u16))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            "timing" => {
                let p: Vec<i32> = v[1..]
                    .into_iter()
                    .map(|e| e.parse::<i32>().unwrap_or(-1))
                    .collect();
                let mut t = VpiTimes::default();
                if p.len() >= 1 && p[0] > 20 && p[0] <= u16::max as i32 {
                    t.short_tm = p[0] as u16
                }
                if p.len() >= 2 && p[1] > 100 && p[1] <= u16::max as i32 {
                    t.space_tm = p[1] as u16
                }
                if p.len() >= 3 && p[2] > 0 && p[2] <= u8::max as i32 {
                    t.hold_tm = p[2] as u8
                }
                if p.len() >= 4 && p[3] > 0 && p[3] <= u8::max as i32 {
                    t.grace_tm = p[3] as u8
                }
                Some(VpiCmd::Timing(t))
            }
            "output" => {
                if v.len() >= 2 && (v[1] == "on" || v[1] == "off") {
                    Some(VpiCmd::Output(v[1] == "on"))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn from_string(s: &String) -> Option<Self> {
        VpiCmd::from_vec(s.split_whitespace().collect())
    }
    pub fn from_cmd_vec(cmd: &str, v: Vec<&str>) -> Option<Self> {
        VpiCmd::from_string(&format!("{} {}", cmd, v.join(" ").as_str()))
    }
}
