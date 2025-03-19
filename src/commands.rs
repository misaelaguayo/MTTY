pub enum Command {
    CarriageReturn,
    ClearAbove,
    ClearBelow,
    ClearScreen,
    Exit,
    NewLine,
    Print(char),
    ResetStyles
}
