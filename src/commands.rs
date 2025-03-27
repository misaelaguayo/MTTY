pub enum Command {
    AlternateScreenBuffer(bool),
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
