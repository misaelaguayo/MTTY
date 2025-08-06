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
        log::info!("Set title");
    }

    fn set_cursor_style(&mut self, s: Option<CursorStyle>) {
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
        self.tx.send(ClientCommand::Print(c)).unwrap();
    }

    fn goto(&mut self, line: i32, col: usize) {
        self.tx
            .send(ClientCommand::MoveCursor(line as i16, col as i16))
            .unwrap();
    }

    fn goto_line(&mut self, line: i32) {
        self.tx
            .send(ClientCommand::MoveCursor(line as i16, 0))
            .unwrap();
    }

    fn goto_col(&mut self, col: usize) {
        self.tx
            .send(ClientCommand::MoveCursor(0, col as i16))
            .unwrap();
    }

    fn insert_blank(&mut self, count: usize) {
        log::info!("Insert blank: {}", count);
    }

    fn move_up(&mut self, u: usize) {
        self.tx
            .send(ClientCommand::MoveCursorVertical(-(u as i16)))
            .unwrap();
    }

    fn move_down(&mut self, d: usize) {
        self.tx
            .send(ClientCommand::MoveCursorVertical(d as i16))
            .unwrap();
    }

    fn identify_terminal(&mut self, intermediate: Option<char>) {
        match intermediate {
            Some('>') => {
                self.tx
                    .send(ClientCommand::IdentifyTerminal(
                        IdentifyTerminalMode::Secondary,
                    ))
                    .unwrap();
            }
            _ => {
                log::info!("Unknown intermediate: {:?}", intermediate);
            }
        }
        log::info!("Identify terminal");
    }

    fn device_status(&mut self, arg: usize) {
        match arg {
            5 => {
                self.tx.send(ClientCommand::ReportCondition(true)).unwrap();
            }
            6 => {
                self.tx.send(ClientCommand::ReportCursorPosition).unwrap();
            }
            _ => {
                log::info!("Unknown device status: {}", arg);
            }
        }
    }

    fn move_forward(&mut self, col: usize) {
        self.tx
            .send(ClientCommand::MoveCursorHorizontal(col as i16))
            .unwrap();
    }

    fn move_backward(&mut self, col: usize) {
        self.tx
            .send(ClientCommand::MoveCursorHorizontal(-(col as i16)))
            .unwrap();
    }

    fn move_down_and_cr(&mut self, _row: usize) {
        self.tx
            .send(ClientCommand::MoveCursorVerticalWithCarriageReturn(1))
            .unwrap();
    }

    fn move_up_and_cr(&mut self, _row: usize) {
        self.tx
            .send(ClientCommand::MoveCursorVerticalWithCarriageReturn(-1))
            .unwrap();
    }

    fn put_tab(&mut self, _count: u16) {
        self.tx.send(ClientCommand::PutTab).unwrap();
    }

    fn backspace(&mut self) {
        self.tx.send(ClientCommand::Backspace).unwrap();
    }

    fn carriage_return(&mut self) {
        self.tx.send(ClientCommand::CarriageReturn).unwrap();
    }

    fn linefeed(&mut self) {
        self.tx.send(ClientCommand::LineFeed).unwrap();
    }

    fn bell(&mut self) {
        log::info!("Bell");
    }

    fn substitute(&mut self) {
        log::info!("Substitute");
    }

    fn newline(&mut self) {
        self.tx.send(ClientCommand::NewLine).unwrap();
    }

    fn set_horizontal_tabstop(&mut self) {
        log::info!("Set horizontal tabstop");
    }

    fn scroll_up(&mut self, _: usize) {
        log::info!("Scroll up");
    }

    fn scroll_down(&mut self, _: usize) {
        log::info!("Scroll down");
    }

    fn insert_blank_lines(&mut self, _: usize) {
        log::info!("Insert blank lines");
    }

    fn delete_lines(&mut self, l: usize) {
        self.tx.send(ClientCommand::DeleteLines(l as i16)).unwrap();
    }

    fn erase_chars(&mut self, c: usize) {
        self.tx.send(ClientCommand::ClearCount(c as i16)).unwrap();
    }

    fn delete_chars(&mut self, _: usize) {
        log::info!("Delete chars");
    }

    fn move_backward_tabs(&mut self, _count: u16) {
        log::info!("Move backward tabs");
    }

    fn move_forward_tabs(&mut self, _count: u16) {
        log::info!("Move forward tabs");
    }

    fn save_cursor_position(&mut self) {
        self.tx.send(ClientCommand::SaveCursor).unwrap();
    }

    fn restore_cursor_position(&mut self) {
        self.tx.send(ClientCommand::RestoreCursor).unwrap();
    }

    fn clear_line(&mut self, mode: LineClearMode) {
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
        log::info!("Clear tabs");
    }

    fn set_tabs(&mut self, _interval: u16) {
        log::info!("Set tabs");
    }

    fn reset_state(&mut self) {
        log::info!("Reset state");
    }

    fn reverse_index(&mut self) {
        log::info!("Reverse index");
    }

    fn terminal_attribute(&mut self, attr: Attr) {
        if attr == Attr::Reset {
            self.tx.send(ClientCommand::ResetStyles).unwrap();
        } else {
            self.tx
                .send(ClientCommand::SGR(SgrAttribute::from_vte_attr(attr)))
                .unwrap();
        }
    }

    fn set_mode(&mut self, _mode: Mode) {
        log::info!("Set mode");
    }

    fn unset_mode(&mut self, _mode: Mode) {
        log::info!("Unset mode");
    }

    fn report_mode(&mut self, _mode: Mode) {
        log::info!("Report mode");
    }

    fn set_private_mode(&mut self, mode: PrivateMode) {
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
                log::info!("Set private mode: {:?}", mode);
            }
        }
    }

    fn unset_private_mode(&mut self, mode: PrivateMode) {
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
                log::info!("Unset private mode: {:?}", mode);
            }
        }
    }

    fn report_private_mode(&mut self, _mode: PrivateMode) {
        log::info!("Report private mode");
    }

    fn set_scrolling_region(&mut self, top: usize, bottom: Option<usize>) {
        log::info!("Set scrolling region: {} {:?}", top, bottom);
    }

    fn set_keypad_application_mode(&mut self) {
        log::info!("Set keypad application mode");
    }

    fn unset_keypad_application_mode(&mut self) {
        log::info!("Unset keypad application mode");
    }

    fn set_active_charset(&mut self, _: CharsetIndex) {
        log::info!("Set active charset");
    }

    fn configure_charset(&mut self, c: CharsetIndex, typ: StandardCharset) {
        log::info!("Configure charset: {:?} {:?}", c, typ);
    }

    fn set_color(&mut self, i: usize, rgb: Rgb) {
        self.tx.send(ClientCommand::SetColor(i, rgb)).unwrap();
    }

    fn dynamic_color_sequence(&mut self, _: String, _: usize, _: &str) {
        log::info!("Dynamic color sequence");
    }

    fn reset_color(&mut self, i: usize) {
        self.tx.send(ClientCommand::ResetColor(i)).unwrap();
    }

    fn clipboard_store(&mut self, _: u8, _: &[u8]) {
        log::info!("Clipboard store");
    }

    fn clipboard_load(&mut self, _: u8, _: &str) {
        log::info!("Clipboard load");
    }

    fn decaln(&mut self) {
        log::info!("DECALN");
    }

    fn push_title(&mut self) {
        log::info!("Push title");
    }

    fn pop_title(&mut self) {
        log::info!("Pop title");
    }

    fn text_area_size_pixels(&mut self) {
        log::info!("Text area size pixels");
    }

    fn text_area_size_chars(&mut self) {
        log::info!("Text area size chars");
    }

    fn set_hyperlink(&mut self, _: Option<Hyperlink>) {
        log::info!("Set hyperlink");
    }

    fn set_mouse_cursor_icon(&mut self, _: cursor_icon::CursorIcon) {
        log::info!("Set mouse cursor icon");
    }

    fn report_keyboard_mode(&mut self) {
        log::info!("Report keyboard mode");
    }

    fn push_keyboard_mode(&mut self, _mode: KeyboardModes) {
        log::info!("Push keyboard mode");
    }

    fn pop_keyboard_modes(&mut self, _to_pop: u16) {
        log::info!("Pop keyboard modes");
    }

    fn set_keyboard_mode(&mut self, _mode: KeyboardModes, _behavior: KeyboardModesApplyBehavior) {
        log::info!("Set keyboard mode");
    }

    fn set_modify_other_keys(&mut self, _mode: ModifyOtherKeys) {
        log::info!("Set modify other keys");
    }

    fn report_modify_other_keys(&mut self) {
        log::info!("Report modify other keys");
    }

    fn set_scp(&mut self, _char_path: ScpCharPath, _update_mode: ScpUpdateMode) {
        log::info!("Set SCP");
    }
}
