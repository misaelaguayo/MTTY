use tokio::sync::broadcast::Sender;
use vte::ansi::{
    cursor_icon, Attr, CharsetIndex, ClearMode, CursorShape, CursorStyle, Handler, Hyperlink,
    KeyboardModes, KeyboardModesApplyBehavior, LineClearMode, Mode, ModifyOtherKeys,
    NamedPrivateMode, PrivateMode, Rgb, ScpCharPath, ScpUpdateMode, StandardCharset,
    TabulationClearMode,
};

use crate::commands::{Command, IdentifyTerminalMode, SgrAttribute};

pub struct StateMachine {
    tx: Sender<Command>,
}

impl StateMachine {
    pub fn new(tx: Sender<Command>) -> Self {
        Self { tx }
    }
}

impl Handler for StateMachine {
    fn set_title(&mut self, _: Option<String>) {
        // println!("Set title");
    }

    fn set_cursor_style(&mut self, _: Option<CursorStyle>) {
        // println!("Set cursor style");
    }

    fn set_cursor_shape(&mut self, shape: CursorShape) {
        // println!("Set cursor shape: {:?}", shape);
    }

    fn input(&mut self, c: char) {
        self.tx.send(Command::Print(c)).unwrap();
    }

    fn goto(&mut self, line: i32, col: usize) {
        self.tx
            .send(Command::MoveCursor(line as i16, col as i16))
            .unwrap();
    }

    fn goto_line(&mut self, line: i32) {
        self.tx.send(Command::MoveCursor(line as i16, 0)).unwrap();
    }

    fn goto_col(&mut self, col: usize) {
        self.tx.send(Command::MoveCursor(0, col as i16)).unwrap();
    }

    fn insert_blank(&mut self, count: usize) {
        // println!("Insert blank: {}", count);
    }

    fn move_up(&mut self, u: usize) {
        self.tx
            .send(Command::MoveCursorVertical(-(u as i16)))
            .unwrap();
    }

    fn move_down(&mut self, d: usize) {
        self.tx.send(Command::MoveCursorVertical(d as i16)).unwrap();
    }

    fn identify_terminal(&mut self, intermediate: Option<char>) {
        match intermediate {
            None => {
                self.tx
                    .send(Command::IdentifyTerminal(IdentifyTerminalMode::Primary))
                    .unwrap();
            }
            Some('>') => {
                self.tx
                    .send(Command::IdentifyTerminal(IdentifyTerminalMode::Secondary))
                    .unwrap();
            }
            _ => {
                // println!("Unknown intermediate: {:?}", intermediate);
            }
        }
        // println!("Identify terminal");
    }

    fn device_status(&mut self, arg: usize) {
        match arg {
            5 => {
                self.tx.send(Command::ReportCondition(true)).unwrap();
            }
            6 => {
                self.tx.send(Command::ReportCursorPosition).unwrap();
            }
            _ => {
                // println!("Unknown device status: {}", arg);
            }
        }
    }

    fn move_forward(&mut self, col: usize) {
        self.tx
            .send(Command::MoveCursorHorizontal(col as i16))
            .unwrap();
    }

    fn move_backward(&mut self, col: usize) {
        self.tx
            .send(Command::MoveCursorHorizontal(-(col as i16)))
            .unwrap();
    }

    fn move_down_and_cr(&mut self, _row: usize) {
        // println!("Move down and carriage return");
    }

    fn move_up_and_cr(&mut self, _row: usize) {
        // println!("Move up and carriage return");
    }

    fn put_tab(&mut self, _count: u16) {
        self.tx.send(Command::PutTab).unwrap();
    }

    fn backspace(&mut self) {
        self.tx.send(Command::Backspace).unwrap();
    }

    fn carriage_return(&mut self) {
        self.tx.send(Command::CarriageReturn).unwrap();
    }

    fn linefeed(&mut self) {
        self.tx.send(Command::LineFeed).unwrap();
    }

    fn bell(&mut self) {
        // println!("Bell");
    }

    fn substitute(&mut self) {
        // println!("Substitute");
    }

    fn newline(&mut self) {
        self.tx.send(Command::NewLine).unwrap();
    }

    fn set_horizontal_tabstop(&mut self) {
        // println!("Set horizontal tabstop");
    }

    fn scroll_up(&mut self, _: usize) {
        // println!("Scroll up");
    }

    fn scroll_down(&mut self, _: usize) {
        // println!("Scroll down");
    }

    fn insert_blank_lines(&mut self, _: usize) {
        // println!("Insert blank lines");
    }

    fn delete_lines(&mut self, _: usize) {
        // println!("Delete lines");
    }

    fn erase_chars(&mut self, c: usize) {
        self.tx.send(Command::ClearCount(c as i16)).unwrap();
    }

    fn delete_chars(&mut self, _: usize) {
        // println!("Delete chars");
    }

    fn move_backward_tabs(&mut self, _count: u16) {
        // println!("Move backward tabs");
    }

    fn move_forward_tabs(&mut self, _count: u16) {
        // println!("Move forward tabs");
    }

    fn save_cursor_position(&mut self) {
        self.tx.send(Command::SaveCursor).unwrap();
    }

    fn restore_cursor_position(&mut self) {
        self.tx.send(Command::RestoreCursor).unwrap();
    }

    fn clear_line(&mut self, mode: LineClearMode) {
        match mode {
            LineClearMode::All => {
                self.tx.send(Command::ClearLine).unwrap();
            }
            LineClearMode::Left => {
                self.tx.send(Command::ClearLineBeforeCursor).unwrap();
            }
            LineClearMode::Right => {
                self.tx.send(Command::ClearLineAfterCursor).unwrap();
            }
        }
    }

    fn clear_screen(&mut self, mode: ClearMode) {
        match mode {
            ClearMode::All => {
                self.tx.send(Command::ClearScreen).unwrap();
            }
            ClearMode::Above => {
                self.tx.send(Command::ClearAbove).unwrap();
            }
            ClearMode::Below => {
                self.tx.send(Command::ClearBelow).unwrap();
            }
            ClearMode::Saved => {}
        }
    }

    fn clear_tabs(&mut self, _mode: TabulationClearMode) {
        // println!("Clear tabs");
    }

    fn set_tabs(&mut self, _interval: u16) {
        // println!("Set tabs");
    }

    fn reset_state(&mut self) {
        // println!("Reset state");
    }

    fn reverse_index(&mut self) {
        // println!("Reverse index");
    }

    fn terminal_attribute(&mut self, attr: Attr) {
        if attr == Attr::Reset {
            self.tx.send(Command::ResetStyles).unwrap();
        } else {
            self.tx
                .send(Command::SGR(SgrAttribute::from_vte_attr(attr)))
                .unwrap();
        }
    }

    fn set_mode(&mut self, _mode: Mode) {
        // println!("Set mode");
    }

    fn unset_mode(&mut self, _mode: Mode) {
        // println!("Unset mode");
    }

    fn report_mode(&mut self, _mode: Mode) {
        // println!("Report mode");
    }

    fn set_private_mode(&mut self, mode: PrivateMode) {
        match mode {
            PrivateMode::Named(NamedPrivateMode::ShowCursor) => {
                self.tx.send(Command::ShowCursor).unwrap();
            }
            PrivateMode::Named(NamedPrivateMode::SwapScreenAndSetRestoreCursor) => {
                self.tx
                    .send(Command::SwapScreenAndSetRestoreCursor)
                    .unwrap();
            }
            _ => {
                // println!("Set private mode: {:?}", mode);
            }
        }
    }

    fn unset_private_mode(&mut self, _mode: PrivateMode) {
        // println!("Unset private mode");
    }

    fn report_private_mode(&mut self, _mode: PrivateMode) {
        // println!("Report private mode");
    }

    fn set_scrolling_region(&mut self, _top: usize, _bottom: Option<usize>) {
        // println!("Set scrolling region");
    }

    fn set_keypad_application_mode(&mut self) {
        // println!("Set keypad application mode");
    }

    fn unset_keypad_application_mode(&mut self) {
        // println!("Unset keypad application mode");
    }

    fn set_active_charset(&mut self, _: CharsetIndex) {
        // println!("Set active charset");
    }

    fn configure_charset(&mut self, _: CharsetIndex, _: StandardCharset) {
        // println!("Configure charset");
    }

    fn set_color(&mut self, _: usize, _: Rgb) {
        // println!("Set color");
    }

    fn dynamic_color_sequence(&mut self, _: String, _: usize, _: &str) {
        // println!("Dynamic color sequence");
    }

    fn reset_color(&mut self, _: usize) {
        // println!("Reset color");
    }

    fn clipboard_store(&mut self, _: u8, _: &[u8]) {
        // println!("Clipboard store");
    }

    fn clipboard_load(&mut self, _: u8, _: &str) {
        // println!("Clipboard load");
    }

    fn decaln(&mut self) {
        // println!("DECALN");
    }

    fn push_title(&mut self) {
        // println!("Push title");
    }

    fn pop_title(&mut self) {
        // println!("Pop title");
    }

    fn text_area_size_pixels(&mut self) {
        // println!("Text area size pixels");
    }

    fn text_area_size_chars(&mut self) {
        // println!("Text area size chars");
    }

    fn set_hyperlink(&mut self, _: Option<Hyperlink>) {
        // println!("Set hyperlink");
    }

    fn set_mouse_cursor_icon(&mut self, _: cursor_icon::CursorIcon) {
        // println!("Set mouse cursor icon");
    }

    fn report_keyboard_mode(&mut self) {
        // println!("Report keyboard mode");
    }

    fn push_keyboard_mode(&mut self, _mode: KeyboardModes) {
        // println!("Push keyboard mode");
    }

    fn pop_keyboard_modes(&mut self, _to_pop: u16) {
        // println!("Pop keyboard modes");
    }

    fn set_keyboard_mode(&mut self, _mode: KeyboardModes, _behavior: KeyboardModesApplyBehavior) {
        // println!("Set keyboard mode");
    }

    fn set_modify_other_keys(&mut self, _mode: ModifyOtherKeys) {
        // println!("Set modify other keys");
    }

    fn report_modify_other_keys(&mut self) {
        // println!("Report modify other keys");
    }

    fn set_scp(&mut self, _char_path: ScpCharPath, _update_mode: ScpUpdateMode) {
        // println!("Set SCP");
    }
}
