pub use snafu::{Snafu, ResultExt};
//, Backtrace, ErrorCompat, ensure};
use std::{path::{PathBuf}};

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

}

pub type Result<T, E = Error> = std::result::Result<T, E>;
