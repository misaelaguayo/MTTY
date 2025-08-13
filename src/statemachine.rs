use tokio::sync::broadcast::Sender;
use vte::ansi::{
    cursor_icon, Attr, CharsetIndex, ClearMode, CursorShape as VteCursorShape, CursorStyle,
    Handler, Hyperlink, KeyboardModes, KeyboardModesApplyBehavior, LineClearMode, Mode,
    ModifyOtherKeys, NamedPrivateMode, PrivateMode, Rgb, ScpCharPath, ScpUpdateMode,
    StandardCharset, TabulationClearMode,
};

use crate::{
    commands::{ClientCommand, IdentifyTerminalMode, SgrAttribute},
    styles::{CursorShape, CursorState},
};

pub struct StateMachine {
    tx: Sender<ClientCommand>,
}

impl StateMachine {
    pub fn new(tx: Sender<ClientCommand>) -> Self {
        Self { tx }
    }
}

impl Handler for StateMachine {
    fn set_title(&mut self, _: Option<String>) {
        log::debug!("Set title");
    }

    fn set_cursor_style(&mut self, s: Option<CursorStyle>) {
        log::debug!("Set cursor style: {:?}", s);
        match s {
            Some(s) => {
                let blinking = s.blinking;
                let shape = match s.shape {
                    VteCursorShape::Block => CursorShape::Block,
                    VteCursorShape::Underline => CursorShape::Underline,
                    VteCursorShape::Beam => CursorShape::Beam,
                    VteCursorShape::HollowBlock => CursorShape::HollowBlock,
                    VteCursorShape::Hidden => CursorShape::Hidden,
                };

                self.tx
                    .send(ClientCommand::SetCursorState(CursorState::new(
                        shape, blinking,
                    )))
                    .unwrap();
            }
            _ => {}
        }
    }

    fn set_cursor_shape(&mut self, shape: VteCursorShape) {
        log::debug!("Set cursor shape: {:?}", shape);
        let cursor_shape = match shape {
            VteCursorShape::Block => CursorShape::Block,
            VteCursorShape::Underline => CursorShape::Underline,
            VteCursorShape::Beam => CursorShape::Beam,
            VteCursorShape::HollowBlock => CursorShape::HollowBlock,
            VteCursorShape::Hidden => CursorShape::Hidden,
        };

        self.tx
            .send(ClientCommand::SetCursorShape(cursor_shape))
            .unwrap();
    }

    fn input(&mut self, c: char) {
        log::debug!("Input character: {}", c);
        self.tx.send(ClientCommand::Print(c)).unwrap();
    }

    fn goto(&mut self, line: i32, col: usize) {
        log::debug!("Goto line: {}, col: {}", line, col);
        self.tx
            .send(ClientCommand::MoveCursor(line as i16, col as i16))
            .unwrap();
    }

    fn goto_line(&mut self, line: i32) {
        log::debug!("Goto line: {}", line);
        self.tx
            .send(ClientCommand::MoveCursor(line as i16, 0))
            .unwrap();
    }

    fn goto_col(&mut self, col: usize) {
        log::debug!("Goto col: {}", col);
        self.tx
            .send(ClientCommand::MoveCursor(0, col as i16))
            .unwrap();
    }

    fn insert_blank(&mut self, count: usize) {
        log::debug!("Insert blank: {}", count);
    }

    fn move_up(&mut self, u: usize) {
        log::debug!("Move up: {}", u);
        self.tx
            .send(ClientCommand::MoveCursorVertical(-(u as i16)))
            .unwrap();
    }

    fn move_down(&mut self, d: usize) {
        log::debug!("Move down: {}", d);
        self.tx
            .send(ClientCommand::MoveCursorVertical(d as i16))
            .unwrap();
    }

    fn identify_terminal(&mut self, intermediate: Option<char>) {
        log::debug!("Identify terminal: {:?}", intermediate);
        match intermediate {
            Some('>') => {
                self.tx
                    .send(ClientCommand::IdentifyTerminal(
                        IdentifyTerminalMode::Secondary,
                    ))
                    .unwrap();
            }
            _ => {
                log::debug!("Unknown intermediate: {:?}", intermediate);
            }
        }
        log::debug!("Identify terminal");
    }

    fn device_status(&mut self, arg: usize) {
        log::debug!("Device status: {}", arg);
        match arg {
            5 => {
                self.tx.send(ClientCommand::ReportCondition(true)).unwrap();
            }
            6 => {
                self.tx.send(ClientCommand::ReportCursorPosition).unwrap();
            }
            _ => {
                log::debug!("Unknown device status: {}", arg);
            }
        }
    }

    fn move_forward(&mut self, col: usize) {
        log::debug!("Move forward: {}", col);
        self.tx
            .send(ClientCommand::MoveCursorHorizontal(col as i16))
            .unwrap();
    }

    fn move_backward(&mut self, col: usize) {
        log::debug!("Move backward: {}", col);
        self.tx
            .send(ClientCommand::MoveCursorHorizontal(-(col as i16)))
            .unwrap();
    }

    fn move_down_and_cr(&mut self, _row: usize) {
        log::debug!("Move down and CR");
        self.tx
            .send(ClientCommand::MoveCursorVerticalWithCarriageReturn(1))
            .unwrap();
    }

    fn move_up_and_cr(&mut self, _row: usize) {
        log::debug!("Move up and CR");
        self.tx
            .send(ClientCommand::MoveCursorVerticalWithCarriageReturn(-1))
            .unwrap();
    }

    fn put_tab(&mut self, _count: u16) {
        log::debug!("Put tab");
        self.tx.send(ClientCommand::PutTab).unwrap();
    }

    fn backspace(&mut self) {
        log::debug!("Backspace");
        self.tx.send(ClientCommand::Backspace).unwrap();
    }

    fn carriage_return(&mut self) {
        log::debug!("Carriage return");
        self.tx.send(ClientCommand::CarriageReturn).unwrap();
    }

    fn linefeed(&mut self) {
        log::debug!("Line feed");
        self.tx.send(ClientCommand::LineFeed).unwrap();
    }

    fn bell(&mut self) {
        log::debug!("Bell");
    }

    fn substitute(&mut self) {
        log::debug!("Substitute");
    }

    fn newline(&mut self) {
        log::debug!("Newline");
        self.tx.send(ClientCommand::NewLine).unwrap();
    }

    fn set_horizontal_tabstop(&mut self) {
        log::debug!("Set horizontal tabstop");
    }

    fn scroll_up(&mut self, _: usize) {
        log::debug!("Scroll up");
    }

    fn scroll_down(&mut self, _: usize) {
        log::debug!("Scroll down");
    }

    fn insert_blank_lines(&mut self, _: usize) {
        log::debug!("Insert blank lines");
    }

    fn delete_lines(&mut self, l: usize) {
        log::debug!("Delete lines: {}", l);
        self.tx.send(ClientCommand::DeleteLines(l as i16)).unwrap();
    }

    fn erase_chars(&mut self, c: usize) {
        log::debug!("Erase chars: {}", c);
        self.tx.send(ClientCommand::ClearCount(c as i16)).unwrap();
    }

    fn delete_chars(&mut self, _: usize) {
        log::debug!("Delete chars");
    }

    fn move_backward_tabs(&mut self, _count: u16) {
        log::debug!("Move backward tabs");
    }

    fn move_forward_tabs(&mut self, _count: u16) {
        log::debug!("Move forward tabs");
    }

    fn save_cursor_position(&mut self) {
        log::debug!("Save cursor position");
        self.tx.send(ClientCommand::SaveCursor).unwrap();
    }

    fn restore_cursor_position(&mut self) {
        log::debug!("Restore cursor position");
        self.tx.send(ClientCommand::RestoreCursor).unwrap();
    }

    fn clear_line(&mut self, mode: LineClearMode) {
        log::debug!("Clear line: {:?}", mode);
        match mode {
            LineClearMode::All => {
                self.tx.send(ClientCommand::ClearLine).unwrap();
            }
            LineClearMode::Left => {
                self.tx.send(ClientCommand::ClearLineBeforeCursor).unwrap();
            }
            LineClearMode::Right => {
                self.tx.send(ClientCommand::ClearLineAfterCursor).unwrap();
            }
        }
    }

    fn clear_screen(&mut self, mode: ClearMode) {
        log::debug!("Clear screen: {:?}", mode);
        match mode {
            ClearMode::All => {
                self.tx.send(ClientCommand::ClearScreen).unwrap();
            }
            ClearMode::Above => {
                self.tx.send(ClientCommand::ClearAbove).unwrap();
            }
            ClearMode::Below => {
                self.tx.send(ClientCommand::ClearBelow).unwrap();
            }
            ClearMode::Saved => {}
        }
    }

    fn clear_tabs(&mut self, _mode: TabulationClearMode) {
        log::debug!("Clear tabs");
    }

    fn set_tabs(&mut self, _interval: u16) {
        log::debug!("Set tabs");
    }

    fn reset_state(&mut self) {
        log::debug!("Reset state");
    }

    fn reverse_index(&mut self) {
        log::debug!("Reverse index");
    }

    fn terminal_attribute(&mut self, attr: Attr) {
        log::debug!("Terminal attribute: {:?}", attr);
        if attr == Attr::Reset {
            self.tx.send(ClientCommand::ResetStyles).unwrap();
        } else {
            self.tx
                .send(ClientCommand::SGR(SgrAttribute::from_vte_attr(attr)))
                .unwrap();
        }
    }

    fn set_mode(&mut self, _mode: Mode) {
        log::debug!("Set mode");
    }

    fn unset_mode(&mut self, _mode: Mode) {
        log::debug!("Unset mode");
    }

    fn report_mode(&mut self, _mode: Mode) {
        log::debug!("Report mode");
    }

    fn set_private_mode(&mut self, mode: PrivateMode) {
        log::debug!("Set private mode: {:?}", mode);
        match mode {
            PrivateMode::Named(NamedPrivateMode::ShowCursor) => {
                self.tx.send(ClientCommand::ShowCursor).unwrap();
            }
            PrivateMode::Named(NamedPrivateMode::SwapScreenAndSetRestoreCursor) => {
                self.tx
                    .send(ClientCommand::SwapScreenAndSetRestoreCursor)
                    .unwrap();
            }
            _ => {
                log::debug!("Set private mode: {:?}", mode);
            }
        }
    }

    fn unset_private_mode(&mut self, mode: PrivateMode) {
        log::debug!("Unset private mode: {:?}", mode);
        match mode {
            PrivateMode::Named(NamedPrivateMode::ShowCursor) => {
                self.tx.send(ClientCommand::HideCursor).unwrap();
            }
            PrivateMode::Named(NamedPrivateMode::SwapScreenAndSetRestoreCursor) => {
                self.tx
                    .send(ClientCommand::SwapScreenAndSetRestoreCursor)
                    .unwrap();
            }
            _ => {
                log::debug!("Unset private mode: {:?}", mode);
            }
        }
    }

    fn report_private_mode(&mut self, _mode: PrivateMode) {
        log::debug!("Report private mode");
    }

    fn set_scrolling_region(&mut self, top: usize, bottom: Option<usize>) {
        log::debug!("Set scrolling region: {} {:?}", top, bottom);
    }

    fn set_keypad_application_mode(&mut self) {
        log::debug!("Set keypad application mode");
    }

    fn unset_keypad_application_mode(&mut self) {
        log::debug!("Unset keypad application mode");
    }

    fn set_active_charset(&mut self, _: CharsetIndex) {
        log::debug!("Set active charset");
    }

    fn configure_charset(&mut self, c: CharsetIndex, typ: StandardCharset) {
        log::debug!("Configure charset: {:?} {:?}", c, typ);
    }

    fn set_color(&mut self, i: usize, rgb: Rgb) {
        log::debug!("Set color: {} {:?}", i, rgb);
        self.tx.send(ClientCommand::SetColor(i, rgb)).unwrap();
    }

    fn dynamic_color_sequence(&mut self, _: String, _: usize, _: &str) {
        log::debug!("Dynamic color sequence");
    }

    fn reset_color(&mut self, i: usize) {
        log::debug!("Reset color: {}", i);
        self.tx.send(ClientCommand::ResetColor(i)).unwrap();
    }

    fn clipboard_store(&mut self, _: u8, _: &[u8]) {
        log::debug!("Clipboard store");
    }

    fn clipboard_load(&mut self, _: u8, _: &str) {
        log::debug!("Clipboard load");
    }

    fn decaln(&mut self) {
        log::debug!("DECALN");
    }

    fn push_title(&mut self) {
        log::debug!("Push title");
    }

    fn pop_title(&mut self) {
        log::debug!("Pop title");
    }

    fn text_area_size_pixels(&mut self) {
        log::debug!("Text area size pixels");
    }

    fn text_area_size_chars(&mut self) {
        log::debug!("Text area size chars");
    }

    fn set_hyperlink(&mut self, _: Option<Hyperlink>) {
        log::debug!("Set hyperlink");
    }

    fn set_mouse_cursor_icon(&mut self, _: cursor_icon::CursorIcon) {
        log::debug!("Set mouse cursor icon");
    }

    fn report_keyboard_mode(&mut self) {
        log::debug!("Report keyboard mode");
    }

    fn push_keyboard_mode(&mut self, _mode: KeyboardModes) {
        log::debug!("Push keyboard mode");
    }

    fn pop_keyboard_modes(&mut self, _to_pop: u16) {
        log::debug!("Pop keyboard modes");
    }

    fn set_keyboard_mode(&mut self, _mode: KeyboardModes, _behavior: KeyboardModesApplyBehavior) {
        log::debug!("Set keyboard mode");
    }

    fn set_modify_other_keys(&mut self, _mode: ModifyOtherKeys) {
        log::debug!("Set modify other keys");
    }

    fn report_modify_other_keys(&mut self) {
        log::debug!("Report modify other keys");
    }

    fn set_scp(&mut self, _char_path: ScpCharPath, _update_mode: ScpUpdateMode) {
        log::debug!("Set SCP");
    }
}
