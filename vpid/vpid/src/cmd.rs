use vpi::cmd::{VpiCmd,VpiCmdOutput};
use crossbeam_channel::{Sender,Receiver};
use std::os::raw::c_int;
use std::time::Duration;
use crate::error::{Result,ResultExt,OptionExt,CommandParse,CommandSend,CommandRecv,JsonError};
use serde_json::Value;

/// Type to define possible commands managed by Vpi
#[derive(Debug)]
pub enum VpiCommandBody {
    /// Basic command supported by vpi board
    Basic(VpiCmd),
    /// OS signal
    Signal(c_int),
    /// Get Key-value storage
    GetKey(String),
    /// Set Key-value storage
    SetKey(String,String),
    /// Exit
    Exit(bool),
    /// Reload config
    ReloadConfig,
}

const VPI_COMMAND_TIMEOUT : Duration = Duration::from_secs(2);

/// VpiCommand
#[derive(Debug)]
pub struct VpiCommand {
    pub body: VpiCommandBody,
    back_channel: Option<Sender<String>>
}

/// Parse a command from string
pub fn parse_command(s:&String,bc_sender:&Sender<String>) -> Result<VpiCommand> {
     
    // Rusty version
    VpiCommand::from_string(s, Some(bc_sender.clone())).or_else(|| {
            VpiCmd::from_string(s).and_then(|cmd| Some( VpiCommand::new(VpiCommandBody::Basic(cmd),Some(bc_sender.clone())) ) )
    }).context(CommandParse { cmd: s})

    // 1st try internal server commands
    // let mut vpicommand_opt=VpiCommand::from_string(&s,Some(bc_sender.clone()));
    // if vpicommand_opt.is_none() {
    //     // If command is not found try internal commands of vpi lib
    //     if let Some(cmd)=VpiCmd::from_string(s) {
    //         vpicommand_opt=Some( VpiCommand::new(VpiCommandBody::Basic(cmd),Some(bc_sender.clone())));
    //     }
    // }
    // vpicommand_opt
}

pub fn exec_command(s:&String,cmd_sender:&Sender<VpiCommand>,bc_sender:&Sender<String>,bc_recv: &Receiver<String>) -> Result<String> {
    parse_command(s, bc_sender).and_then( |cmd| {
        cmd_sender.send_timeout(cmd, VPI_COMMAND_TIMEOUT).context(CommandSend)?;
        bc_recv.recv_timeout(VPI_COMMAND_TIMEOUT).context(CommandRecv)
    })
}
pub fn exec_command_json(s:&String,cmd_sender:&Sender<VpiCommand>,bc_sender:&Sender<String>,bc_recv: &Receiver<String>) -> Result<serde_json::Value> {
    let val=exec_command(s, cmd_sender, bc_sender, bc_recv)?;
    let jval : Value = serde_json::from_str(val.as_str()).context(JsonError)?;
    Ok(jval)
}

/// Implementation of VpiCommand
impl VpiCommand {

    pub fn new(b: VpiCommandBody,bc: Option<Sender<String>> ) -> Self {
        Self {
            body: b,
            back_channel: bc,
        }
    }
    #[inline]
    pub fn new_nbc(b: VpiCommandBody) -> Self {
        Self::new(b,None)
    }
    /*pub fn nop() -> Self {
        Self::new_nbc(VpiCommandBody::Basic(VpiCmd::Nop))
    }*/
    //pub fn set_back_channel(&mut self,bc:Option<Sender<String>>) {
    //    self.back_channel=bc;
    //}
    //#[inline]
    //pub fn nop() -> Self {
    //    Self::new_nbc(VpiCommandBody::Basic(VpiCmd::Nop))
    //}
    #[inline]
    pub fn from_string(s:&String,bc: Option<Sender<String>>) -> Option<Self> {
        Self::from_vec(s.split_whitespace().collect(),bc)
    }

    pub fn from_vec(v:Vec<&str>,bc: Option<Sender<String>>) -> Option<Self> {
        if v.len() < 1 { return None; }
        match v[0].to_lowercase().as_str() {
            "reload" => {
                Some(Self::new(VpiCommandBody::ReloadConfig,bc))
            },
            "getkey" => {
                if v.len() >= 2 {
                    Some(Self::new(VpiCommandBody::GetKey(v[1].to_string()),bc))
                } else {
                    None
                }

            },
            "setkey" => {
                if v.len() >=3 {
                    let value=v[2..].join(" "); //to_string();
                    Some(Self::new(VpiCommandBody::SetKey(v[1].to_string(),value),bc))
                } else {
                    None
                }
            },
            "exit" => {
                let reboot:bool= v.len() >= 2 && v[1]=="reboot";
                Some(Self::new(VpiCommandBody::Exit(reboot),bc))
            }
            _ => None
        }
    }


    pub fn send_response(&self,response:String) {
        if let Some(bc) = &self.back_channel {
            let _=bc.send(response);
        }
    }
    pub fn send_output(&self,out:&VpiCmdOutput) {
        match out {
            VpiCmdOutput::Json(s) => self.send_response(s.to_string()),
            other => self.send_output(&other.to_json())
        }
    }
    pub fn send_ok(&self) {
        self.send_response(r#"{"result":true}"#.to_string());
    }
    pub fn send_error(&self) {
        self.send_response(r#"{"result":false}"#.to_string());
    }
}
