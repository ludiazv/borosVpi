///! Lua & Scripting engine
/// Handle rules and run scripts 
/// 
use vpi::{VpiStatus,VpiStats};
use crate::config::{VpiConfig,VpiRule,VpiRuleType};
use crate::error::{Result,ResultExt,JsonError};
use crate::cmd::{VpiCommand,exec_command_json};
use rlua::{Lua, UserDataMethods,UserData};
use serde_json::Value;
use std::process::{Command,Child};
use crossbeam_channel::{Sender,bounded};
use std::thread::{JoinHandle};
use std::time::{Instant,Duration};
use std::thread;
use std::sync::atomic::{AtomicBool,Ordering};
use std::sync::{Arc,Weak};
// For force killing of pthreads
//use std::os::unix::thread::JoinHandleExt;
//use std::libc::pthread_cancel;


/// Info of Child process 
struct ChildInfo {
    /// Sequence ID
    id: u32,
    /// Child information
    child: Child,
    /// Name of the childs
    name: String,
    /// Time child process started
    started: Instant,
    /// Timeout 0 = no timeout
    timeout: u32,
}

/// Info of Lua script
struct LuaInfo {
    id: u32,
    handle:JoinHandle<()>,
    name:String,
    started: Instant,
    timeout: u32,
    control: Weak<AtomicBool>,
}

/// Engine object
pub struct Engine<'a> {
    pub cfg: &'a VpiConfig,
    command_sender: &'a Sender<VpiCommand>,
    childs: Vec<ChildInfo>,
    lchilds: Vec<LuaInfo>,
    seq: u32,
}


/// Serde -> Lua
fn json_to_lua<'lua>(js:&serde_json::Value,ctx: &rlua::Context<'lua>) -> rlua::Result<rlua::Value<'lua>> {
    match js {
        Value::Null => Ok(rlua::Value::Nil),
        Value::Bool(b) => Ok(rlua::Value::Boolean(*b)),
        Value::Number(n) => {
            if n.is_i64() {
                Ok(rlua::Value::Integer(n.as_i64().unwrap()))
            } else if n.is_f64() {
                Ok(rlua::Value::Number(n.as_f64().unwrap()))
            } else { Ok(rlua::Value::Nil) }
        },
        Value::String(s) => {
            let ss=ctx.create_string(s)?;
            Ok(rlua::Value::String(ss))
        },
        Value::Array(vc) => {
            let a=ctx.create_table()?;
            for (pos,e) in vc.iter().enumerate() {
                let v=json_to_lua(&e,ctx)?;
                a.set(pos,v)?;
            }
            Ok(rlua::Value::Table(a))
        },
        Value::Object(ob) => {
            let o=ctx.create_table()?;
            for (k,e) in ob {
                let v=json_to_lua(&e,ctx)?;
                o.set(k.as_str(),v)?;
            }
            Ok(rlua::Value::Table(o))
        }
    }
}

/// Vpi object inside lua
struct LuaVpi {
    sender: Sender<VpiCommand>,
}

impl LuaVpi {
    /// Create a new instance of vpi object inside lua
    pub fn new(snd: &Sender<VpiCommand>) -> Self {
        Self {
            sender: snd.clone()
        }
    }
    ///
    fn exec(&self,cmd:&String) -> Result<Value> {
        let (bc_sender,bc_receiver) = bounded::<String>(1); // Back channel for the response
        exec_command_json(cmd, &self.sender, &bc_sender, &bc_receiver) // Return JSON 
    }
} //LuaVPi

impl UserData for LuaVpi {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {

        methods.add_method("cmd", |ctx, lvpi, cmd:String | {
            let val =lvpi.exec(&cmd)?;
            //Ok(LuaJson(&val))
            json_to_lua(&val, &ctx)
        });

        methods.add_method("sleep", |_,_lvpi,ms:u32| {
            thread::sleep(Duration::from_millis(ms as u64)); 
            Ok(())
        });

        methods.add_method("info", |_,_lvpi,msg:String| {
            info!("Lua>>{}",msg); 
            Ok(())
        });
        methods.add_method("warn", |_,_lvpi,msg:String| {
            warn!("Lua>>{}",msg); 
            Ok(())
        });

        methods.add_method("parse_json", |ctx,_lvpi,js:String| {
            let js=serde_json::from_str(js.as_str()).context(JsonError)?;
            json_to_lua(&js, &ctx)
        });
    }
}

/// Engine implementation
impl<'a> Engine<'a> {
    /// Create a new engine object
    pub fn new(cfg:&'a VpiConfig,command_sender:&'a Sender<VpiCommand>) -> Self {
        Engine {
            cfg: cfg,
            command_sender: command_sender,
            childs: vec!(),
            lchilds: vec!(),
            seq: 0,
        }
    }
    /// Fills lua context with exposed variables of current status and stats.
    fn add_lua_variables(&self, stat: &VpiStatus, sts: &VpiStats,ctx: & rlua::Context) {
        let globs=ctx.globals();
        
        let _=globs.set("has_click",stat.has_click);
        let _=globs.set("has_rpm",stat.has_rpm);
        let _=globs.set("has_irq",stat.has_irq);
        let _=globs.set("long",stat.pwr_long);
        let _=globs.set("short",stat.pwr_short);
        let _=globs.set("aux_long",stat.aux_long);
        let _=globs.set("aux_short",stat.aux_short);
        let _=globs.set("is_running",stat.is_running);
        let _=globs.set("out_value",stat.out_value);
        let _=globs.set("rpm",stat.rpm);
        let _=globs.set("error_count",stat.error_count);
       
        let _=globs.set("vpi_crc_errors",sts.crc_errors);
        let _=globs.set("vpi_recovers",sts.recovers);
        let _=globs.set("vpi_status_checks",sts.status_checks);
        let _=globs.set("vpi_retries",sts.retries);
        
    }
 
    /// Run a lua script in a independent thread
    fn run_lua(&mut self,name:&str,script:&str,timeout: u32) {
        
        let cancel_control=Arc::new(AtomicBool::new(false));    // Strong reference with flag for cancellation will be moved to lua thread
        let downgraded_cancel=Arc::downgrade(&cancel_control); // Weak version to control the flag for main thread
        let scr=String::from(script); // Copy script to be used in the thread
        let names=String::from(name); // Copy to be used in thread 
        let luavpi= LuaVpi::new(self.command_sender); // Lua vpi object for this execution
        
        
        let handle= thread::spawn(move || {
            let lua = Lua::new();
            let mut return_value:i32=-255;
    
            lua.context(|lua_ctx| {    
                let globs=lua_ctx.globals();
                let _= globs.set("vpi",luavpi); // move vpi object to the thread
    
                let cancel_fnr=lua_ctx.create_function( move |_, () | {
                    Ok((*cancel_control).load(Ordering::Relaxed))
                });
                if let Ok(cancel_fn) = cancel_fnr {
                    let _=globs.set("vpi_test_cancel",cancel_fn);
                }
                match lua_ctx.load(scr.as_str()).eval::<i32>() {
                    Ok(n) => return_value=n,
                    Err(e) => warn!("Lua script [{}] failed: {}",names,e ),
                }
                info!("Lua script [{}] finished return value:{}",names,return_value);
            });
           
        });

        let job= LuaInfo {
            id: self.seq,
            name: name.to_string(),
            started: std::time::Instant::now(), 
            timeout: timeout,
            control: downgraded_cancel,
            handle: handle,
        };
        self.seq+=1;
        self.lchilds.push(job);

    }
    /// Run a shell command
    fn run_shell(&mut self,name:&str,cmd:&str,args:&str,asyncr: bool,timeout: u32) {
        if cmd == "" { return; }
        let cmds : Vec<&str>= cmd.split_whitespace().collect();
        let mut c=Command::new(cmds[0]);
        if cmds.len() >= 2 { c.args(cmds[1..].iter()); }
        if args != "" { c.arg(args) ;}
        trace!("Shell command:'{:?}'",c);
        info!("Starting shell process id: {}-[{}] async:{} , timeout:{}",self.seq,name,asyncr,timeout);
        if asyncr {
            let child_result=c.spawn();
            if let Ok(child) = child_result {
                self.childs.push(ChildInfo { id: self.seq, child: child, name: name.to_string(), started: std::time::Instant::now(), timeout: timeout});
            } else {
                error!("Shell process {} failed [{}]",name,child_result.unwrap_err());
            }
        } else {
            let out_result=c.output();
            if let Ok(out) = out_result {
                let code=out.status.code().unwrap_or(255);
                if code ==0 {
                    info!("Shell process finished succesfully {}-[{}] exit code:{}",self.seq, name, code);
                   } else {
                    warn!("Shell process finished with error {}-[{}] exit code:{}",self.seq, name, code);
                   }
            } else {
                error!("Shell process {} failed [{}]",name,out_result.unwrap_err());
            }
        }
        self.seq+=1;
    }
    /// Run Rule
    fn run_rule(&mut self,rule :&VpiRule)  {
        match rule.kind {
                VpiRuleType::Reboot   => self.run_shell(rule.name.as_str(),self.cfg.reboot_command.as_str(),"",false,0),
                VpiRuleType::Shutdown => self.run_shell(rule.name.as_str(),self.cfg.shutdown_command.as_str(),"",false,0),
                VpiRuleType::Lua      => {
                    if let Some(script) = &rule.script {
                        self.run_lua(rule.name.as_str(), script, rule.timeout);
                    }
                },
                VpiRuleType::Shell    => {
                    if let Some(script) = &rule.script {
                        self.run_shell(rule.name.as_str(),self.cfg.shell.as_str(),script,rule.asyncr,rule.timeout);
                    }
                },
                VpiRuleType::Nop      => { }
        }
    }
    /// Starts all configured mini services
    pub fn start_miniservices(&mut self) -> Result<u32> {
        let mut done = 0;
        let total= self.cfg.services.len();
        info!("Starting miniservices total:{}",total);
        for ser in self.cfg.services.iter() {
            info!("Launching lua mini service [{}] {} of {}",ser.name,done,total);
            self.run_lua(ser.name.as_str(), ser.script.as_str(), 0);
            done+=1;
        }
        Ok(done)
    }
    /// check and run rules
    pub fn run_rules(&mut self,stat: &VpiStatus, sts: &VpiStats) -> Result<()> {

        if self.cfg.rules.len() == 0 { 
            trace!("No rules to execute");
            return Ok(()); 
        } // Ignore if rules empty
    
        let lua = Lua::new();
        lua.context(|lua_ctx| {
            self.add_lua_variables(stat,sts,&lua_ctx); // Fill the context
            for rule in &self.cfg.rules {
                match lua_ctx.load(rule.when.as_str()).eval::<bool>() {
                    Ok(res) => {
                        if res {
                            info!("Rule [{}] matched!",rule.name);
                            self.run_rule(&rule);
                            break;
                        }
                    },
                    Err(e) => warn!("can't evaluate rule [{}]:{}",rule.when,e)
                }
            }

        });
        Ok(())
    }
    /// Test childs
    pub fn test_childs(&mut self,force_kill:bool) {
        for pos in 0..self.childs.len() {
            let e=&mut self.childs[pos];
            if let Ok(Some(status)) = e.child.try_wait() {
                let code = status.code().unwrap_or(255);
                if code ==0 {
                    info!("Shell process finished succesfully [{}] exit code:{}", e.name, code);
                   } else {
                    warn!("Shell process finished with error [{}] exit code:{}", e.name, code);
                   }
                   self.childs.remove(pos);
                  continue;
            }
            if force_kill || (e.timeout>0 && e.started.elapsed() > Duration::from_secs(e.timeout as u64)) {
                warn!("Process {}-[{}] timed out. Killing!",e.id,e.name);
                let _=e.child.kill();
                let _=e.child.wait();
                self.childs.remove(pos);
            }

        }
    }
    /// Test lua childs
    pub fn test_lua_childs(&mut self,force_kill:bool) {
        let mut to_join=vec!();
        for (pos,e) in self.lchilds.iter().enumerate() {
            match e.control.upgrade() {
                None => {
                    to_join.push(pos);
                },
                Some(cancel) => { // Still runing
                    let has_timeout=e.timeout>0 && e.started.elapsed() > Duration::from_secs(e.timeout as u64);
                    if !(*cancel).load(Ordering::Relaxed) && (force_kill || has_timeout) {
                        (*cancel).store(true,Ordering::Relaxed);
                        if has_timeout {
                            warn!("Timeout signal to lua script {}-[{}] sent. timeout:{} s",e.id,e.name,e.timeout);
                        } else{
                            info!("Forced end signal sent to lua script {}-[{}]",e.id,e.name);
                        } 
                        
                    }
                }
            }
        }
        for i in to_join.iter() {
            let _= self.lchilds.remove(*i).handle.join();
        }
    }

}

/// Drop Implementation -> Cancel all threads and processes.
impl<'a> Drop for Engine<'a> {
    fn drop(&mut self) {
        info!("Stopping rule and process engine");
        self.test_childs(true);     // Force stop async scripts
        self.test_lua_childs(true); // Force stop lua childs
        if self.lchilds.len()>0  {  // If still lua childs pending give a retry
            thread::sleep(std::time::Duration::from_secs(5)); // Wait some time
            self.test_lua_childs(true); // Try again to exit lua threads in controlled way
           /* if self.lchilds.len() > 0 {  // If still pending try to cancell via unsafe THIS IS HACK
                for lc in self.lchilds {
                    unsafe {
                        pthread_cancel(lc.handle.as_pthread_t());
                    }
                }
            } */
            self.lchilds.clear();
        }
    }
}



