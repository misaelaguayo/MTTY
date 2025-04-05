use std::sync::{atomic::AtomicBool, Arc};

use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::{commands::Command, config::Config, ui::Ui};

#[test]
fn set_pos_should_set_cursor_position() {
    let (input_tx, _): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel(100);
    let (_, output_rx): (Sender<Command>, Receiver<Command>) = mpsc::channel(100);

    let mut ui = Ui::new(
        &Config::default(),
        Arc::new(AtomicBool::new(false)),
        input_tx,
        output_rx,
    );

    ui.set_pos(5, 10);

    assert_eq!(ui.pos, (5, 10));
}

#[test]
fn handle_command_backspace_should_remove_character_in_row() {
    let (input_tx, _): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel(100);
    let (_, output_rx): (Sender<Command>, Receiver<Command>) = mpsc::channel(100);

    let mut ui = Ui::new(
        &Config::default(),
        Arc::new(AtomicBool::new(false)),
        input_tx,
        output_rx,
    );

    ui.set_pos(5, 10);
    ui.grid[5][10] = 'a';

    ui.handle_command(Command::Backspace);

    assert_eq!(ui.pos, (5, 9));
    assert_eq!(ui.grid[5][10], ' ');
}

#[test]
fn handle_command_backspace_should_wrap_to_previous_line_if_pos_at_beginning() {
    let (input_tx, _): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel(100);
    let (_, output_rx): (Sender<Command>, Receiver<Command>) = mpsc::channel(100);
    let config = Config {
        rows: 10,
        cols: 10,
        ..Config::default()
    };

    let mut ui = Ui::new(
        &config,
        Arc::new(AtomicBool::new(false)),
        input_tx,
        output_rx,
    );

    ui.set_pos(9, 0);
    ui.grid[9][0] = 'a';

    ui.handle_command(Command::Backspace);

    assert_eq!(ui.pos, (8, 9));
    assert_eq!(ui.grid[9][0], ' ');
}

#[test]
fn handle_command_print_character_should_place_character_in_grid() {
    let (input_tx, _): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel(100);
    let (_, output_rx): (Sender<Command>, Receiver<Command>) = mpsc::channel(100);

    let mut ui = Ui::new(
        &Config::default(),
        Arc::new(AtomicBool::new(false)),
        input_tx,
        output_rx,
    );

    ui.set_pos(5, 10);
    ui.handle_command(Command::Print('a'));

    assert_eq!(ui.grid[5][10], 'a');
}

#[test]
fn handle_command_new_line_should_move_cursor_to_next_line() {
    let (input_tx, _): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel(100);
    let (_, output_rx): (Sender<Command>, Receiver<Command>) = mpsc::channel(100);
    let config = Config {
        rows: 10,
        cols: 10,
        ..Config::default()
    };

    let mut ui = Ui::new(
        &config,
        Arc::new(AtomicBool::new(false)),
        input_tx,
        output_rx,
    );

    ui.set_pos(5, 8);
    ui.handle_command(Command::NewLine);

    assert_eq!(ui.pos, (6, 8));
}

#[test]
fn handle_command_carriage_return_should_move_cursor_to_start_of_line() {
    let (input_tx, _): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel(100);
    let (_, output_rx): (Sender<Command>, Receiver<Command>) = mpsc::channel(100);

    let mut ui = Ui::new(
        &Config::default(),
        Arc::new(AtomicBool::new(false)),
        input_tx,
        output_rx,
    );

    ui.set_pos(5, 10);
    ui.handle_command(Command::CarriageReturn);

    assert_eq!(ui.pos, (5, 0));
}

#[test]
fn handle_command_clear_screen_should_clear_grid() {
    let (input_tx, _): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel(100);
    let (_, output_rx): (Sender<Command>, Receiver<Command>) = mpsc::channel(100);

    let mut ui = Ui::new(
        &Config::default(),
        Arc::new(AtomicBool::new(false)),
        input_tx,
        output_rx,
    );

    ui.grid.iter_mut().for_each(|row| {
        row.iter_mut().for_each(|cell| *cell = 'a');
    });

    ui.handle_command(Command::ClearScreen);

    assert!(ui
        .grid
        .iter()
        .all(|row| row.iter().all(|&cell| cell == ' ')));
}

#[test]
fn handle_command_move_cursor_should_move_cursor_to_position() {
    let (input_tx, _): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel(100);
    let (_, output_rx): (Sender<Command>, Receiver<Command>) = mpsc::channel(100);

    let mut ui = Ui::new(
        &Config::default(),
        Arc::new(AtomicBool::new(false)),
        input_tx,
        output_rx,
    );

    ui.handle_command(Command::MoveCursor(5, 10));

    assert_eq!(ui.pos, (10, 5));
}

#[test]
fn handle_command_move_cursor_absolute_horizontal_should_move_cursor_to_position() {
    let (input_tx, _): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel(100);
    let (_, output_rx): (Sender<Command>, Receiver<Command>) = mpsc::channel(100);

    let mut ui = Ui::new(
        &Config::default(),
        Arc::new(AtomicBool::new(false)),
        input_tx,
        output_rx,
    );

    ui.set_pos(0, 1);
    ui.handle_command(Command::MoveCursorAbsoluteHorizontal(5));

    assert_eq!(ui.pos, (0, 5));
}

#[test]
fn handle_command_move_cursor_horizontal_should_move_cursor_relative() {
    let (input_tx, _): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel(100);
    let (_, output_rx): (Sender<Command>, Receiver<Command>) = mpsc::channel(100);

    let mut ui = Ui::new(
        &Config::default(),
        Arc::new(AtomicBool::new(false)),
        input_tx,
        output_rx,
    );

    ui.set_pos(0, 5);
    ui.handle_command(Command::MoveCursorHorizontal(3));

    assert_eq!(ui.pos, (0, 8));
}

#[test]
fn handle_command_move_cursor_vertical_should_move_cursor_relative() {
    let (input_tx, _): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel(100);
    let (_, output_rx): (Sender<Command>, Receiver<Command>) = mpsc::channel(100);

    let mut ui = Ui::new(
        &Config::default(),
        Arc::new(AtomicBool::new(false)),
        input_tx,
        output_rx,
    );

    ui.set_pos(5, 1);
    ui.handle_command(Command::MoveCursorVertical(3));

    assert_eq!(ui.pos, (4, 5));
}
