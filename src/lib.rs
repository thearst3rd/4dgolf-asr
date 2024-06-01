// We need std for the string class
//#![no_std]

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
use std::str;
use asr::{
    future::next_tick,
    game_engine::unity::{get_scene_name, il2cpp::{Class, Module, Version}, SceneManager},
    print_limited, print_message,
    settings::Gui,
    timer, Address64, Process,
};

asr::async_main!(stable);
//asr::panic_handler!(); // Doesn't work with std

#[derive(Gui)]
struct Settings {
    /// Automatically start timer
    ///
    /// Disable this if you want to do practice without the timer starting.
    #[default = true]
    auto_start: bool,
    /// Only split on the 9th hole
    ///
    /// Causes the autosplitter to only split on the 9th hole of the course, for single course 9-hole runs
    #[default = false]
    split_9_only: bool,
    /// Only split on the 18th hole
    ///
    /// Causes the autosplitter to only split on the 18th hole of the course, for single course 18 hole runs
    #[default = false]
    split_18_only: bool,
    /// Split on new course started
    ///
    /// Causes a new split at the beginning of each course, for easy timekeeping of individual course times
    #[default = false]
    split_course_begin: bool,
    /// Split on the beginning of hole 10
    ///
    /// Causes a new split at the beginning of hole 10, for easy timekeeping of individual challenge course times
    #[default = false]
    split_hole_10_begin: bool,
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

async fn main() {
    // Since we are using std we need to add the panic handler ourselves
    std::panic::set_hook(Box::new(|panic_info| {
        print_message(&panic_info.to_string());
    }));

    let mut settings = Settings::register();

    loop {
        let process = Process::wait_attach("4D Golf.exe").await;
        process
            .until_closes(async {
                let module = Module::wait_attach(&process, Version::V2020).await;
                print_message("Found mono");

                let image = module.wait_get_default_image(&process).await;
                print_message("Found Assembly-CSharp");

                let scene_manager = SceneManager::wait_attach(&process).await;
                print_message("Attached SceneManager");

                let game_state_class = GameState::bind(&process, &module, &image).await;
                print_message("Found GameState class");

                let ball_4d_class = Ball4D::bind(&process, &module, &image).await;
                let ball_5d_class = Ball5D::bind(&process, &module, &image).await;
                print_message("Found Ball classes"); // 😳

                let main_menu_class = MainMenu::bind(&process, &module, &image).await;
                print_message("Found MainMenu class");

                let course_class = image.wait_get_class(&process, &module, "Course").await;
                print_message("Found Course class");
                let course_static_table = course_class.wait_get_static_table(&process, &module).await;
                print_limited::<128>(&format_args!("Course static table: {}", course_static_table));

                let mut old_course_type_ix = 0;
                let mut old_hole_ix = 0;

                let mut old_balls_array: Address64 = Address64::from(0);
                let mut old_ball_sinking = false;

                let mut old_skip_to_game_menu = false;
                let mut old_is_level_loaded = false;

                if let Ok(main_menu) = main_menu_class.read(&process) {
                    old_skip_to_game_menu = main_menu.skip_to_main_menu;
                }

                if let Ok(is_level_loaded) = process.read::<u8>(course_static_table + 0x10) {
                    old_is_level_loaded = is_level_loaded != 0;
                }

                let mut old_is_loading = (!old_balls_array.is_null() && !old_is_level_loaded && !old_ball_sinking) || old_skip_to_game_menu;
                if old_is_loading {
                    timer::pause_game_time();
                }

                let mut old_scene_name = String::from("");

                if let Ok(path) = scene_manager.get_current_scene_path::<128>(&process) {
                    let name = get_scene_name(path.as_bytes());
                    if let Ok(name) = str::from_utf8(name) {
                        old_scene_name.push_str(name);
                        print_limited::<256>(&format_args!("Current Scene: {}", name));
                    }
                }

                if let Ok(game_state) = game_state_class.read(&process) {
                    old_course_type_ix = game_state.course_type_ix;
                    old_hole_ix = game_state.hole_ix;
                    old_balls_array = game_state.balls_array;

                    if let Ok(ball_addr) = process.read::<Address64>(old_balls_array + 0x20) {
                        if old_scene_name.as_str().cmp("Level5D").is_eq() {
                            if let Ok(ball) = ball_5d_class.read(&process, ball_addr.into()) {
                                old_ball_sinking = ball.sinking;
                            }
                        } else {
                            if let Ok(ball) = ball_4d_class.read(&process, ball_addr.into()) {
                                old_ball_sinking = ball.sinking;
                            }
                        }
                    }
                }

                loop {
                    settings.update();

                    let mut current_course_type_ix = old_course_type_ix;
                    let mut current_hole_ix = old_hole_ix;
                    let mut current_balls_array = old_balls_array;
                    let mut current_ball_sinking = false; // Defaults to false unless we properly find everything
                    let mut current_skip_to_game_menu = old_skip_to_game_menu;
                    let mut current_is_level_loaded = old_is_level_loaded;
                    let mut current_scene_name = old_scene_name.clone();

                    if let Ok(path) = scene_manager.get_current_scene_path::<128>(&process) {
                        let name = get_scene_name(path.as_bytes());
                        if let Ok(name) = str::from_utf8(name) {
                            if !name.is_empty() {
                                current_scene_name.clear();
                                current_scene_name.push_str(name);
                            }
                        }
                    }

                    if let Ok(game_state) = game_state_class.read(&process) {
                        current_course_type_ix = game_state.course_type_ix;
                        current_hole_ix = game_state.hole_ix;
                        current_balls_array = game_state.balls_array;

                        if let Ok(ball_addr) = process.read::<Address64>(current_balls_array + 0x20) {
                            if current_scene_name.as_str().cmp("Level5D").is_eq() {
                                if let Ok(ball) = ball_5d_class.read(&process, ball_addr.into()) {
                                    current_ball_sinking = ball.sinking;
                                }
                            } else {
                                if let Ok(ball) = ball_4d_class.read(&process, ball_addr.into()) {
                                    current_ball_sinking = ball.sinking;
                                }
                            }
                        }

                        if let Ok(main_menu) = main_menu_class.read(&process) {
                            current_skip_to_game_menu = main_menu.skip_to_main_menu;
                        }

                        if let Ok(is_level_loaded) = process.read::<u8>(course_static_table + 0x10) {
                            current_is_level_loaded = is_level_loaded != 0;
                        }
                    }

                    if old_scene_name.cmp(&current_scene_name).is_ne() {
                        print_limited::<400>(&format_args!("Current scene changed! {} -> {}", &old_scene_name, &current_scene_name));
                    }

                    if current_hole_ix != old_hole_ix {
                        print_limited::<128>(&format_args!("Hole changed!! {} -> {}", old_hole_ix, current_hole_ix));
                        if current_hole_ix == 9 && settings.split_hole_10_begin {
                            timer::split();
                        }
                    }

                    if current_ball_sinking != old_ball_sinking {
                        print_limited::<128>(&format_args!("Balls sinking changed!! {} -> {}", old_ball_sinking, current_ball_sinking));
                        if current_ball_sinking {
                            if settings.split_18_only {
                                if current_hole_ix == 17 {
                                    timer::split();
                                }
                            } else if settings.split_9_only {
                                if current_hole_ix == 8 || current_hole_ix == 17 {
                                    timer::split();
                                }
                            } else {
                                timer::split();
                            }
                        }
                    }

                    if current_is_level_loaded != old_is_level_loaded {
                        print_limited::<128>(&format_args!("Level loaded changed!! {} -> {}", old_is_level_loaded, current_is_level_loaded));
                    }

                    let current_is_loading = (!current_balls_array.is_null() && !current_is_level_loaded && !current_ball_sinking) || current_skip_to_game_menu;
                    if current_is_loading != old_is_loading {
                        if current_is_loading {
                            print_message("Game loading");
                            timer::pause_game_time();
                        } else {
                            print_limited::<128>(&format_args!("Loading finished"));
                            if current_scene_name.as_str().cmp("MainMenu").is_ne() {
                                if current_hole_ix == 0 && settings.split_course_begin {
                                    timer::split();
                                }
                                if settings.auto_start {
                                    timer::start();
                                }
                            }
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
                    old_scene_name = current_scene_name;

                    // TODO: Do something on every tick.
                    next_tick().await;
                }
            })
            .await;
    }
}
