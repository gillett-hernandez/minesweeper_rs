use rayon::iter::{
    IndexedParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator,
};

use crate::game::*;

pub trait Strategy {
    fn attempt(&mut self, game_state: &GameState) -> Vec<Event>;
    fn update(&mut self, game_state: &GameState, event: Event);
}

pub struct BijectionDetection {
    initialized: bool,
    cells_of_interest: Vec<bool>,
}

impl Strategy for BijectionDetection {
    fn attempt(&mut self, game_state: &GameState) -> Vec<Event> {
        let width = game_state.width;
        let first_cells: Vec<Event> = self
            .cells_of_interest
            .par_iter_mut()
            .enumerate()
            .filter_map(|(i, tracked)| {
                if !*tracked {
                    return None;
                }
                let (x, y) = (i % width, i / width);
                let center_cell = game_state.at(x, y).unwrap();
                let mut suggested_cell = Event::None;
                match center_cell {
                    Cell {
                        visibility: CellVisibility::Empty(num_neighbor_mines),
                        ..
                    } => {
                        // detect one to one correspondence of unclicked cells to number of active unflagged mines.
                        let unknown_neighbor_count: usize = game_state
                            .neighbors(x, y)
                            .iter()
                            .map(|c| {
                                if let Some(Cell {
                                    visibility: CellVisibility::Unknown,
                                    ..
                                }) = game_state.at(c.0, c.1)
                                {
                                    1usize
                                } else {
                                    0usize
                                }
                            })
                            .sum();
                        let flagged_neighbor_count: usize = game_state
                            .neighbors(x, y)
                            .iter()
                            .map(|c| {
                                if let Some(Cell {
                                    visibility: CellVisibility::Flagged,
                                    ..
                                }) = game_state.at(c.0, c.1)
                                {
                                    1usize
                                } else {
                                    0usize
                                }
                            })
                            .sum();
                        if unknown_neighbor_count + flagged_neighbor_count != num_neighbor_mines {
                            return None;
                        }
                        if flagged_neighbor_count == num_neighbor_mines {
                            // remove current cell because there's no more neighbor cells to click
                            *tracked = false;
                            return None;
                        }
                        for (nx, ny) in game_state.neighbors(x, y) {
                            if let Some(cell) = game_state.at(nx, ny) {
                                if cell.visibility == CellVisibility::Unknown {
                                    // bijection_opportunities += 1;
                                    suggested_cell = Event::Flag { pos: (nx, ny) };
                                    // neighbor_cell = Some((x, y));
                                    break;
                                }
                            }
                        }
                    }
                    _ => *tracked = false,
                }
                // println!("returning from BijectionDetection with {:?} with neighbor {:?} when there were {} useful bijection cells to explore from", suggested_cell, neighbor_cell, bijection_opportunities);
                Some(suggested_cell)
            })
            .collect();
        first_cells
    }
    fn update(&mut self, game_state: &GameState, event: Event) {
        if !self.initialized {
            // do initialization step.
            self.cells_of_interest = vec![false; game_state.width * game_state.height];
            self.initialized = true;
        } else {
            // process event to update cells_of_interest, such that useless cells are ignored.
            match event {
                Event::Flag { pos } => {
                    self.cells_of_interest[pos.0 + pos.1 * game_state.width] = false;
                    for (x, y) in game_state.neighbors(pos.0, pos.1) {
                        self.cells_of_interest[x + y * game_state.width] = true;
                    }
                }
                Event::Click { pos } => {
                    self.cells_of_interest[pos.0 + pos.1 * game_state.width] = true;
                    for (x, y) in game_state.neighbors(pos.0, pos.1) {
                        self.cells_of_interest[x + y * game_state.width] = true;
                    }
                }
                _ => {}
            }
        }
    }
}

pub struct ExhaustedCellDetection {
    initialized: bool,
    cells_of_interest: Vec<bool>,
}

impl Strategy for ExhaustedCellDetection {
    fn attempt(&mut self, game_state: &GameState) -> Vec<Event> {
        // let mut neighbor_cell = None;
        // let mut zero_count = 0;

        let first_cells: Vec<Event> = self
            .cells_of_interest
            .par_iter_mut()
            .enumerate()
            .filter_map(|(i, tracked)| {
                if !*tracked {
                    return None;
                }
                let mut suggested_cell = Event::None;

                let (x, y) = (i % game_state.width, i / game_state.width);
                let center_cell = game_state.at(x, y).unwrap();
                let mut suggested_cell = Event::None;
                match center_cell {
                    Cell {
                        visibility: CellVisibility::Empty(num_neighbor_mines),
                        ..
                    } => {
                        // this cell has `neighbors` active mines surrounding it
                        let flagged_neighbor_count: usize = game_state
                            .neighbors(x, y)
                            .iter()
                            .map(|c| {
                                if let Some(Cell {
                                    visibility: CellVisibility::Flagged,
                                    ..
                                }) = game_state.at(c.0, c.1)
                                {
                                    1usize
                                } else {
                                    0usize
                                }
                            })
                            .sum();
                        // this cell also has `flagged` flagged mines surrounding it.
                        if num_neighbor_mines != flagged_neighbor_count {
                            return None;
                        }
                        // if they are the same, then no other unknown cell could contain a mine.
                        for (nx, ny) in game_state.neighbors(x, y).iter() {
                            if let Some(Cell {
                                visibility: CellVisibility::Unknown,
                                ..
                            }) = game_state.at(*nx, *ny)
                            {
                                // zero_count += 1;
                                suggested_cell = Event::Click { pos: (*nx, *ny) };
                                // neighbor_cell = Some((x, y));
                                return Some(suggested_cell);
                            }
                        }

                        // remove current cell because there's no more neighbor cells to click and we didn't break.
                        *tracked = false;
                        return None;
                    }
                    _ => {
                        *tracked = false;
                    }
                }

                // println!("returning from ZeroNeighborDetection with {:?} with neighbor cell {:?} when there were {} useful zero cells to choose from", suggested_cell, neighbor_cell, zero_count);
                Some(suggested_cell)
            })
            .collect();

        first_cells
    }
    fn update(&mut self, game_state: &GameState, event: Event) {
        if !self.initialized {
            // do initialization step.
            self.cells_of_interest = vec![false; game_state.width * game_state.height];
            self.initialized = true;
        } else {
            // process event to update cells_of_interest, such that useless cells are ignored.
            match event {
                Event::Flag { pos } => {
                    self.cells_of_interest[pos.0 + pos.1 * game_state.width] = false;
                    for (x, y) in game_state.neighbors(pos.0, pos.1) {
                        self.cells_of_interest[x + y * game_state.width] = true;
                    }
                }
                Event::Click { pos } => {
                    self.cells_of_interest[pos.0 + pos.1 * game_state.width] = true;
                    for (x, y) in game_state.neighbors(pos.0, pos.1) {
                        self.cells_of_interest[x + y * game_state.width] = true;
                    }
                }
                _ => {}
            }
        }
    }
}

pub struct Solver {
    // add various internal trackers
    strategies: Vec<Box<dyn Strategy>>,
}

impl Solver {
    pub fn new() -> Self {
        let mut solvers: Vec<Box<dyn Strategy>> = Vec::new();
        solvers.push(Box::new(ExhaustedCellDetection {
            initialized: false,
            cells_of_interest: vec![],
        }));
        solvers.push(Box::new(BijectionDetection {
            initialized: false,
            cells_of_interest: vec![],
        }));
        Solver {
            strategies: solvers,
        }
    }

    pub fn next_clicks(&mut self, game_state: &GameState) -> Vec<Event> {
        let events: Vec<Event> = (&mut self.strategies)
            .iter_mut() // mutably iterate over strategies
            .map(|solver| solver.attempt(game_state)) // attempt to solve with each strategy
            .flatten() // flatten to a iterator of events
            .collect::<Vec<Event>>() // collect
            .par_iter() // parallel iterate
            .filter(|e| match &&e {
                // filter out None events
                Event::None => false,
                _ => true,
            })
            .map(move |&e| e) // dereference/copy
            .collect();
        // println!("{}", events.len());
        events
    }

    pub fn update(&mut self, game_state: &GameState, event: Event) {
        for solver in self.strategies.iter_mut() {
            solver.update(&game_state, event);
        }
    }
}
