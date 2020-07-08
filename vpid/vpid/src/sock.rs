///! Socket control mdodule
/// 

use crate::error::{Result,ResultExt,SockBind};
use std::thread;
use std::thread::JoinHandle;
use std::os::unix::net::{UnixListener};
use crossbeam_channel::{bounded,Sender};
use std::path::PathBuf;
use crate::{VpiCommand,VpiCommandBody};
use std::io::{Write,BufRead,BufReader};
use std::net::Shutdown;
use std::time::Duration;
use vpi::cmd::VpiCmd;

pub fn run_socket(sock:&PathBuf,command_sender_orig: &Sender<VpiCommand>) -> Result<JoinHandle<()>> {
    let listener = UnixListener::bind(sock).context( SockBind { sock: sock } )?;
    let (bc_sender,bc_receiver) = bounded::<String>(1);
    let command_sender= command_sender_orig.clone();
    let handle = thread::spawn( move | | {
        loop {
            match listener.accept() {
                Ok((mut socket, _)) => {
                    let mut reader = BufReader::new(std::io::Read::by_ref(&mut socket));
                    let mut request = String::new();
                    match reader.read_line(&mut request) {
                        Ok(_len) => {
                            let request_trimmed=request.trim();
                            if request_trimmed.len() >0 {
                                if let Some(cmd) = VpiCmd::from_string(request_trimmed.to_string()) {
                                    if command_sender.send(VpiCommand::new(VpiCommandBody::Basic(cmd),Some(bc_sender.clone()))).is_ok() {
                                        match bc_receiver.recv_timeout(Duration::from_secs(2)) {
                                            Ok(val) => { let _=socket.write(val.as_bytes()); },
                                            Err(_) =>  { let _=socket.write(r#"{"result":"error","data":"Internal error - command failed or timeout"}"#.as_bytes());}
                                        };
                                    } else {
                                        let _=socket.write(r#"{"result":"error","data":"Internal error - can't deliver command"}"#.as_bytes());
                                    }
                                } else {
                                    let _=socket.write(r#"{"result":"error","data":"Invalid command"}"#.as_bytes());
                                }
                            } else {
                                let _=socket.write(r#"{"result":"error","data":"command len 0"}"#.as_bytes());
                            }
                        },
                        Err(e) => error!("Socket server read failed:{}",e)
                    }
                    let _=socket.flush();
                    let _=socket.shutdown(Shutdown::Both);  
                },
                Err(e) => error!("accept function failed: {:?}", e),
            }
        }
    });
    Ok(handle)
}

pub fn close_socket(socket:&PathBuf) {
    let _=std::fs::remove_file(socket);
}
