use serde::{Deserialize, Serialize};
use vte::ansi::{Attr, Rgb};

use crate::styles::{Color, CursorShape, CursorState};

/// Serializable wrapper for vte::ansi::Rgb
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SerializableRgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl From<Rgb> for SerializableRgb {
    fn from(rgb: Rgb) -> Self {
        Self {
            r: rgb.r,
            g: rgb.g,
            b: rgb.b,
        }
    }
}

impl From<SerializableRgb> for Rgb {
    fn from(rgb: SerializableRgb) -> Self {
        Self {
            r: rgb.r,
            g: rgb.g,
            b: rgb.b,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IdentifyTerminalMode {
    Primary,
    Secondary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SgrAttribute {
    Reset,
    Bold,
    Dim,
    Italic,
    Underline,
    DoubleUnderline,
    Undercurl,
    DottedUnderline,
    DashedUnderline,
    BlinkSlow,
    BlinkFast,
    Reverse,
    Hidden,
    Strike,
    CancelBold,
    CancelBoldDim,
    CancelItalic,
    CancelUnderline,
    CancelBlink,
    CancelReverse,
    CancelHidden,
    CancelStrike,
    Foreground(Color),
    Background(Color),
    UnderlineColor(Option<Color>),
}

impl SgrAttribute {
    pub fn default() -> Self {
        SgrAttribute::Reset
    }

    pub fn from_vte_attr(attr: Attr) -> Self {
        match attr {
            Attr::Reset => SgrAttribute::Reset,
            Attr::Bold => SgrAttribute::Bold,
            Attr::Dim => SgrAttribute::Dim,
            Attr::Italic => SgrAttribute::Italic,
            Attr::Underline => SgrAttribute::Underline,
            Attr::DoubleUnderline => SgrAttribute::DoubleUnderline,
            Attr::Undercurl => SgrAttribute::Undercurl,
            Attr::DottedUnderline => SgrAttribute::DottedUnderline,
            Attr::DashedUnderline => SgrAttribute::DashedUnderline,
            Attr::BlinkSlow => SgrAttribute::BlinkSlow,
            Attr::BlinkFast => SgrAttribute::BlinkFast,
            Attr::Reverse => SgrAttribute::Reverse,
            Attr::Hidden => SgrAttribute::Hidden,
            Attr::Strike => SgrAttribute::Strike,
            Attr::CancelBold => SgrAttribute::CancelBold,
            Attr::CancelBoldDim => SgrAttribute::CancelBoldDim,
            Attr::CancelItalic => SgrAttribute::CancelItalic,
            Attr::CancelUnderline => SgrAttribute::CancelUnderline,
            Attr::CancelBlink => SgrAttribute::CancelBlink,
            Attr::CancelReverse => SgrAttribute::CancelReverse,
            Attr::CancelHidden => SgrAttribute::CancelHidden,
            Attr::CancelStrike => SgrAttribute::CancelStrike,
            Attr::Foreground(color) => SgrAttribute::Foreground(Color::from_vte_color(color)),
            Attr::Background(color) => SgrAttribute::Background(Color::from_vte_color(color)),
            Attr::UnderlineColor(color) => {
                SgrAttribute::UnderlineColor(color.map(Color::from_vte_color))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerCommand {
    Resize(u16, u16, u16, u16),
    RawData(Vec<u8>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientCommand {
    SetTitle(Option<String>),
    AlternateScreenBuffer(bool),
    Backspace,
    BracketedPasteMode(bool),
    CarriageReturn,
    CursorKeysMode(bool),
    ClearAbove,
    ClearBelow,
    ClearCount(i16),
    ClearLine,
    ClearLineAfterCursor,
    ClearLineBeforeCursor,
    ClearScreen,
    Exit,
    HideCursor,
    IdentifyTerminal(IdentifyTerminalMode),
    LineFeed,
    MoveCursor(i16, i16),
    MoveCursorAbsoluteHorizontal(i16),
    MoveCursorHorizontal(i16),
    MoveCursorLineVertical(i16),
    MoveCursorVertical(i16),
    MoveCursorVerticalWithCarriageReturn(i16),
    NewLine,
    Print(char),
    PutTab,
    ReportCondition(bool),
    ReportCursorPosition,
    ResetColor(usize),
    RestoreCursor,
    SGR(SgrAttribute),
    SaveCursor,
    SetColor(usize, SerializableRgb),
    ShowCursor,
    /// Enter (true) or exit (false) alternate screen with cursor save/restore
    SwapScreenAndSetRestoreCursor(bool),
    DeleteLines(i16),
    InsertBlankLines(i16),
    ScrollUp(i16),
    ScrollDown(i16),
    SetScrollingRegion(usize, Option<usize>),
    ReverseIndex,
    InsertBlanks(i16),
    DeleteChars(i16),
    SetCursorState(CursorState),
    SetCursorShape(CursorShape),
    SetDefaultForeground(SerializableRgb),
    SetDefaultBackground(SerializableRgb),
    ReportTextAreaSizeChars,
    ReportTextAreaSizePixels,
}
