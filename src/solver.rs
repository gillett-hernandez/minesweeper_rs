use crate::game::*;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Event {
    Click { pos: (usize, usize) },
    Flag { pos: (usize, usize) },
    None,
}

pub trait Strategy<const X: usize, const Y: usize> {
    fn attempt(&self, game_state: &GameState<X, Y>) -> Event;
}

pub struct BijectionDetection {}

impl<const X: usize, const Y: usize> Strategy<X, Y> for BijectionDetection {
    fn attempt(&self, game_state: &GameState<X, Y>) -> Event {
        let mut suggested_cell = Event::None;
        let mut neighbor_cell = None;
        let mut bijection_opportunities = 0;
        'outer: for (y, row) in game_state.field.iter().enumerate() {
            for (x, center_cell) in row.iter().enumerate() {
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
                        if unknown_neighbor_count + flagged_neighbor_count != *num_neighbor_mines {
                            continue;
                        }
                        for (nx, ny) in game_state.neighbors(x, y) {
                            if let Some(cell) = game_state.at(nx, ny) {
                                if cell.visibility == CellVisibility::Unknown {
                                    bijection_opportunities += 1;
                                    suggested_cell = Event::Flag { pos: (nx, ny) };
                                    neighbor_cell = Some((x, y));
                                    break 'outer;
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        // println!("returning from BijectionDetection with {:?} with neighbor {:?} when there were {} useful bijection cells to explore from", suggested_cell, neighbor_cell, bijection_opportunities);
        suggested_cell
    }
}

pub struct ZeroNeighborDetection {}

impl<const X: usize, const Y: usize> Strategy<X, Y> for ZeroNeighborDetection {
    fn attempt(&self, game_state: &GameState<X, Y>) -> Event {
        let mut suggested_cell = Event::None;
        let mut neighbor_cell = None;
        let mut zero_count = 0;

        'outer: for (y, row) in game_state.field.iter().enumerate() {
            for (x, center_cell) in row.iter().enumerate() {
                match center_cell {
                    Cell {
                        visibility: CellVisibility::Empty(neighbors),
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
                        if *neighbors != flagged_neighbor_count {
                            continue;
                        }
                        for (nx, ny) in game_state.neighbors(x, y).iter() {
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
                    }
                    _ => {}
                }
            }
        }

        // println!("returning from ZeroNeighborDetection with {:?} with neighbor cell {:?} when there were {} useful zero cells to choose from", suggested_cell, neighbor_cell, zero_count);
        suggested_cell
    }
}

pub struct Solver<const X: usize, const Y: usize> {
    // add various internal trackers
    strategies: Vec<Box<dyn Strategy<X, Y>>>,
}

impl<const X: usize, const Y: usize> Solver<X, Y> {
    pub fn new() -> Self {
        let mut solvers: Vec<Box<dyn Strategy<X, Y>>> = Vec::new();
        solvers.push(Box::new(BijectionDetection {}));
        solvers.push(Box::new(ZeroNeighborDetection {}));
        Solver {
            strategies: solvers,
        }
    }

    pub fn next_click(&mut self, game_state: &GameState<X, Y>) -> Event {
        let mut first_good_attempt = Event::None;
        for solver in self.strategies.iter() {
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
}
