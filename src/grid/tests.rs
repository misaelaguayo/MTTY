use crate::{
    config::Config,
    grid::{Cell, Grid},
    styles::Color,
};

#[test]
fn set_pos_should_set_cursor_position() {
    let mut grid = Grid::new(&Config::default());

    grid.set_pos(5, 10);
    assert_eq!(grid.cursor_pos, (5, 10));
}

#[test]
fn delete_character_should_remove_character_in_row() {
    let mut grid = Grid::new(&Config::default());

    grid.set_pos(5, 10);
    grid.cells[5][10] = Cell {
        char: 'a',
        ..Cell::default()
    };

    grid.delete_character();

    assert_eq!(grid.cursor_pos, (5, 9));
    assert_eq!(grid.cells[5][10].char, ' ');
}

#[test]
fn delete_character_should_wrap_to_previous_line_if_pos_at_beginning() {
    let config = Config {
        rows: 10,
        cols: 10,
        ..Config::default()
    };
    let mut grid = Grid::new(&config);

    grid.set_pos(9, 0);
    grid.cells[9][0] = Cell {
        char: 'a',
        ..Cell::default()
    };

    grid.delete_character();

    assert_eq!(grid.cursor_pos, (8, 9));
    assert_eq!(grid.cells[9][0].char, ' ');
}

#[test]
fn place_character_in_grid_should_place_character_in_grid() {
    let mut grid = Grid::new(&Config::default());

    grid.set_pos(5, 10);
    grid.place_character_in_grid(10, 'a');

    assert_eq!(grid.cursor_pos, (6, 1));
    assert_eq!(grid.cells[6][0].char, 'a');
}

#[test]
fn place_character_in_grid_with_newline_should_move_cursor_to_start_of_line() {
    let mut grid = Grid::new(&Config::default());

    grid.set_pos(5, 5);
    grid.place_character_in_grid(10, '\n');

    assert_eq!(grid.cursor_pos, (6, 0));
}

#[test]
fn place_character_in_grid_with_carriage_return_should_move_cursor_to_start_of_line() {
    let mut grid = Grid::new(&Config::default());

    grid.set_pos(5, 5);
    grid.place_character_in_grid(10, '\r');

    assert_eq!(grid.cursor_pos, (5, 0));
}

#[test]
fn clear_screen_should_clear_grid() {
    let mut grid = Grid::new(&Config::default());

    grid.cells.iter_mut().for_each(|row| {
        row.iter_mut()
            .for_each(|cell| *cell = Cell::new('a', Color::White, Color::Black));
    });

    grid.clear_screen();
    assert!(grid
        .cells
        .iter()
        .all(|row| row.iter().all(|cell| cell.char == ' ')));
}
