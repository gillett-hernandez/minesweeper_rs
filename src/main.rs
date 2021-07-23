use std::{
    borrow::Borrow,
    collections::{HashMap, HashSet},
};

use minifb::{Key, KeyRepeat, MouseButton, MouseMode, Scale, Window, WindowOptions};
pub use rand::prelude::*;
use rayon::prelude::*;
use structopt::StructOpt;

mod game;
mod lib;
mod solver;

use game::*;
use lib::CombinationIterator;
use solver::*;

pub fn rgb_to_u32(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

pub fn ramanujan_approximation(n: f32) -> f32 {
    n * n.ln() - n
        + (n * (1.0 + 4.0 * n * (1.0 + 2.0 * n))).ln() / 6.0
        + std::f32::consts::PI.ln() / 2.0
}

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
struct Opt {
    #[structopt(short, default_value = "100")]
    pub width: usize,

    #[structopt(short, default_value = "100")]
    pub height: usize,

    #[structopt(long)]
    pub silence: bool,

    #[structopt(long, default_value = "22")]
    pub threads: usize,

    #[structopt(long, default_value = "2")]
    pub skip: usize,

    #[structopt(long, default_value = "1500")]
    pub num_bombs: usize,

    #[structopt(long, default_value = "0")]
    pub delay_ms: usize,
}

fn check_and_restart_game(
    game_state: &mut GameState,
    solver: &mut Solver,
    saved_valid_clicks: &mut Vec<Event>,
    guess_count: &mut usize,
    wins: &mut (usize, usize),
    num_bombs: usize,
    opt: &Opt,
) -> bool {
    let mut restart = false;
    if game_state.game_condition == GameCondition::Lost {
        println!(
            "game lost, with {} remaining mines, {} unknown squares, and {} total guesses\n\n\n",
            game_state.remaining_mines(),
            game_state
                .field
                .iter()
                .filter(|c| c.visibility == CellVisibility::Unknown)
                .count(),
            guess_count
        );
        restart = true;

        // one_off = true;
    }
    if game_state.game_condition == GameCondition::Won {
        for i in game_state
            .field
            .iter()
            .enumerate()
            .filter_map(|(i, e)| match e.visibility {
                CellVisibility::Unknown => Some(i),
                _ => None,
            })
            .collect::<Vec<usize>>()
        {
            let (x, y) = (i % game_state.width, i / game_state.width);
            game_state.click(x, y);
            if game_state.game_condition == GameCondition::Lost {
                println!("{}, {} caused the game to lose after it had already been detected as won. contents were {:?}", x, y, game_state.at(x, y).unwrap());
                break;
            }
        }
        println!(
            "game won, with {} remaining mines, {} unknown squares, and {} total guesses\n\n\n",
            game_state.remaining_mines(),
            game_state
                .field
                .iter()
                .filter(|c| c.visibility == CellVisibility::Unknown)
                .count(),
            guess_count
        );
        restart = true;
        // one_off = true;
    }
    if restart {
        if game_state.remaining_mines() as f32 / (num_bombs as f32) < 0.03
            && game_state.game_condition == GameCondition::Lost
            && !opt.silence
        {
            std::thread::sleep(std::time::Duration::from_millis(opt.delay_ms as u64));
        }
        if game_state.game_condition == GameCondition::Won {
            wins.0 += 1;
            wins.1 += 1;
        } else if (game_state.width * game_state.height
            - game_state
                .field
                .iter()
                .filter(|c| c.visibility == CellVisibility::Unknown)
                .count())
            > 10
        {
            // only count attempts that have more than 10 clicked/flagged cells. removes games that end really quickly from consideration
            wins.1 += 1;
        }
        if wins.1 > 0 {
            println!("winrate: {}", wins.0 as f32 / wins.1 as f32);
        }

        *game_state = GameState::new(game_state.width, game_state.height, num_bombs);
        *solver = Solver::new();
        *guess_count = 0;
        saved_valid_clicks.clear();
        true
    } else {
        false
    }
}

fn educated_guess(
    game_state: &mut GameState,
    guess_count: &mut usize,
    saved_valid_clicks: &mut Vec<Event>,
) -> Event {
    let mut event = Event::None;
    let mut unknown_cells = Vec::new();
    let (width, _) = (game_state.width, game_state.height);
    for (x, y, cell) in game_state
        .field
        .iter()
        .enumerate()
        .map(|(i, e)| (i % width, i / width, e))
    {
        match cell {
            Cell {
                visibility: CellVisibility::Unknown,
                ..
            } => {
                unknown_cells.push((x, y));
            }
            _ => {}
        }
    }
    // execute optimal guessing strategy:
    // partition unknown cells into territory based groups.
    // iterate through all possible partitions of bomb counts for the given number of groups.
    // then for each partition and presupposition of bomb counts per group, iterate through all combinations of bomb positions, checking for hint consistency between current board and hypothetical board.
    // if the hypothetical is consistent with the current board, track probabilities
    // after all this is over, select all the cells that had a 0 probability of having a bomb.

    let mut groups: Vec<HashSet<(usize, usize)>> = Vec::new();
    groups.push(HashSet::new());
    let mut ungrouped_cells: HashSet<(usize, usize)> = unknown_cells.iter().cloned().collect();
    let mut check_queue = Vec::new();
    let mut group_idx = 0;
    // while there are any ungrouped cells
    while ungrouped_cells.len() > 0 {
        let first_ungrouped_cell = ungrouped_cells.iter().take(1).next().unwrap();
        groups[group_idx].insert(*first_ungrouped_cell);
        check_queue.push(*first_ungrouped_cell);
        // grow the currently active group by consuming the check queue
        while check_queue.len() > 0 {
            let cell = check_queue.pop().unwrap();
            let neighborhood = game_state
                .neighbors(cell.0, cell.1)
                .iter()
                .map(|e| game_state.neighbors(e.0, e.1))
                .flatten()
                .collect::<HashSet<_>>();
            for neighbor in neighborhood.iter() {
                if *neighbor == cell {
                    continue;
                }
                let neighbor_cell = game_state.at(neighbor.0, neighbor.1).unwrap();
                if ungrouped_cells.contains(neighbor) {
                    match neighbor_cell.visibility {
                        CellVisibility::Unknown => {
                            groups[group_idx].insert(*neighbor);
                            check_queue.push(*neighbor);
                            ungrouped_cells.remove(neighbor);
                        }
                        _ => {}
                    }
                }
            }
        }
        // check queue must be empty now, so continue to the next ungrouped cell and start a new group.

        if ungrouped_cells.len() > 0 {
            group_idx += 1;
            groups.push(HashSet::new());
        }
    }
    let remaining_mines = game_state.remaining_mines();

    println!(
        "partitioned {} bombs into {} unknown_cells: {} groups total, {:?} distribution",
        remaining_mines,
        unknown_cells.len(),
        groups.len(),
        groups.iter().map(|e| e.len()).collect::<Vec<usize>>()
    );
    // let remaining_mines_float = game_state.remaining_mines() as f32;
    let mut histogram = HashMap::new();
    for (x, y) in unknown_cells.iter() {
        histogram.insert(x + y * width, 0usize);
    }

    let empty_iter: Vec<Vec<usize>> = vec![vec![]];
    let iter: Box<dyn Iterator<Item = Vec<usize>>>;
    if groups.len() == 1 {
        iter = Box::new(empty_iter.iter().cloned());
    } else {
        iter = Box::new(CombinationIterator::new(remaining_mines, groups.len() - 1));
    }

    let search_scale = (ramanujan_approximation(remaining_mines as f32)
        - ramanujan_approximation(groups.len() as f32 - 1.0)
        - ramanujan_approximation(1.0 + remaining_mines as f32 - groups.len() as f32))
        / 10.0f32.ln();
    if search_scale < 3.0 {
        println!("searching partitions and combinations.");
        let mut collected = iter.collect::<Vec<_>>();
        let collected_hashmaps: HashMap<_, _> = collected
            .par_iter_mut()
            .map(|partition| {
                partition.insert(0, 0);
                partition.push(remaining_mines);

                let mine_counts: Vec<_> = partition.windows(2).map(|w| w[1] - w[0]).collect();
                if groups
                    .iter()
                    .enumerate()
                    .any(|(i, e)| mine_counts[i] > e.len())
                {
                    return HashMap::new();
                }
                // println!("");
                let mut local_histogram = HashMap::new();
                for (group_idx, group) in groups.iter().enumerate() {
                    let remaining_mines = mine_counts[group_idx];
                    let remaining_mines_float = remaining_mines as f32;
                    let unknown_cells: Vec<_> = group.iter().cloned().collect();
                    let unknown_cells_float = unknown_cells.len() as f32;
                    let sub = unknown_cells_float - remaining_mines_float;

                    // calculate order of magnitude of combinations that need to be searched.
                    let search_scale = (ramanujan_approximation(unknown_cells_float)
                        - ramanujan_approximation(remaining_mines_float)
                        - ramanujan_approximation(sub))
                        / 10.0f32.ln();

                    if search_scale < 3.0 {
                        // need to generate combinations and track valid solutions.
                        print!(".");
                        let mut hypothetical = game_state.clone();

                        for combination in
                            CombinationIterator::new(unknown_cells.len(), remaining_mines)
                        {
                            // combination is the indices into local unknown_cells
                            let mut idx = 0;
                            let mut last_seen = combination[idx];
                            for (i, (x, y)) in unknown_cells.iter().enumerate() {
                                if i == last_seen {
                                    idx += 1;
                                    if idx < combination.len() {
                                        last_seen = combination[idx];
                                    }

                                    hypothetical.at_mut(*x, *y).unwrap().state = CellState::Mine;
                                    continue;
                                } else {
                                    hypothetical.at_mut(*x, *y).unwrap().state = CellState::Empty;
                                }
                            }
                            if game_state.validate(&hypothetical) {
                                // if game_state and hypothetical were compatible, it means that either state could have resulted in the current visible appearance.
                                // for each bomb position in the hypothetical, add 1 to its position in the histogram

                                for idx in combination.iter() {
                                    let cell = unknown_cells[*idx];
                                    *local_histogram.entry(cell.1 * width + cell.0).or_insert(0) +=
                                        1;
                                }
                            }
                        }
                    } else {
                        print!("#");
                        for (i, cell) in unknown_cells.iter().enumerate() {
                            *local_histogram
                                .entry(cell.1 * width + cell.0)
                                .or_insert(0usize) += 1;
                        }
                    }
                }
                local_histogram
            })
            .reduce(
                || HashMap::new(),
                |mut a, b| {
                    b.iter().for_each(|e| *a.entry(*e.0).or_insert(0) += e.1);
                    a
                },
            );
        // fold parallel hashmaps into main histogram
        collected_hashmaps
            .iter()
            .for_each(|e| *histogram.entry(*e.0).or_insert(0) += e.1);
    } else {
        histogram.par_iter_mut().for_each(|(k, v)| *v += 1);
    }

    // now that the histogram has been tallied, select one of the cells with the lowest probability of being a bomb.
    let mut augmented_histogram: Vec<(usize, usize)> =
        histogram.iter().map(|(k, v)| (*k, *v)).collect();
    augmented_histogram.sort_unstable_by_key(|e| e.1);

    if augmented_histogram.len() < 100 {
        println!(
            "guessed combinatorically, unknown: {}, remaining mines: {}. pdf was {:?}",
            unknown_cells.len(),
            game_state.remaining_mines(),
            augmented_histogram,
        );
    } else {
        println!("guessing randomly");
    }

    let index = augmented_histogram[0].0;
    // if we have some nonzero number of cells that have been combinatorically deduced to not be mines,
    if augmented_histogram[0].1 == 0 {
        // add all but the 1st to a list so that they can be clicked on later without wasting additional computational effort.
        for (idx, ct) in augmented_histogram.iter().skip(1) {
            if *ct == 0 {
                saved_valid_clicks.push(Event::Click {
                    pos: (*idx % width, *idx / width),
                });
            } else {
                break;
            }
        }
    }

    if augmented_histogram[0].1 > 0 {
        // since the chance of the picked entry being a bomb is nonzero, add 1 to the guess count to indicate that actual guesses (rather than combinatoric deductions) are being performed
        *guess_count += 1;
    }
    let (x, y) = (index % width, index / width);
    drop(unknown_cells);
    event = Event::Click { pos: (x, y) };

    event
}

fn main() {
    let opt = Opt::from_args();
    let (width, height) = (opt.width, opt.height);

    let mut window = None;

    if !opt.silence {
        window = Some(
            Window::new(
                "Minesweeper",
                width,
                height,
                WindowOptions {
                    scale: Scale::X8,
                    ..WindowOptions::default()
                },
            )
            .unwrap(),
        );
    }

    let frame_micros = 1000000.0 / 144.0;
    &mut window
        .as_mut()
        .map(|w| w.limit_update_rate(Some(std::time::Duration::from_micros(frame_micros as u64))));

    let mut game_state = GameState::new(width, height, opt.num_bombs);
    let mut window_pixels = vec![0u32; width * height];

    rayon::ThreadPoolBuilder::new()
        .num_threads(opt.threads)
        .build_global()
        .unwrap();
    // let (x, y) = game_state.random_xy_2();
    // game_state.click(x, y);
    // let mut one_off = true;
    let mut solver = Solver::new();
    let mut guess_count = 0;
    let mut frame = 0;
    let framerule = opt.skip;
    let mut saved_valid_clicks = Vec::new();
    let mut wins = (0, 0);

    'outer: loop {
        if let Some(w) = &window {
            if !(w.is_open() && !w.is_key_down(Key::Escape)) {
                break;
            }
        }
        frame += 1;
        // draw phase
        if !opt.silence {
            for (i, cell) in game_state.field.iter().enumerate() {
                let (x, y) = (i % width, i / width);
                match cell {
                    Cell {
                        visibility: CellVisibility::Unknown,
                        ..
                    } => {
                        window_pixels[y * width + x] = rgb_to_u32(128, 128, 128);
                    }
                    Cell {
                        visibility: CellVisibility::Empty(neighbors),
                        ..
                    } => {
                        // println!("blue is {}", (*neighbors as f32 * 256.0 / 8.0));

                        window_pixels[y * width + x] = match neighbors {
                            0 => rgb_to_u32(0, 0, 0),
                            1 => rgb_to_u32(0, 64, 64),
                            2 => rgb_to_u32(0, 64, 127),
                            3 => rgb_to_u32(80, 127, 255),
                            4 => rgb_to_u32(80, 127, 0),
                            5 => rgb_to_u32(80, 180, 127),
                            6 => rgb_to_u32(160, 180, 180),
                            7 => rgb_to_u32(160, 255, 255),
                            _ => rgb_to_u32(255, 255, 255),
                        };
                    }
                    Cell {
                        visibility: CellVisibility::Flagged,
                        ..
                    } => {
                        window_pixels[y * width + x] = rgb_to_u32(255, 0, 0);
                    }
                }
            }
        }

        // ai update and gamestate progression phase

        let mut events = solver.next_clicks(&game_state);
        events.append(&mut saved_valid_clicks);
        for event in events.iter() {
            match event {
                Event::Flag { pos } => game_state.flag(pos.0, pos.1),
                Event::Click { pos } => game_state.click(pos.0, pos.1),

                Event::None => {}
            }

            if check_and_restart_game(
                &mut game_state,
                &mut solver,
                &mut saved_valid_clicks,
                &mut guess_count,
                &mut wins,
                opt.num_bombs,
                &opt,
            ) {
                continue 'outer;
            }
            solver.update(&game_state, *event);
        }

        if events.len() == 0 {
            let event = educated_guess(&mut game_state, &mut guess_count, &mut saved_valid_clicks);

            match event {
                Event::Flag { pos } => game_state.flag(pos.0, pos.1),
                Event::Click { pos } => game_state.click(pos.0, pos.1),

                Event::None => {}
            }

            if check_and_restart_game(
                &mut game_state,
                &mut solver,
                &mut saved_valid_clicks,
                &mut guess_count,
                &mut wins,
                opt.num_bombs,
                &opt,
            ) {
                continue 'outer;
            }
            solver.update(&game_state, event);
        }

        // window update
        if let Some(window) = &mut window {
            if frame % framerule == 0 {
                window
                    .update_with_buffer(&window_pixels, width, height)
                    .unwrap();
                frame %= framerule;
            }
        }
    }
}
