use minifb::{Key, KeyRepeat, MouseButton, MouseMode, Scale, Window, WindowOptions};
use rayon::prelude::*;
use structopt::StructOpt;

mod ai;
mod game;

use ai::*;
use game::*;

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
    let mut one_off = true;

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
                        window_pixels[y * width + x] = rgb_to_u32(
                            0,
                            0,
                            (0.0 + 256.0 * (*neighbors as f32 / 8.0).powf(0.4)) as u8,
                        );
                    }
                    Cell {
                        visibility: CellVisibility::Flagged,
                        ..
                    } => {
                        window_pixels[y * width + x] = rgb_to_u32(0, 255, 0);
                    }
                }
            }
        }

        // ai update and gamestate progression phase
        if one_off {
            let (x, y) = game_state.random_xy_2();
            game_state.click(x, y);
            one_off = false;
        }

        let mut restart = false;
        if game_state.sub_state == GameCondition::Lost {
            println!("game lost");
            restart = true;
            one_off = true;
        }
        if game_state.sub_state == GameCondition::Won {
            println!("game won");
            restart = true;
            one_off = true;
        }
        if restart {
            game_state = GameState::<WIDTH, HEIGHT>::new(300);
        }

        // window update
        window
            .update_with_buffer(&window_pixels, width, height)
            .unwrap();
    }
}
