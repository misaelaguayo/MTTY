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

    /// Send a command, logging any errors instead of panicking
    fn send(&self, cmd: ClientCommand) {
        if let Err(e) = self.tx.send(cmd) {
            log::trace!("Failed to send command (channel closed): {}", e);
        }
    }
}

impl Handler for StateMachine {
    fn set_title(&mut self, _: Option<String>) {
        log::error!("Set title");
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

                self.send(ClientCommand::SetCursorState(CursorState::new(
                    shape, blinking,
                )));
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

        self.send(ClientCommand::SetCursorShape(cursor_shape));
    }

    fn input(&mut self, c: char) {
        log::trace!("Input character: {}", c);
        self.send(ClientCommand::Print(c));
    }

    fn goto(&mut self, line: i32, col: usize) {
        log::debug!("Goto line: {}, col: {}", line, col);
        self.send(ClientCommand::MoveCursor(line as i16, col as i16));
    }

    fn goto_line(&mut self, line: i32) {
        log::debug!("Goto line: {}", line);
        self.send(ClientCommand::MoveCursor(line as i16, 0));
    }

    fn goto_col(&mut self, col: usize) {
        log::debug!("Goto col: {}", col);
        self.send(ClientCommand::MoveCursorAbsoluteHorizontal(col as i16));
    }

    fn insert_blank(&mut self, count: usize) {
        log::debug!("Insert blank: {}", count);
        self.send(ClientCommand::InsertBlanks(count as i16));
    }

    fn move_up(&mut self, u: usize) {
        log::debug!("Move up: {}", u);
        self.send(ClientCommand::MoveCursorVertical(-(u as i16)));
    }

    fn move_down(&mut self, d: usize) {
        log::debug!("Move down: {}", d);
        self.send(ClientCommand::MoveCursorVertical(d as i16));
    }

    fn identify_terminal(&mut self, intermediate: Option<char>) {
        log::debug!("Identify terminal: {:?}", intermediate);
        match intermediate {
            None => {
                // Primary device attributes (DA1) - report as VT220
                self.send(ClientCommand::IdentifyTerminal(
                    IdentifyTerminalMode::Primary,
                ));
            }
            Some('>') => {
                // Secondary device attributes (DA2)
                self.send(ClientCommand::IdentifyTerminal(
                    IdentifyTerminalMode::Secondary,
                ));
            }
            _ => {
                log::debug!("Unknown identify terminal intermediate: {:?}", intermediate);
            }
        }
    }

    fn device_status(&mut self, arg: usize) {
        log::debug!("Device status: {}", arg);
        match arg {
            5 => {
                self.send(ClientCommand::ReportCondition(true));
            }
            6 => {
                self.send(ClientCommand::ReportCursorPosition);
            }
            _ => {
                log::error!("Unknown device status: {}", arg);
            }
        }
    }

    fn move_forward(&mut self, col: usize) {
        log::debug!("Move forward: {}", col);
        self.send(ClientCommand::MoveCursorHorizontal(col as i16));
    }

    fn move_backward(&mut self, col: usize) {
        log::debug!("Move backward: {}", col);
        self.send(ClientCommand::MoveCursorHorizontal(-(col as i16)));
    }

    fn move_down_and_cr(&mut self, _row: usize) {
        log::debug!("Move down and CR");
        self.send(ClientCommand::MoveCursorVerticalWithCarriageReturn(1));
    }

    fn move_up_and_cr(&mut self, _row: usize) {
        log::debug!("Move up and CR");
        self.send(ClientCommand::MoveCursorVerticalWithCarriageReturn(-1));
    }

    fn put_tab(&mut self, _count: u16) {
        log::debug!("Put tab");
        self.send(ClientCommand::PutTab);
    }

    fn backspace(&mut self) {
        log::debug!("Backspace");
        self.send(ClientCommand::Backspace);
    }

    fn carriage_return(&mut self) {
        log::debug!("Carriage return");
        self.send(ClientCommand::CarriageReturn);
    }

    fn linefeed(&mut self) {
        log::debug!("Line feed");
        self.send(ClientCommand::LineFeed);
    }

    fn bell(&mut self) {
        log::error!("Bell");
    }

    fn substitute(&mut self) {
        log::error!("Substitute");
    }

    fn newline(&mut self) {
        log::debug!("Newline");
        self.send(ClientCommand::NewLine);
    }

    fn set_horizontal_tabstop(&mut self) {
        log::error!("Set horizontal tabstop");
    }

    fn scroll_up(&mut self, count: usize) {
        log::debug!("Scroll up: {}", count);
        self.send(ClientCommand::ScrollUp(count as i16));
    }

    fn scroll_down(&mut self, count: usize) {
        log::debug!("Scroll down: {}", count);
        self.send(ClientCommand::ScrollDown(count as i16));
    }

    fn insert_blank_lines(&mut self, count: usize) {
        log::debug!("Insert blank lines: {}", count);
        self.send(ClientCommand::InsertBlankLines(count as i16));
    }

    fn delete_lines(&mut self, l: usize) {
        log::debug!("Delete lines: {}", l);
        self.send(ClientCommand::DeleteLines(l as i16));
    }

    fn erase_chars(&mut self, c: usize) {
        log::debug!("Erase chars: {}", c);
        self.send(ClientCommand::ClearCount(c as i16));
    }

    fn delete_chars(&mut self, count: usize) {
        log::debug!("Delete chars: {}", count);
        self.send(ClientCommand::DeleteChars(count as i16));
    }

    fn move_backward_tabs(&mut self, _count: u16) {
        log::error!("Move backward tabs");
    }

    fn move_forward_tabs(&mut self, _count: u16) {
        log::error!("Move forward tabs");
    }

    fn save_cursor_position(&mut self) {
        log::debug!("Save cursor position");
        self.send(ClientCommand::SaveCursor);
    }

    fn restore_cursor_position(&mut self) {
        log::debug!("Restore cursor position");
        self.send(ClientCommand::RestoreCursor);
    }

    fn clear_line(&mut self, mode: LineClearMode) {
        log::debug!("Clear line: {:?}", mode);
        match mode {
            LineClearMode::All => {
                self.send(ClientCommand::ClearLine);
            }
            LineClearMode::Left => {
                self.send(ClientCommand::ClearLineBeforeCursor);
            }
            LineClearMode::Right => {
                self.send(ClientCommand::ClearLineAfterCursor);
            }
        }
    }

    fn clear_screen(&mut self, mode: ClearMode) {
        log::debug!("Clear screen: {:?}", mode);
        match mode {
            ClearMode::All => {
                self.send(ClientCommand::ClearScreen);
            }
            ClearMode::Above => {
                self.send(ClientCommand::ClearAbove);
            }
            ClearMode::Below => {
                self.send(ClientCommand::ClearBelow);
            }
            ClearMode::Saved => {
                // Should also delete lines saved in the scrollback buffer
                self.send(ClientCommand::ClearScreen);
            }
        }
    }

    fn clear_tabs(&mut self, _mode: TabulationClearMode) {
        log::error!("Clear tabs");
    }

    fn set_tabs(&mut self, _interval: u16) {
        log::error!("Set tabs");
    }

    fn reset_state(&mut self) {
        log::error!("Reset state");
    }

    fn reverse_index(&mut self) {
        log::debug!("Reverse index");
        self.send(ClientCommand::ReverseIndex);
    }

    fn terminal_attribute(&mut self, attr: Attr) {
        log::debug!("Terminal attribute: {:?}", attr);
        self.send(ClientCommand::SGR(SgrAttribute::from_vte_attr(attr)));
    }

    fn set_mode(&mut self, _mode: Mode) {
        log::error!("Set mode");
    }

    fn unset_mode(&mut self, _mode: Mode) {
        log::error!("Unset mode");
    }

    fn report_mode(&mut self, _mode: Mode) {
        log::error!("Report mode");
    }

    fn set_private_mode(&mut self, mode: PrivateMode) {
        log::debug!("Set private mode: {:?}", mode);
        match mode {
            PrivateMode::Named(NamedPrivateMode::ShowCursor) => {
                self.send(ClientCommand::ShowCursor);
            }
            PrivateMode::Named(NamedPrivateMode::SwapScreenAndSetRestoreCursor) => {
                self.send(ClientCommand::SwapScreenAndSetRestoreCursor);
            }
            PrivateMode::Named(NamedPrivateMode::CursorKeys) => {
                self.send(ClientCommand::CursorKeysMode(true));
            }
            PrivateMode::Named(NamedPrivateMode::BracketedPaste) => {
                self.send(ClientCommand::BracketedPasteMode(true));
            }
            _ => {
                log::debug!("Unhandled set private mode: {:?}", mode);
            }
        }
    }

    fn unset_private_mode(&mut self, mode: PrivateMode) {
        log::debug!("Unset private mode: {:?}", mode);
        match mode {
            PrivateMode::Named(NamedPrivateMode::ShowCursor) => {
                self.send(ClientCommand::HideCursor);
            }
            PrivateMode::Named(NamedPrivateMode::SwapScreenAndSetRestoreCursor) => {
                self.send(ClientCommand::SwapScreenAndSetRestoreCursor);
            }
            PrivateMode::Named(NamedPrivateMode::CursorKeys) => {
                self.send(ClientCommand::CursorKeysMode(false));
            }
            PrivateMode::Named(NamedPrivateMode::BracketedPaste) => {
                self.send(ClientCommand::BracketedPasteMode(false));
            }
            _ => {
                log::debug!("Unhandled unset private mode: {:?}", mode);
            }
        }
    }

    fn report_private_mode(&mut self, _mode: PrivateMode) {
        log::error!("Report private mode");
    }

    fn set_scrolling_region(&mut self, top: usize, bottom: Option<usize>) {
        log::debug!("Set scrolling region: {} {:?}", top, bottom);
        self.send(ClientCommand::SetScrollingRegion(top, bottom));
    }

    fn set_keypad_application_mode(&mut self) {
        log::error!("Set keypad application mode");
    }

    fn unset_keypad_application_mode(&mut self) {
        log::error!("Unset keypad application mode");
    }

    fn set_active_charset(&mut self, _: CharsetIndex) {
        // Character set switching is not implemented but is harmless to ignore
    }

    fn configure_charset(&mut self, _: CharsetIndex, _: StandardCharset) {
        // Character set configuration (e.g., G0 ASCII) is not implemented but is harmless to ignore
    }

    fn set_color(&mut self, i: usize, rgb: Rgb) {
        log::debug!("Set color: {} {:?}", i, rgb);
        self.send(ClientCommand::SetColor(i, rgb));
    }

    fn dynamic_color_sequence(&mut self, _prefix: String, index: usize, color: &str) {
        log::debug!("Dynamic color sequence: index={}, color={}", index, color);

        // Parse color string - formats like "#RRGGBB" or "rgb:RR/GG/BB" or "rgbi:R/G/B"
        let rgb = if let Some(hex) = color.strip_prefix('#') {
            // #RRGGBB format
            if hex.len() >= 6 {
                let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
                let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
                let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
                Some(Rgb { r, g, b })
            } else {
                None
            }
        } else if let Some(rgb_str) = color.strip_prefix("rgb:") {
            // rgb:RR/GG/BB or rgb:RRRR/GGGG/BBBB format (X11 color spec)
            let parts: Vec<&str> = rgb_str.split('/').collect();
            if parts.len() == 3 {
                // Take first 2 hex digits of each component (handles both 2 and 4 digit forms)
                let r = u8::from_str_radix(&parts[0].chars().take(2).collect::<String>(), 16)
                    .unwrap_or(0);
                let g = u8::from_str_radix(&parts[1].chars().take(2).collect::<String>(), 16)
                    .unwrap_or(0);
                let b = u8::from_str_radix(&parts[2].chars().take(2).collect::<String>(), 16)
                    .unwrap_or(0);
                Some(Rgb { r, g, b })
            } else {
                None
            }
        } else {
            None
        };

        if let Some(rgb) = rgb {
            match index {
                10 => self.send(ClientCommand::SetDefaultForeground(rgb)),
                11 => self.send(ClientCommand::SetDefaultBackground(rgb)),
                // Index 12 is cursor color, not implemented yet
                _ => log::debug!("Unhandled dynamic color index: {}", index),
            }
        }
    }

    fn reset_color(&mut self, i: usize) {
        log::debug!("Reset color: {}", i);
        self.send(ClientCommand::ResetColor(i));
    }

    fn clipboard_store(&mut self, _: u8, _: &[u8]) {
        log::error!("Clipboard store");
    }

    fn clipboard_load(&mut self, _: u8, _: &str) {
        log::error!("Clipboard load");
    }

    fn decaln(&mut self) {
        log::error!("DECALN");
    }

    fn push_title(&mut self) {
        log::error!("Push title");
    }

    fn pop_title(&mut self) {
        log::error!("Pop title");
    }

    fn text_area_size_pixels(&mut self) {
        log::error!("Text area size pixels");
    }

    fn text_area_size_chars(&mut self) {
        log::error!("Text area size chars");
    }

    fn set_hyperlink(&mut self, _: Option<Hyperlink>) {
        log::error!("Set hyperlink");
    }

    fn set_mouse_cursor_icon(&mut self, _: cursor_icon::CursorIcon) {
        log::error!("Set mouse cursor icon");
    }

    fn report_keyboard_mode(&mut self) {
        log::error!("Report keyboard mode");
    }

    fn push_keyboard_mode(&mut self, _mode: KeyboardModes) {
        log::error!("Push keyboard mode");
    }

    fn pop_keyboard_modes(&mut self, _to_pop: u16) {
        log::error!("Pop keyboard modes");
    }

    fn set_keyboard_mode(&mut self, _mode: KeyboardModes, _behavior: KeyboardModesApplyBehavior) {
        log::error!("Set keyboard mode");
    }

    fn set_modify_other_keys(&mut self, _mode: ModifyOtherKeys) {
        log::error!("Set modify other keys");
    }

    fn report_modify_other_keys(&mut self) {
        log::error!("Report modify other keys");
    }

    fn set_scp(&mut self, _char_path: ScpCharPath, _update_mode: ScpUpdateMode) {
        log::error!("Set SCP");
    }
}
