///! Socket control module
///! The main mechanims to

use crate::error::{Result,ResultExt,SockBind};
use std::thread;
use std::thread::JoinHandle;
use std::os::unix::net::{UnixListener};
use crossbeam_channel::{Sender,bounded};
use std::path::PathBuf;
use crate::cmd::{VpiCommand,exec_command};
use std::io::{Write,BufRead,BufReader};
use std::net::Shutdown;



pub fn run_socket(sock:&PathBuf,command_sender_orig: &Sender<VpiCommand>) -> Result<JoinHandle<()>> {
    let listener = UnixListener::bind(sock).context( SockBind { sock: sock } )?;
    let (bc_sender,bc_receiver) = bounded::<String>(1); // Back channel for the response
    let command_sender= command_sender_orig.clone(); // Clone the sender to move it to sock thread
    // spawn thread for socket server
    let handle = thread::spawn( move | | {
        loop {
            match listener.accept() {
                Ok((mut socket, _)) => {
                    let mut reader = BufReader::new(std::io::Read::by_ref(&mut socket));
                    let mut request = String::new();
                    match reader.read_line(&mut request) {
                        Ok(_len) => {
                            let request_trimmed=request.trim().to_string();
                            if request_trimmed.len() >0 {
                                // Process the request
                                match exec_command(&request_trimmed, &command_sender, &bc_sender, &bc_receiver) {
                                    Ok(val) => { let _=socket.write(val.as_bytes()); },
                                    Err(e)  => { let _=socket.write(format!(r#"{{"result":false,"data":"{}"}}"#,e).as_bytes()); }
                                }

                            } else {
                                let _=socket.write(r#"{"result":false,"data":"command len 0"}"#.as_bytes());
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
