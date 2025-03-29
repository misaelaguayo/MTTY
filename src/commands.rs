pub enum Command {
    AlternateScreenBuffer(bool),
    Backspace,
    BrackPasteMode(bool),
    CarriageReturn,
    ClearAbove,
    ClearBelow,
    ClearScreen,
    Exit,
    NewLine,
    Print(char),
    ResetStyles,
}
