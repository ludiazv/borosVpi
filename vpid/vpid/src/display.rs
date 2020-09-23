use embedded_graphics::pixelcolor::BinaryColor;
use crossbeam_channel::{Sender,Receiver,bounded};


type DisplayColor = BinaryColor; 

pub enum VpiDisplayCommand {
    Clear,
    Sleep,
}

pub struct VpiDisplay {
    receiver: Receiver<VpiDisplayCommand>,
    
}