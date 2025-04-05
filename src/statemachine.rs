use tokio::sync::mpsc::Sender;
use vte::{Params, Perform};

use crate::commands::Command;

pub struct StateMachine {
    tx: Sender<Command>,
}

impl StateMachine {
    pub fn new(tx: Sender<Command>) -> Self {
        Self { tx }
    }
}

impl Perform for StateMachine {
    fn print(&mut self, c: char) {
        self.tx.try_send(Command::Print(c)).unwrap();
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            0x08 => {
                self.tx.try_send(Command::Backspace).unwrap();
            }
            0x0a => {
                self.tx.try_send(Command::NewLine).unwrap();
            }
            0x0d => {
                self.tx.try_send(Command::CarriageReturn).unwrap();
            }
            _ => {
                println!("[execute] {:02x}", byte);
            }
        }
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
        match c {
            'h' => match params.len() {
                1049 => {
                    self.tx
                        .try_send(Command::AlternateScreenBuffer(true))
                        .unwrap();
                }
                2004 => {
                    self.tx.try_send(Command::BrackPasteMode(true)).unwrap();
                }
                _ => {}
            },
            'l' => match params.len() {
                1049 => {
                    self.tx
                        .try_send(Command::AlternateScreenBuffer(false))
                        .unwrap();
                }
                2004 => {
                    self.tx.try_send(Command::BrackPasteMode(false)).unwrap();
                }
                _ => {}
            },
            'J' => {
                if let Some(clear_type) = params.iter().next().map(|param| param[0]) {
                    match clear_type {
                        0 => {
                            self.tx.try_send(Command::ClearBelow).unwrap();
                        }
                        1 => {
                            self.tx.try_send(Command::ClearAbove).unwrap();
                        }
                        2 => {
                            self.tx.try_send(Command::ClearScreen).unwrap();
                        }
                        _ => {}
                    }
                }
            }
            'm' => {
                if intermediates.is_empty() {
                    for param in params.iter() {
                        match param[0] {
                            0 => {
                                self.tx.try_send(Command::ResetStyles).unwrap();
                            }
                            _ => {}
                        }
                    }
                }
            }
            'H' => {
                if params.len() == 0 {
                    self.tx.try_send(Command::MoveCursor(0, 0)).unwrap();
                }

                params.iter().for_each(|p| match p.len() {
                    1 => {
                        self.tx
                            .try_send(Command::MoveCursor(p[0] as i16, 0))
                            .unwrap();
                    }
                    2 => {
                        self.tx
                            .try_send(Command::MoveCursor(p[0] as i16, p[1] as i16))
                            .unwrap();
                    }
                    _ => {}
                })
            }
            'A' => {
                self.tx
                    .try_send(Command::MoveCursorVertical(params.len() as i16))
                    .unwrap();
            }
            'B' => {
                self.tx
                    .try_send(Command::MoveCursorVertical(params.len() as i16 * -1))
                    .unwrap();
            }
            'C' => {
                self.tx
                    .try_send(Command::MoveCursorHorizontal(params.len() as i16))
                    .unwrap();
            }
            'D' => {
                self.tx
                    .try_send(Command::MoveCursorHorizontal(params.len() as i16 * -1))
                    .unwrap();
            }
            'E' => {
                self.tx
                    .try_send(Command::MoveCursorLineVertical(params.len() as i16))
                    .unwrap();
            }
            'F' => {
                self.tx
                    .try_send(Command::MoveCursorLineVertical(params.len() as i16 * -1))
                    .unwrap();
            }
            'G' => {
                self.tx
                    .try_send(Command::MoveCursorAbsoluteHorizontal(params.len() as i16))
                    .unwrap();
            }
            'K' => {
                if let Some(clear_type) = params.iter().next().map(|param| param[0]) {
                    match clear_type {
                        0 => {
                            self.tx.try_send(Command::ClearLineAfterCursor).unwrap();
                        }
                        1 => {
                            self.tx.try_send(Command::ClearLineBeforeCursor).unwrap();
                        }
                        2 => {
                            self.tx.try_send(Command::ClearLine).unwrap();
                        }
                        _ => {}
                    }
                }
            }
            'X' => {
                if let Some(count) = params.iter().next().map(|param| param[0]) {
                    self.tx.try_send(Command::ClearCount(count as i16)).unwrap();
                }
            }
            _ => {
                println!(
                    "[csi_dispatch] params={:#?}, intermediates={:?}, ignore={:?}, char={:?}",
                    params, intermediates, ignore, c
                );
            }
        }
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], ignore: bool, byte: u8) {
        println!(
            "[esc_dispatch] intermediates={:?}, ignore={:?}, byte={:02x}",
            intermediates, ignore, byte
        );
    }
}
