use vte::ansi::{Attr, Rgb};

use crate::styles::{Color, CursorShape, CursorState};

#[derive(Debug, Clone)]
pub enum IdentifyTerminalMode {
    Primary,
    Secondary,
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub enum Command {
    AlternateScreenBuffer(bool),
    Backspace,
    BrackPasteMode(bool),
    CarriageReturn,
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
    Put(char),
    PutTab,
    ReportCondition(bool),
    ReportCursorPosition,
    ResetColor(usize),
    ResetStyles,
    RestoreCursor,
    SGR(SgrAttribute),
    SaveCursor,
    SetColor(usize, Rgb),
    ShowCursor,
    SwapScreenAndSetRestoreCursor,
    DeleteLines(i16),
    SetCursorState(CursorState),
    SetCursorShape(CursorShape),
}
