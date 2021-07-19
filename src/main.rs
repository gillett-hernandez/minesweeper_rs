use std::borrow::Borrow;

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
}

fn check_and_restart_game(
    game_state: &mut GameState,
    solver: &mut Solver,
    saved_valid_clicks: &mut Vec<Event>,
    guess_count: &mut usize,
    num_bombs: usize,
) -> bool {
    let mut restart = false;
    if game_state.game_condition == GameCondition::Lost {
        println!(
            "game lost, with {} remaining mines, {} unknown squares, and {} total guesses",
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
            "game won, with {} remaining mines, {} unknown squares, and {} total guesses",
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
        {
            std::thread::sleep(std::time::Duration::from_secs(3));
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
                opt.num_bombs,
            ) {
                continue 'outer;
            }
            solver.update(&game_state, *event);
        }

        if events.len() == 0 {
            let mut event = Event::None;
            let mut unknown_cells = Vec::new();
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
                        unknown_cells.push((x, y, cell));
                    }
                    _ => {}
                }
            }
            // execute optimal guessing strategy:

            let unknown_cells_float = unknown_cells.len() as f32;
            let remaining_mines = game_state.remaining_mines() as f32;
            let sub = unknown_cells_float - remaining_mines;
            let search_scale = (ramanujan_approximation(unknown_cells_float)
                - ramanujan_approximation(remaining_mines)
                - ramanujan_approximation(sub))
                / 10.0f32.ln();
            if search_scale < 10.0 {
                println!("guessing with search scale = {}", search_scale);
            }
            if search_scale < 4.5 {
                // need to generate combinations and track valid solutions.
                let mut histogram = vec![0; unknown_cells.len()];
                let mut hypothetical = game_state.clone();
                let mut valid_combinations = 0;
                for combination in
                    CombinationIterator::new(unknown_cells.len(), game_state.remaining_mines())
                {
                    // combination is the indices into unknown_cells
                    let mut idx = 0;
                    let mut last_seen = combination[idx];
                    for (i, (x, y, _)) in unknown_cells.iter().enumerate() {
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
                        valid_combinations += 1;
                        for idx in combination.iter() {
                            histogram[*idx] += 1;
                        }
                    }
                }

                // now that the histogram has been tallied, select one of the cells with the lowest probability of being a bomb.
                let mut augmented_histogram = histogram
                    .iter()
                    .enumerate()
                    .map(|e| (e.0, *e.1))
                    .collect::<Vec<(usize, i32)>>();
                augmented_histogram.sort_unstable_by_key(|e| e.1);

                println!(
                    "guessed combinatorically, unknown: {}, remaining mines: {}. pdf was {:?}",
                    unknown_cells.len(),
                    game_state.remaining_mines(),
                    augmented_histogram,
                );

                let index = augmented_histogram[0].0;
                if augmented_histogram[0].1 == 0 {
                    for (idx, ct) in augmented_histogram.iter().skip(1) {
                        if *ct == 0 {
                            saved_valid_clicks.push(Event::Click {
                                pos: (unknown_cells[*idx].0, unknown_cells[*idx].1),
                            });
                        } else {
                            break;
                        }
                    }
                }

                if augmented_histogram[0].1 > 0 {
                    guess_count += 1;
                }
                let (x, y) = (unknown_cells[index].0, unknown_cells[index].1);
                drop(unknown_cells);
                game_state.click(x, y);
                event = Event::Click { pos: (x, y) };
            } else {
                guess_count += 1;
                println!(
                    "guessed randomly, unknown: {}, remaining mines: {}. search scale was {}",
                    unknown_cells.len(),
                    game_state.remaining_mines(),
                    search_scale
                );

                let index = (unknown_cells.len() as f32 * random::<f32>()) as usize;
                let (x, y) = (unknown_cells[index].0, unknown_cells[index].1);
                drop(unknown_cells);
                game_state.click(x, y);
                event = Event::Click { pos: (x, y) };
            }

            if check_and_restart_game(
                &mut game_state,
                &mut solver,
                &mut saved_valid_clicks,
                &mut guess_count,
                opt.num_bombs,
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
