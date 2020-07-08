///! Lua & Scripting engine
/// Handle rules and run processes.
/// 
use vpi::{VpiStatus,VpiStats};
use crate::config::{VpiConfig,VpiRule,VpiRuleType};
use crate::error::{Result};
use crate::cmd::{VpiCommand,VpiCommandBody};
use rlua::{Function, Lua, MetaMethod,UserData};
use std::process::{Command,Child};
use crossbeam_channel::{Sender};
use std::thread::{JoinHandle};
use std::time::{Instant,Duration};
use std::sync::atomic::{AtomicBool,Ordering};
use std::sync::{Arc,Weak,Mutex};
use std::thread;



struct ChildInfo {
    id: u32,
    child: Child,
    name: String,
    started: Instant,
    timeout: u32,
}

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
    last_status: Option<VpiStatus>,
    last_stats: Option<VpiStats>,

}

impl<'a> Engine<'a> {
    /// Create a new engine object
    pub fn new(cfg:&'a VpiConfig,command_sender:&'a Sender<VpiCommand>) -> Self {
        Engine {
            cfg: cfg,
            command_sender: command_sender,
            childs: vec!(),
            lchilds: vec!(),
            seq: 0,
            last_status: None,
            last_stats: None,
        }
    }
    /// Fills lua context with exposed variables and helper functions
    fn fill_lua_context(&self, ctx : &rlua::Context, simple : bool) {
        if self.last_status.is_none() { return; }
        let ls=self.last_status.unwrap();
        let globs=ctx.globals();
        let _=globs.set("has_click",ls.has_click);
        let _=globs.set("has_rpm",ls.has_rpm);
        let _=globs.set("has_irq",ls.has_irq);
        let _=globs.set("long",ls.pwr_long);
        let _=globs.set("short",ls.pwr_short);
        let _=globs.set("aux_long",ls.aux_long);
        let _=globs.set("aux_short",ls.aux_short);
        let _=globs.set("is_running",ls.is_running);
        let _=globs.set("out_value",ls.out_value);
        let _=globs.set("rpm",ls.rpm);
        
        if !simple { // Fill context with helpers 

        }

        //pub has_error:              bool,


        //pub out_value:              bool,
        //pub integrity:              bool,

        //pub error_count:            i32,
 
    }

    fn run_lua(&mut self,name:&str,script:&str,timeout: u32) {
        
        let cancel_control=Arc::new(AtomicBool::new(false));
        let downgraded_cancel=Arc::downgrade(&cancel_control);
        let scr=String::from(script);
        let names=String::from(name);
        
        let handle= thread::spawn(move || {
            let lua = Lua::new();
            let mut return_value:i32=-255;
    
            lua.context(|lua_ctx| {
                let globs=lua_ctx.globals();

                let cancel_fnr=lua_ctx.create_function( move |_, () | {
                    Ok((*cancel_control).load(Ordering::Relaxed))
                });
                if let Ok(cancel_fn) = cancel_fnr {
                    let _=globs.set("test_cancel",cancel_fn);
                }
                match lua_ctx.load(scr.as_str()).eval::<i32>() {
                    Ok(n) => return_value=n,
                    Err(e) => warn!("Lua script [{}] failed: {}",names,e ),
                }
                info!("Lua script [{}] finished return value:{}",names,return_value);
            });
            //return_value
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

    /// check and run rules
    pub fn run_rules(&mut self,stat: &VpiStatus, sts: &VpiStats) -> Result<()> {

        self.last_status=Some(*stat);
        self.last_stats=Some(*sts);
        if self.cfg.rules.len() == 0 { return Ok(()); } // Ignore if rules empty
    
        let lua = Lua::new();
        lua.context(|lua_ctx| {
            self.fill_lua_context(&lua_ctx,true); // Fill the context

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
        for pos in 0..self.lchilds.len() {
            /*let handle=self.lchilds[pos].handle;
            let control=&self.lchilds[pos].control;
            let name=&self.lchilds[pos].name;
            let timeout=self.lchilds[pos].timeout;
            let started=self.lchilds[pos].started; */
            let e=self.lchilds.remove(pos); // mov out element
            match e.control.upgrade() {
                None => {
                    e.handle.join().unwrap_or(());
                    //self.lchilds.remove(pos);
                   
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
                    // Do not remove from the list, in the next runn will be tried.
                    self.lchilds.push(e)
                }
            }
        }
    }

}

impl<'a> Drop for Engine<'a> {
    fn drop(&mut self) {
        info!("Stopping rule and process engine");
        self.test_childs(true);
        self.test_lua_childs(true);
        if self.lchilds.len()>0  {
            thread::sleep(std::time::Duration::from_secs(5));
            self.test_lua_childs(true);
        }
    }
}



