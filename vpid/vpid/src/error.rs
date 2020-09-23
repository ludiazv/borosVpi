pub use snafu::{Snafu, ResultExt,OptionExt};
//, Backtrace, ErrorCompat, ensure};
use std::{path::{PathBuf}};
use crate::cmd::VpiCommand;
use crossbeam_channel::{RecvTimeoutError,SendTimeoutError};

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum Error {
    #[snafu(display("Couldn't read file {}: {}", filename.display(), source ))]
    ReadConfig { filename: PathBuf, source: std::io::Error },
    #[snafu(display("Could not save config to {}: {}", filename.display(), source ))]
    ParseConfig { filename: PathBuf, source: serde_yaml::Error },
    #[snafu(display("Couldn't communicate with vpi board on {} addr {}: {}",dev.display(),addr ,source ))]
    I2cOpen { dev: PathBuf, addr: u16 , source: vpi::Error },
    #[snafu(display("Couldn't open socket {}: {}", sock.display(), source ))]
    SockBind { sock: PathBuf, source: std::io::Error },
    #[snafu(display("Could not configure vpi board: {}", source ))]
    VpiConfigureError { source: vpi::Error },
    #[snafu(display("Command parse failed, Unkwnon command vpi: {}", cmd ))]
    CommandParse { cmd: String },
    #[snafu(display("Command could not be sent: {}", source ))]
    CommandSend { source: SendTimeoutError<VpiCommand> },
    #[snafu(display("Command response failed or timedout: {}", source ))]
    CommandRecv { source: RecvTimeoutError },
    #[snafu(display("Invalid JSON: {}", source ))]
    JsonError { source: serde_json::error::Error },

}

pub type Result<T, E = Error> = std::result::Result<T, E>;

impl std::convert::From<Error> for rlua::Error {
    fn from(s: Error) -> Self {
        rlua::Error::RuntimeError(format!("vpi-lua error:{}",s))
    }
}   