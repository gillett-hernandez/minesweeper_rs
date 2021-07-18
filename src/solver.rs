use rayon::iter::{IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator};

use crate::game::*;

pub trait Strategy<const X: usize, const Y: usize> {
    fn attempt(&mut self, game_state: &GameState<X, Y>) -> Vec<Event>;
    fn update(&mut self, game_state: &GameState<X, Y>, event: Event);
}

pub struct BijectionDetection {
    initialized: bool,
    cells_of_interest: Vec<Option<(usize, usize, Cell)>>,
}

impl<const X: usize, const Y: usize> Strategy<X, Y> for BijectionDetection {
    fn attempt(&mut self, game_state: &GameState<X, Y>) -> Vec<Event> {

        let first_cells: Vec<Event> = self
            .cells_of_interest
            .par_iter_mut()
            .filter_map(|cell| {
                if cell.is_none() {
                    return None;
                }
                let cell0 = cell.unwrap();
                let (x, y) = (cell0.0, cell0.1);
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
                            *cell = None;
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
                    _ => *cell = None,
                }
                // println!("returning from BijectionDetection with {:?} with neighbor {:?} when there were {} useful bijection cells to explore from", suggested_cell, neighbor_cell, bijection_opportunities);
                Some(suggested_cell)
            })
            .collect();
        first_cells
    }
    fn update(&mut self, game_state: &GameState<X, Y>, event: Event) {
        if !self.initialized {
            // do initialization step.
            // self.cells_of_interest = game_state
            //     .field
            //     .iter()
            //     .enumerate()
            //     .map(|(y, e)| e.iter().enumerate().map(move |(x, cell)| (x, y, *cell)))
            //     .flatten()
            //     .collect();
            self.initialized = true;
        } else {
            // process event to update cells_of_interest, such that useless cells are ignored.
            match event {
                Event::Flag { pos } => {
                    let mut delete_idx = None;
                    for (i, (x, y, cell)) in
                        self.cells_of_interest.iter().filter_map(|e| *e).enumerate()
                    {
                        if pos.0 == x && pos.1 == y {
                            delete_idx = Some(i);
                            break;
                        }
                    }
                    if let Some(delete_idx) = delete_idx {
                        self.cells_of_interest.swap_remove(delete_idx);
                    }
                }
                Event::Click { pos } => {
                    self.cells_of_interest.push(Some((
                        pos.0,
                        pos.1,
                        game_state.at(pos.0, pos.1).unwrap(),
                    )));
                    for (x, y) in game_state.neighbors(pos.0, pos.1) {
                        self.cells_of_interest
                            .push(Some((x, y, game_state.at(x, y).unwrap())));
                    }
                }
                _ => {}
            }
            let mut cells_to_delete = Vec::new();
            for (i, _) in self
                .cells_of_interest
                .iter()
                .enumerate()
                .filter(|(i, e)| e.is_none())
            {
                cells_to_delete.push(i);
            }
            for delete_idx in cells_to_delete.iter().rev() {
                self.cells_of_interest.swap_remove(*delete_idx);
            }
        }
    }
}

pub struct ExhaustedCellDetection {
    initialized: bool,
    cells_of_interest: Vec<Option<(usize, usize, Cell)>>,
}

impl<const X: usize, const Y: usize> Strategy<X, Y> for ExhaustedCellDetection {
    fn attempt(&mut self, game_state: &GameState<X, Y>) -> Vec<Event> {
        // let mut neighbor_cell = None;
        // let mut zero_count = 0;

        let first_cells: Vec<Event> = self
            .cells_of_interest
            .par_iter_mut()
            .filter_map(|cell| {
                if cell.is_none() {
                    return None;
                }
                let mut suggested_cell = Event::None;
                let cell0 = cell.unwrap();
                let (x, y) = (cell0.0, cell0.1);
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
                        *cell = None;
                        return None;
                    }
                    _ => {
                        *cell = None;
                    }
                }

                // println!("returning from ZeroNeighborDetection with {:?} with neighbor cell {:?} when there were {} useful zero cells to choose from", suggested_cell, neighbor_cell, zero_count);
                Some(suggested_cell)
            })
            .collect();

        first_cells
    }
    fn update(&mut self, game_state: &GameState<X, Y>, event: Event) {
        if !self.initialized {
            // do initialization step.
            // self.cells_of_interest = game_state
            //     .field
            //     .iter()
            //     .enumerate()
            //     .map(|(y, e)| e.iter().enumerate().map(move |(x, cell)| (x, y, *cell)))
            //     .flatten()
            //     .collect();
            self.initialized = true;
        } else {
            // process event to update cells_of_interest, such that useless cells are ignored.
            match event {
                Event::Flag { pos } => {
                    let mut delete_idx = None;
                    for (i, (x, y, cell)) in
                        self.cells_of_interest.iter().filter_map(|e| *e).enumerate()
                    {
                        if pos.0 == x && pos.1 == y {
                            delete_idx = Some(i);
                            break;
                        }
                    }
                    if let Some(delete_idx) = delete_idx {
                        self.cells_of_interest.swap_remove(delete_idx);
                    }
                }
                Event::Click { pos } => {
                    self.cells_of_interest.push(Some((
                        pos.0,
                        pos.1,
                        game_state.at(pos.0, pos.1).unwrap(),
                    )));
                    for (x, y) in game_state.neighbors(pos.0, pos.1) {
                        self.cells_of_interest
                            .push(Some((x, y, game_state.at(x, y).unwrap())));
                    }
                }
                _ => {}
            }
            let mut cells_to_delete = Vec::new();
            for (i, _) in self
                .cells_of_interest
                .iter()
                .enumerate()
                .filter(|(i, e)| e.is_none())
            {
                cells_to_delete.push(i);
            }
            for delete_idx in cells_to_delete.iter().rev() {
                self.cells_of_interest.swap_remove(*delete_idx);
            }
        }
    }
}

pub struct Solver<const X: usize, const Y: usize> {
    // add various internal trackers
    strategies: Vec<Box<dyn Strategy<X, Y>>>,
}

impl<const X: usize, const Y: usize> Solver<X, Y> {
    pub fn new() -> Self {
        let mut solvers: Vec<Box<dyn Strategy<X, Y>>> = Vec::new();
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

    pub fn next_clicks(&mut self, game_state: &GameState<X, Y>) -> Vec<Event> {
        let events: Vec<Event> = (&mut self.strategies)
            .iter_mut()
            .map(|solver| solver.attempt(game_state))
            .flatten()
            .collect::<Vec<Event>>()
            .par_iter()
            .filter(|e| match e {
                Event::None => false,
                _ => true,
            })
            .map(|e| *e)
            .collect();
        println!("{}", events.len());
        events
    }

    pub fn update(&mut self, game_state: &GameState<X, Y>, event: Event) {
        for solver in self.strategies.iter_mut() {
            solver.update(&game_state, event);
        }
    }
}
