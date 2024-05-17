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
    future::next_tick, game_engine::unity::il2cpp::{Class, Module, Version}, print_limited, print_message, settings::Gui, Address64, Process
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

#[derive(Class)]
struct Ball4D {
    sinking: bool,
}

#[derive(Class)]
struct Ball5D {
    sinking: bool,
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

                let ball_4d_class = Ball4D::bind(&process, &module, &image).await;
                let ball_5d_class = Ball5D::bind(&process, &module, &image).await;
                print_message("Found Ball classes");

                if ball_4d_class.sinking != ball_5d_class.sinking {
                    print_limited::<150>(&format_args!(
                            "Warning, Ball4D sinking ({}) is not the same as Ball5D sinking ({})!!! 5D levels might be broken",
                            ball_4d_class.sinking,
                            ball_5d_class.sinking));
                }

                let mut old_course_type_ix = 0;
                let mut old_hole_ix = 0;
                let mut old_balls_array: Address64 = Address64::from(0);

                let mut old_ball_sinking = false;

                loop {
                    settings.update();

                    if let Ok(game_state) = game_state_class.read(&process) {
                        let current_course_type_ix = game_state.course_type_ix;
                        let current_current_hole_ix = game_state.hole_ix;
                        let current_balls_array = game_state.balls_array;

                        if current_course_type_ix != old_course_type_ix {
                            print_message("Course type changed!!");
                        }
                        if current_current_hole_ix != old_hole_ix {
                            print_message("Hole changed!!");
                        }
                        if current_balls_array != old_balls_array {
                            print_message("Balls array changed!!");
                        }

                        if let Ok(ball_addr) = process.read::<Address64>(current_balls_array + 0x20) {
                            //print_limited::<64>(&format_args!("Ball addr: {}", ball_addr));
                            if let Ok(ball) = ball_4d_class.read(&process, ball_addr.into()) {
                                let new_ball_sinking = ball.sinking;
                                //print_limited::<64>(&format_args!("Ball sinking: {}", new_ball_sinking));
                                if new_ball_sinking && !old_ball_sinking {
                                    print_message("Ball sunk!!");
                                }
                                old_ball_sinking = new_ball_sinking;
                            }
                        }

                        old_course_type_ix = current_course_type_ix;
                        old_hole_ix = current_current_hole_ix;
                        old_balls_array = current_balls_array;
                    }

                    // TODO: Do something on every tick.
                    next_tick().await;
                }
            })
            .await;
    }
}
