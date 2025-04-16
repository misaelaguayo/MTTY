use tokio::sync::mpsc::Sender;
use vte::ansi::Handler;

use crate::commands::Command;

pub struct StateMachine {
    tx: Sender<Command>,
}

impl StateMachine {
    pub fn new(tx: Sender<Command>) -> Self {
        Self { tx }
    }
}

impl Handler for StateMachine {
    fn input(&mut self, c: char) {
        self.tx.try_send(Command::Print(c)).unwrap();
    }

    fn backspace(&mut self) {
        self.tx.try_send(Command::Backspace).unwrap();
    }

    fn newline(&mut self) {
        self.tx.try_send(Command::NewLine).unwrap();
    }

    fn carriage_return(&mut self) {
        self.tx.try_send(Command::CarriageReturn).unwrap();
    }

    fn clear_screen(&mut self, mode: vte::ansi::ClearMode) {
        match mode {
            vte::ansi::ClearMode::All => {
                self.tx.try_send(Command::ClearScreen).unwrap();
            }
            vte::ansi::ClearMode::Above => {
                self.tx.try_send(Command::ClearAbove).unwrap();
            }
            vte::ansi::ClearMode::Below => {
                self.tx.try_send(Command::ClearBelow).unwrap();
            }
            vte::ansi::ClearMode::Saved => {}
        }
    }

    fn terminal_attribute(&mut self, attr: vte::ansi::Attr) {
        if attr == vte::ansi::Attr::Reset {
            self.tx.try_send(Command::ResetStyles).unwrap();
        } else {
            // self.tx.try_send(Command::SGR(vec![])).unwrap();
        }
    }

    fn device_status(&mut self, _: usize) {
        println!("Device status request");
    }

    fn goto(&mut self, line: i32, col: usize) {
        self.tx
            .try_send(Command::MoveCursor(line as i16, col as i16))
            .unwrap();
    }

    fn goto_col(&mut self, col: usize) {
        self.tx
            .try_send(Command::MoveCursor(0, col as i16))
            .unwrap();
    }

    fn goto_line(&mut self, line: i32) {
        self.tx
            .try_send(Command::MoveCursor(line as i16, 0))
            .unwrap();
    }

    fn move_up(&mut self, u: usize) {
        self.tx
            .try_send(Command::MoveCursorVertical(-(u as i16)))
            .unwrap();
    }

    fn move_down(&mut self, d: usize) {
        self.tx
            .try_send(Command::MoveCursorVertical(d as i16))
            .unwrap();
    }

    fn move_forward(&mut self, col: usize) {
        self.tx
            .try_send(Command::MoveCursorHorizontal(col as i16))
            .unwrap();
    }

    fn move_backward(&mut self, col: usize) {
        self.tx
            .try_send(Command::MoveCursorHorizontal(-(col as i16)))
            .unwrap();
    }

    fn clear_line(&mut self, mode: vte::ansi::LineClearMode) {
        match mode {
            vte::ansi::LineClearMode::All => {
                self.tx.try_send(Command::ClearLine).unwrap();
            }
            vte::ansi::LineClearMode::Left => {
                self.tx.try_send(Command::ClearLineBeforeCursor).unwrap();
            }
            vte::ansi::LineClearMode::Right => {
                self.tx.try_send(Command::ClearLineAfterCursor).unwrap();
            }
        }
    }

    fn erase_chars(&mut self, c: usize) {
        self.tx.try_send(Command::ClearCount(c as i16)).unwrap();
    }
}
