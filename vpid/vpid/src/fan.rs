//! Fan control module

use serde::Deserialize;
use serde_piecewise_default::DeserializePiecewiseDefault;
use std::fs::read_to_string;
use std::path::Path;

#[derive(Copy,Clone,Eq,PartialEq,Debug,Deserialize)]
pub enum VpiFanMode {
    Off,
    On,
    Custom,
    Pi,
    Linear
}

//#[derive(DeserializePiecewiseDefault,PartialEq,Eq,Debug)]
#[derive(DeserializePiecewiseDefault,Debug,Clone,PartialEq)]
pub struct VpiFanConfig {
     pins: u8,
     divisor: u8,
     sample: u32,
     pwm_freq: Option<u16>,
     thermal_path: String,
     mode: VpiFanMode,
     linear_max_temp: i32,
     linear_min_temp: i32,
     pi_desired_temp: i32,
     custom_value: u8,
     kp: f32,
     ki: f32,
     //#[serde(skip)]
     pi_sum: i64,
}

impl Default for VpiFanConfig {
    fn default() -> Self {
        VpiFanConfig {
            pins: 2,
            sample: 5,
            divisor: 2,
            pwm_freq: None,
            thermal_path: String::from("/sys/class/thermal/thermal_zone0/temp"),
            mode: VpiFanMode::Off,
            linear_max_temp: 70*1000,
            linear_min_temp: 35*1000,
            pi_desired_temp: 465*100,
            custom_value:    0,
            kp: 20.0/1000.0,
            ki: 20.0/100000.0,
            pi_sum: 0
        }

    }
}

impl VpiFanConfig {
    /// Get recommended pwm frequency
    pub fn get_pwmfreq(&self) -> u16 {
        if let Some(p) = self.pwm_freq {
            if p >= 1 && p<=62500 { return p; }
            warn!("Invalid frequency {} using pin defaults",p); 
        } 
        // frequency based on pins    
        if self.pins == 4 {
                25500u16
        } else {
                250u16
        }
    }
    /// Get RPM divisor
    pub fn get_divisor(&self) -> u8 {
        if self.divisor==0 {
            warn!("Invalid divisor 0 using 2 as deault"); 
            2u8
        } else {
            self.divisor
        }
    }
    /// Force fan regulation to fixed value.
    pub fn set_fan(&mut self,val:u8) {
        self.mode = VpiFanMode::Custom;
        self.custom_value=val;
    }
    /// Regulate fan based on selected mode
    pub fn regulate(&mut self) -> u8 {

        match self.mode {
            VpiFanMode::Off => 0,
            VpiFanMode::On  => 255,
            VpiFanMode::Custom => self.custom_value,
            VpiFanMode::Linear => {
                let t=self.get_temp();
                if t != -1 {
                    let mut scaled_t= 256 * (t-self.linear_min_temp);
                    scaled_t/= self.linear_max_temp - self.linear_min_temp;
                    match scaled_t {
                        n if n>=0 && n<=255 => scaled_t as u8,
                        n if n < 0 => 0u8,
                        n if n > 255 => 255u8,
                        _ => 0u8
                    }
                 } else { 0u8 }
            },
            VpiFanMode::Pi => {
                let t=self.get_temp();
                if t!= -1 {
                    let diff= t - self.pi_desired_temp;
                    self.pi_sum+= diff as i64;
                    let pdiff= (diff as f32) * self.kp;
                    let idiff= (self.pi_sum as f32) * self.ki;
                    match pdiff + idiff {
                        n if n>0.0 && n<255.0 =>  n as u8,
                        n if n<=0.0  => { self.pi_sum=0; 0u8 },
                        n if n>255.0 => 255u8,
                        _ => 0u8
                    }

                }else { 0u8 }
            }
        }
    }
    /// Get temperature 
    pub fn get_temp(&self) -> i32 {
        
        let s=read_to_string(Path::new(&self.thermal_path));
        if let Ok(ss) = s {
            let t=ss.trim().parse::<i32>().unwrap_or(-1);
            if t ==-1 {
                error!("Could not parse temperature information '{}'",ss);
            }
            trace!("Got temp from SOC {}",t);
            t   
        }
        else {
            error!("Could not read temperature in {} -> [{}]",self.thermal_path,s.unwrap_err());
            -1
        }
    }

}