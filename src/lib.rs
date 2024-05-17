#![no_std]

/**
 * Autosplitter for 4D Golf
 * 4D Golf by CodeParade
 * Autosplitter by thearst3rd and Derko
 *
 * This is my first time using Rust and I kinda have no idea what I'm doing :) I have been heavily referencing CryZe's
 * autosplitter for lunistice as well as the asr docs.
 *
 * https://github.com/CryZe/lunistice-auto-splitter/blob/master/src/lib.rs
 * https://livesplit.org/asr/asr/
 */


use asr::{
    future::{next_tick, retry}, game_engine::unity::il2cpp::{Class, Module, Version}, print_limited, print_message, settings::Gui, timer, Address64, Process
};

static EXECUTABLE_NAMES: [&str; 2] = [
    "4D Golf.exe", // Windows
    "4DGolf.x86_64" // Linux
];

asr::async_main!(stable);
asr::panic_handler!();

#[derive(Gui)]
struct Settings {
    /// My Setting
    #[default = true]
    my_setting: bool,
    // TODO: Change these settings.
}

#[derive(Class)]
struct GameState {
    #[rename = "courseTypeIx"]
    #[static_field]
    course_type_ix: i32,
    #[rename = "holeIx"]
    #[static_field]
    hole_ix: i32,
    #[rename = "balls"]
    #[static_field]
    balls_array: Address64,
}

#[derive(Class)]
struct Ball4D {
    sinking: bool,
}

#[derive(Class)]
struct Ball5D {
    sinking: bool,
}


#[derive(Class)]
struct MainMenu {
    #[rename = "skipToGameMenu"]
    #[static_field]
    skip_to_main_menu: bool,
}

// Can you do this lol
#[derive(Class)]
struct Course {
    #[rename = "loadedLevel"]
    #[static_field]
    loaded_level: Address64,
}

async fn main() {
    // TODO: Set up some general state and settings.
    let mut settings = Settings::register();

    loop {
        let process = retry(|| {
            EXECUTABLE_NAMES.into_iter().find_map(Process::attach)
        }).await;
        process
            .until_closes(async {
                print_message("Trying to find mono...");
                let module = Module::wait_attach(&process, Version::V2020).await;
                print_message("Found mono");

                let image = module.wait_get_default_image(&process).await;
                print_message("Found Assembly-CSharp");

                let game_state_class = GameState::bind(&process, &module, &image).await;
                print_message("Found GameState class");

                let ball_4d_class = Ball4D::bind(&process, &module, &image).await;
                let ball_5d_class = Ball5D::bind(&process, &module, &image).await;
                print_message("Found Ball classes");

                if ball_4d_class.sinking != ball_5d_class.sinking {
                    print_limited::<150>(&format_args!(
                            "Warning, Ball4D sinking ({}) is not the same as Ball5D sinking ({})!!! 5D levels might be broken",
                            ball_4d_class.sinking,
                            ball_5d_class.sinking));
                }

                let main_menu_class = MainMenu::bind(&process, &module, &image).await;
                print_message("Found MainMenu class");

                let course_class = Course::bind(&process, &module, &image).await;
                print_message("Found Course class");

                let mut old_course_type_ix = 0;
                let mut old_hole_ix = 0;

                let mut old_balls_array: Address64 = Address64::from(0);
                let mut old_ball_sinking = false;

                let mut old_skip_to_game_menu = false;
                let mut old_is_level_loaded = false;

                if let Ok(game_state) = game_state_class.read(&process) {
                    old_course_type_ix = game_state.course_type_ix;
                    old_hole_ix = game_state.hole_ix;
                    old_balls_array = game_state.balls_array;

                    if let Ok(ball_addr) = process.read::<Address64>(old_balls_array + 0x20) {
                        if let Ok(ball) = ball_4d_class.read(&process, ball_addr.into()) {
                            old_ball_sinking = ball.sinking;
                        }
                    }
                }

                if let Ok(main_menu) = main_menu_class.read(&process) {
                    old_skip_to_game_menu = main_menu.skip_to_main_menu;
                }

                if let Ok(is_level_loaded) = process.read::<u8>(course_class.static_table + 0x10) {
                    old_is_level_loaded = is_level_loaded != 0;
                }

                let mut old_is_loading = (!old_balls_array.is_null() && !old_is_level_loaded && !old_ball_sinking) || old_skip_to_game_menu;
                if old_is_loading {
                    timer::pause_game_time();
                }

                loop {
                    settings.update();

                    let mut current_course_type_ix = old_course_type_ix;
                    let mut current_hole_ix = old_hole_ix;
                    let mut current_balls_array = old_balls_array;
                    let mut current_ball_sinking = false; // Defaults to false unless we properly find everything
                    let mut current_skip_to_game_menu = old_skip_to_game_menu;
                    let mut current_is_level_loaded = old_is_level_loaded;

                    if let Ok(game_state) = game_state_class.read(&process) {
                        current_course_type_ix = game_state.course_type_ix;
                        current_hole_ix = game_state.hole_ix;
                        current_balls_array = game_state.balls_array;

                        if let Ok(ball_addr) = process.read::<Address64>(current_balls_array + 0x20) {
                            if let Ok(ball) = ball_4d_class.read(&process, ball_addr.into()) {
                                current_ball_sinking = ball.sinking;
                            }
                        }

                        if let Ok(main_menu) = main_menu_class.read(&process) {
                            current_skip_to_game_menu = main_menu.skip_to_main_menu;
                        }

                        if let Ok(is_level_loaded) = process.read::<u8>(course_class.static_table + 0x10) {
                            current_is_level_loaded = is_level_loaded != 0;
                        }
                    }

                    if current_hole_ix != old_hole_ix {
                        print_limited::<128>(&format_args!("Hole changed!! {} -> {}", old_hole_ix, current_hole_ix));
                    }

                    if current_ball_sinking != old_ball_sinking {
                        print_limited::<128>(&format_args!("Balls sinking changed!! {} -> {}", old_ball_sinking, current_ball_sinking));
                        if current_ball_sinking {
                            timer::split();
                        }
                    }

                    if current_is_level_loaded != old_is_level_loaded {
                        print_limited::<128>(&format_args!("Level loaded changed!! {} -> {}", old_is_level_loaded, current_is_level_loaded));
                        if current_is_level_loaded {
                            timer::start();
                            timer::resume_game_time();
                        }
                    }

                    let current_is_loading = (!current_balls_array.is_null() && !current_is_level_loaded && !current_ball_sinking) || current_skip_to_game_menu;
                    if current_is_loading != old_is_loading {
                        if current_is_loading {
                            print_message("Game loading");
                            timer::pause_game_time();
                        } else {
                            print_message("Loading finished");
                            timer::resume_game_time();
                        }
                    }

                    old_course_type_ix = current_course_type_ix;
                    old_hole_ix = current_hole_ix;
                    old_balls_array = current_balls_array;
                    old_ball_sinking = current_ball_sinking;
                    old_skip_to_game_menu = current_skip_to_game_menu;
                    old_is_level_loaded = current_is_level_loaded;
                    old_is_loading = current_is_loading;

                    // TODO: Do something on every tick.
                    next_tick().await;
                }
            })
            .await;
    }
}
