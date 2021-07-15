use minifb::{Key, KeyRepeat, MouseButton, MouseMode, Scale, Window, WindowOptions};
use rayon::prelude::*;
use structopt::StructOpt;
pub  use rand::prelude::*;

mod game;
mod solver;

use game::*;
use solver::*;

pub fn rgb_to_u32(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
struct Opt {
    #[structopt(short, default_value = "50")]
    pub width: usize,

    #[structopt(short, default_value = "50")]
    pub height: usize,

    #[structopt(long, default_value = "22")]
    pub threads: usize,
}

fn main() {
    let opt = Opt::from_args();
    let (width, height) = (opt.width, opt.height);

    let mut window = Window::new(
        "Minesweeper",
        width,
        height,
        WindowOptions {
            scale: Scale::X8,
            ..WindowOptions::default()
        },
    )
    .unwrap();

    let frame_micros = 1000000.0 / 60.0;
    window.limit_update_rate(Some(std::time::Duration::from_micros(frame_micros as u64)));

    const WIDTH: usize = 50;
    const HEIGHT: usize = 50;
    let mut game_state = GameState::<WIDTH, HEIGHT>::new(300);
    let mut window_pixels = vec![0u32; width * height];

    rayon::ThreadPoolBuilder::new()
        .num_threads(opt.threads)
        .build_global()
        .unwrap();
    // let (x, y) = game_state.random_xy_2();
    // game_state.click(x, y);
    // let mut one_off = true;
    let mut solver = Solver::<WIDTH, HEIGHT>::new();
    let mut guess_count = 0;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        // draw phase
        for (y, row) in game_state.field.iter().enumerate() {
            for (x, cell) in row.iter().enumerate() {
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

        match solver.next_click(&game_state) {
            Event::Flag { pos } => game_state.flag(pos.0, pos.1),
            Event::Click { pos } => game_state.click(pos.0, pos.1),

            Event::None => {
                let mut unknown_cells = Vec::new();
                for (x, y, cell) in game_state
                    .field
                    .iter()
                    .enumerate()
                    .map(|(y, e)| e.iter().enumerate().map(move |(x, cell)| (x, y, cell)))
                    .flatten()
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
                guess_count += 1;
                // execute optimal guessing strategy:

                // need to generate combinations.


                println!(
                    "guessed, unknown: {}, remaining mines: {}",
                    game_state.field.iter().flatten().fold(0, |a, b| {
                        a + if b.visibility == CellVisibility::Unknown {
                            1
                        } else {
                            0
                        }
                    }),
                    game_state.remaining_mines()
                );

                let index = (unknown_cells.len() as f32 * random::<f32>()) as usize;
                game_state.click(unknown_cells[index].0, unknown_cells[index].1);
            }
        }

        let mut restart = false;
        if game_state.sub_state == GameCondition::Lost {
            println!("game lost, with {} guesses", guess_count);
            restart = true;
            // one_off = true;
        }
        if game_state.sub_state == GameCondition::Won {
            println!("game won, with {} guesses", guess_count);
            restart = true;
            // one_off = true;
        }
        if restart {
            std::thread::sleep(std::time::Duration::from_secs(30));

            game_state = GameState::<WIDTH, HEIGHT>::new(300);
            guess_count = 0
        }

        // window update
        window
            .update_with_buffer(&window_pixels, width, height)
            .unwrap();
    }
}
