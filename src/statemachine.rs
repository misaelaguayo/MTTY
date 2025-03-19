use tokio::sync::mpsc::Sender;
use vte::{Params, Perform};

pub struct StateMachine {
    tx: Sender<Vec<u8>>,
}

impl StateMachine {
    pub fn new(tx: Sender<Vec<u8>>) -> Self {
        Self { tx }
    }
}

impl Perform for StateMachine {
    fn print(&mut self, c: char) {
        self.tx.try_send(vec![c as u8]).unwrap();
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            0x0a => {
                self.tx.try_send("\n".as_bytes().to_vec()).unwrap();
            }
            0x0d => {
                self.tx.try_send("\r".as_bytes().to_vec()).unwrap();
            }
            _ => {}
        }
        println!("[execute] {:02x}", byte);
    }

    fn hook(&mut self, params: &Params, intermediates: &[u8], ignore: bool, c: char) {
        println!(
            "[hook] params={:?}, intermediates={:?}, ignore={:?}, char={:?}",
            params, intermediates, ignore, c
        );
    }

    fn put(&mut self, byte: u8) {
        println!("[put] {:02x}", byte);
    }

    fn unhook(&mut self) {
        println!("[unhook]");
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], bell_terminated: bool) {
        println!(
            "[osc_dispatch] params={:?} bell_terminated={}",
            params, bell_terminated
        );
    }

    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], ignore: bool, c: char) {
        println!(
            "[csi_dispatch] params={:#?}, intermediates={:?}, ignore={:?}, char={:?}",
            params, intermediates, ignore, c
        );
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], ignore: bool, byte: u8) {
        println!(
            "[esc_dispatch] intermediates={:?}, ignore={:?}, byte={:02x}",
            intermediates, ignore, byte
        );
    }
}
