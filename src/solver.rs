use crate::game::*;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Event {
    Click { pos: (usize, usize) },
    Flag { pos: (usize, usize) },
    None,
}

pub trait Strategy<const X: usize, const Y: usize> {
    fn attempt(&mut self, game_state: &GameState<X, Y>) -> Event;
    fn update(&mut self, game_state: &GameState<X, Y>, event: Event);
}

pub struct BijectionDetection {
    initialized: bool,
    cells_of_interest: Vec<(usize, usize, Cell)>,
}

impl<const X: usize, const Y: usize> Strategy<X, Y> for BijectionDetection {
    fn attempt(&mut self, game_state: &GameState<X, Y>) -> Event {
        let mut suggested_cell = Event::None;
        let mut neighbor_cell = None;
        let mut bijection_opportunities = 0;

        let mut idx = 0;
        'outer: loop {
            if idx >= self.cells_of_interest.len() {
                break;
            }
            let (x, y, _) = &self.cells_of_interest[idx];
            let center_cell = game_state.at(*x, *y).unwrap();
            match center_cell {
                Cell {
                    visibility: CellVisibility::Empty(num_neighbor_mines),
                    ..
                } => {
                    // detect one to one correspondence of unclicked cells to number of active unflagged mines.
                    let unknown_neighbor_count: usize = game_state
                        .neighbors(*x, *y)
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
                        .neighbors(*x, *y)
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
                        idx += 1;
                        continue;
                    }
                    if flagged_neighbor_count == num_neighbor_mines {
                        // remove current cell because there's no more neighbor cells to click
                        self.cells_of_interest.swap_remove(idx);
                        continue;
                    }
                    for (nx, ny) in game_state.neighbors(*x, *y) {
                        if let Some(cell) = game_state.at(nx, ny) {
                            if cell.visibility == CellVisibility::Unknown {
                                bijection_opportunities += 1;
                                suggested_cell = Event::Flag { pos: (nx, ny) };
                                neighbor_cell = Some((*x, *y));
                                break 'outer;
                            }
                        }
                    }
                }
                _ => {
                    self.cells_of_interest.swap_remove(idx);
                }
            }
        }
        // println!("returning from BijectionDetection with {:?} with neighbor {:?} when there were {} useful bijection cells to explore from", suggested_cell, neighbor_cell, bijection_opportunities);
        suggested_cell
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
                    for (i, (x, y, cell)) in self.cells_of_interest.iter().enumerate() {
                        if pos.0 == *x && pos.1 == *y {
                            delete_idx = Some(i);
                            break;
                        }
                    }
                    if let Some(delete_idx) = delete_idx {
                        self.cells_of_interest.swap_remove(delete_idx);
                    }
                }
                Event::Click { pos } => {
                    self.cells_of_interest.push((
                        pos.0,
                        pos.1,
                        game_state.at(pos.0, pos.1).unwrap(),
                    ));
                    for (x, y) in game_state.neighbors(pos.0, pos.1) {
                        self.cells_of_interest
                            .push((x, y, game_state.at(x, y).unwrap()));
                    }
                }
                _ => {}
            }
        }
    }
}

pub struct ZeroNeighborDetection {
    initialized: bool,
    cells_of_interest: Vec<(usize, usize, Cell)>,
}

impl<const X: usize, const Y: usize> Strategy<X, Y> for ZeroNeighborDetection {
    fn attempt(&mut self, game_state: &GameState<X, Y>) -> Event {
        let mut suggested_cell = Event::None;
        let mut neighbor_cell = None;
        let mut zero_count = 0;

        let mut idx = 0;
        'outer: loop {
            if idx >= self.cells_of_interest.len() {
                break;
            }
            let (x, y, _) = &self.cells_of_interest[idx];
            let center_cell = game_state.at(*x, *y).unwrap();
            match center_cell {
                Cell {
                    visibility: CellVisibility::Empty(num_neighbor_mines),
                    ..
                } => {
                    // this cell has `neighbors` active mines surrounding it
                    let flagged_neighbor_count: usize = game_state
                        .neighbors(*x, *y)
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
                        idx += 1;
                        continue;
                    }
                    // if they are the same, then no other unknown cell could contain a mine.
                    for (nx, ny) in game_state.neighbors(*x, *y).iter() {
                        if let Some(Cell {
                            visibility: CellVisibility::Unknown,
                            ..
                        }) = game_state.at(*nx, *ny)
                        {
                            zero_count += 1;
                            suggested_cell = Event::Click { pos: (*nx, *ny) };
                            neighbor_cell = Some((x, y));
                            break 'outer;
                        }
                    }

                    // remove current cell because there's no more neighbor cells to click and we didn't break.
                    self.cells_of_interest.swap_remove(idx);
                    continue;
                }
                _ => {
                    self.cells_of_interest.swap_remove(idx);
                }
            }
        }

        // println!("returning from ZeroNeighborDetection with {:?} with neighbor cell {:?} when there were {} useful zero cells to choose from", suggested_cell, neighbor_cell, zero_count);
        suggested_cell
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
                    for (i, (x, y, cell)) in self.cells_of_interest.iter().enumerate() {
                        if pos.0 == *x && pos.1 == *y {
                            delete_idx = Some(i);
                            break;
                        }
                    }
                    if let Some(delete_idx) = delete_idx {
                        self.cells_of_interest.swap_remove(delete_idx);
                    }
                }
                Event::Click { pos } => {
                    self.cells_of_interest.push((
                        pos.0,
                        pos.1,
                        game_state.at(pos.0, pos.1).unwrap(),
                    ));
                    for (x, y) in game_state.neighbors(pos.0, pos.1) {
                        self.cells_of_interest
                            .push((x, y, game_state.at(x, y).unwrap()));
                    }
                }
                _ => {}
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
        solvers.push(Box::new(ZeroNeighborDetection {
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

    pub fn next_click(&mut self, game_state: &GameState<X, Y>) -> Event {
        let mut first_good_attempt = Event::None;
        for solver in self.strategies.iter_mut() {
            let event = solver.attempt(game_state);
            match event {
                Event::None => {}
                Event::Click { .. } => {
                    // println!("found event {:?}", event);
                    first_good_attempt = event;
                    break;
                }
                Event::Flag { .. } => {
                    // println!("found event {:?}", event);

                    first_good_attempt = event;
                    break;
                }
            }
        }
        first_good_attempt
    }

    pub fn update(&mut self, game_state: &GameState<X, Y>, event: Event) {
        for solver in self.strategies.iter_mut() {
            solver.update(&game_state, event);
        }
    }
}
