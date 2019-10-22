
/// Crates
extern crate strum;
#[macro_use]
extern crate strum_macros;
#[macro_use]
extern crate log;
extern crate simple_logger;

// Modules declaration
mod config;

// Constant
const VPID_VERSION :&'static str = "0.1.0";

fn main() {
    // Init log
    simple_logger::init().unwrap();
    println!("Vpid daemon init {}",VPID_VERSION);



}
