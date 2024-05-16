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
    future::next_tick, game_engine::unity::il2cpp::{Class, Module, Version}, print_message, settings::Gui, Address64, Process
};

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

async fn main() {
    // TODO: Set up some general state and settings.
    let mut settings = Settings::register();

    loop {
        let process = Process::wait_attach("4D Golf.exe").await;
        process
            .until_closes(async {

                let module = Module::wait_attach(&process, Version::V2020).await;
                print_message("Found mono");

                let image = module.wait_get_default_image(&process).await;
                print_message("Found Assembly-CSharp");

                let game_state_class = GameState::bind(&process, &module, &image).await;
                print_message("Found GameState class");

                let mut current_course_type_ix = 0;
                let mut current_hole_ix = 0;
                let mut current_balls_array: Address64 = Address64::from(0);


                loop {
                    settings.update();

                    if let Ok(game_state) = game_state_class.read(&process) {
                        let new_course_type_ix = game_state.course_type_ix;
                        let new_current_hole_ix = game_state.hole_ix;
                        let new_balls_array = game_state.balls_array;


                        if new_course_type_ix != current_course_type_ix {
                            print_message("Course type changed!!");
                        }
                        if new_current_hole_ix != current_hole_ix {
                            print_message("Hole changed!!");
                        }
                        if new_balls_array != current_balls_array {
                            print_message("Balls array changed!!");
                        }
                        current_course_type_ix = new_course_type_ix;
                        current_hole_ix = new_current_hole_ix;
                        current_balls_array = new_balls_array;
                    }

                    // TODO: Do something on every tick.
                    next_tick().await;
                }
            })
            .await;
    }
}
