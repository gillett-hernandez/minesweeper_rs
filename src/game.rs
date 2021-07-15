use rand::prelude::*;
#[derive(Copy, Clone, PartialEq)]
pub enum CellState {
    Empty,
    Mine,
}

#[derive(Copy, Clone, PartialEq)]
pub enum CellVisibility {
    Unknown,
    Flagged,
    Empty(usize), // number of neighbors that are mines.
}

#[derive(Copy, Clone)]
pub struct Cell {
    pub state: CellState,
    pub visibility: CellVisibility,
}

#[derive(Copy, Clone, PartialEq)]
pub enum GameCondition {
    InProgress,
    Won,
    Lost,
}

pub struct GameState<const X: usize, const Y: usize> {
    pub field: Vec<Vec<Cell>>,
    pub sub_state: GameCondition,
    pub bomb_count: usize,
    flagged_count: usize,
}

impl<const X: usize, const Y: usize> GameState<X, Y> {
    pub fn new(num_bombs: usize) -> Self {
        let mut cells = vec![
            vec![
                Cell {
                    state: CellState::Empty,
                    visibility: CellVisibility::Unknown
                };
                X
            ];
            Y
        ];

        for bomb in 0..num_bombs {
            // note: naive mine generation can lead to unsolvable patterns.
            loop {
                let (x, y) = GameState::<X, Y>::random_xy();
                if cells[y][x].state == CellState::Empty {
                    cells[y][x].state = CellState::Mine;
                    break;
                }
            }
        }

        GameState {
            field: cells,
            sub_state: GameCondition::InProgress,
            bomb_count: num_bombs,
            flagged_count: 0,
        }
    }
    pub fn remaining_mines(&self) -> usize {
        self.bomb_count - self.flagged_count
    }

    pub fn random_xy() -> (usize, usize) {
        (
            (random::<f32>() * X as f32) as usize,
            (random::<f32>() * Y as f32) as usize,
        )
    }

    pub fn random_xy_2(&self) -> (usize, usize) {
        GameState::<X, Y>::random_xy()
    }

    pub fn at(&self, x: usize, y: usize) -> Option<Cell> {
        if x >= X || y >= Y {
            None
        } else {
            Some(self.field[y][x])
        }
    }

    pub fn at_offset(&self, x: usize, y: usize, xo: isize, yo: isize) -> Option<Cell> {
        println!("{} {} {} {}", x, y, xo, yo);
        if (x == 0 && xo < 0) || (y == 0 && xo < 0) {
            None
        } else if (x as isize + xo) as usize >= X || (y as isize + yo) as usize >= Y {
            None
        } else {
            Some(self.field[y][x])
        }
    }

    pub fn at_mut(&mut self, x: usize, y: usize) -> Option<&mut Cell> {
        if x >= X || y >= Y {
            None
        } else {
            Some(&mut self.field[y][x])
        }
    }

    pub fn neighbors(&self, x: usize, y: usize) -> Vec<(usize, usize)> {
        let mut neighbors = Vec::new();
        for x_offset in [-1isize, 0, 1].iter() {
            for y_offset in [-1isize, 0, 1].iter() {
                if *x_offset == 0 && *y_offset == 0 {
                    continue;
                }
                if (x == 0 && *x_offset < 0) || (y == 0 && *y_offset < 0) {
                    continue;
                }
                neighbors.push((
                    (x as isize + *x_offset) as usize,
                    (y as isize + *y_offset) as usize,
                ));
            }
        }
        neighbors
    }

    pub fn flag(&mut self, x: usize, y: usize) {
        let copy = self.at(x, y);
        if copy.is_none() {
            return;
        }
        let copy = copy.unwrap();
        if copy.state == CellState::Mine {
            self.flagged_count += 1;
            if self.flagged_count == self.bomb_count {
                self.sub_state = GameCondition::Won;
            }
        }
        // for (nx, ny) in self.neighbors(x, y).iter() {
        //     if let Some(Cell {
        //         visibility: CellVisibility::Empty(neighbors),
        //         state,
        //     }) = self.at(*nx, *ny)
        //     {
        //         if neighbors >= 1 {
        //             *self.at_mut(*nx, *ny).unwrap() = Cell {
        //                 state,
        //                 visibility: CellVisibility::Empty(neighbors - 1),
        //             };
        //         }
        //     }
        // }
        let mut_cell = self.at_mut(x, y).unwrap();
        *mut_cell = Cell {
            visibility: CellVisibility::Flagged,
            ..copy
        };
    }

    pub fn click(&mut self, x: usize, y: usize) {
        let mut click_queue = vec![(x, y)];
        loop {
            let coords = click_queue.pop();
            if coords.is_none() {
                break;
            }
            let (x, y) = coords.unwrap();
            if let Some(copy) = self.at(x, y) {
                let mut click_neighbors = false;
                *self.at_mut(x, y).unwrap() = match copy {
                    Cell {
                        state: CellState::Mine,
                        ..
                    } => {
                        self.sub_state = GameCondition::Lost;
                        copy
                    }
                    Cell {
                        state: CellState::Empty,
                        visibility: CellVisibility::Unknown,
                    } => {
                        // calculate neighbors
                        let mine_count = self
                            .neighbors(x, y)
                            .iter()
                            .map(|(x, y)| {
                                if let Some(cell) = self.at(*x, *y) {
                                    if cell.state == CellState::Mine {
                                        1usize
                                    } else {
                                        0
                                    }
                                } else {
                                    0
                                }
                            })
                            .sum::<usize>();

                        if mine_count == 0 {
                            click_neighbors = true;
                        }

                        Cell {
                            visibility: CellVisibility::Empty(mine_count),
                            ..copy
                        }
                    }
                    _ => copy,
                };
                if click_neighbors {
                    for x_offset in [-1isize, 0, 1].iter() {
                        for y_offset in [-1isize, 0, 1].iter() {
                            if *x_offset == 0 && *y_offset == 0 {
                                continue;
                            }
                            if (x == 0 && *x_offset < 0) || (y == 0 && *y_offset < 0) {
                                continue;
                            }
                            click_queue.push((
                                (x as isize + x_offset) as usize,
                                (y as isize + y_offset) as usize,
                            ));
                        }
                    }
                }
            }
        }
    }

    pub fn validate(&self, hypothetical: &GameState<X, Y>) -> bool {
        // returns whether the hypothetical gamestate is the same as the current gamestate
        // after visibility is factored in.

        for (cell1, cell2) in self
            .field
            .iter()
            .flatten()
            .zip(hypothetical.field.iter().flatten())
        {
            if cell1.visibility != cell2.visibility {
                return false;
            }
        }
        true
    }
}
