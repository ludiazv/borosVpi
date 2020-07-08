use vpi::cmd::{VpiCmd,VpiCmdOutput};
use crossbeam_channel::{Sender};
use std::os::raw::c_int;

#[derive(Debug)]
pub enum VpiCommandBody {
    Basic(VpiCmd),
    Signal(c_int),
    ReloadConfig,
}

#[derive(Debug)]
pub struct VpiCommand {
    pub body: VpiCommandBody,
    back_channel: Option<Sender<String>>
}

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
        self.send_response(r#"{"result":"ok"}"#.to_string());
    }
    /*pub fn send_error(&self) {
        self.send_response(r#"{"result":"error"}"#.to_string());
    }*/
}
