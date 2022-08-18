use bevy::prelude::*;
use iyes_loopless::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(MyTimer::new())
        .insert_resource(GameMode::Multiplayer)
        .init_resource::<ServerState>()
        .add_system(
            notify
                .run_if(in_multiplayer)
                .run_if(on_mytimer)
                // labels and ordering must come at the end
                .after("tick"),
        )
        .add_system(
            tick_mytimer
                .run_if(in_multiplayer)
                .run_if(spacebar_pressed)
                .label("tick")
        )
        .run();
}

/// Condition checking our timer
fn on_mytimer(mytimer: Res<MyTimer>) -> bool {
    mytimer.timer.just_finished()
}

/// Condition checking if we are connected to multiplayer server
fn in_multiplayer(gamemode: Res<GameMode>, connected: Res<ServerState>) -> bool {
    *gamemode == GameMode::Multiplayer && connected.is_active()
}

/// Condition checking if spacebar is pressed
fn spacebar_pressed(kbd: Res<Input<KeyCode>>) -> bool {
    kbd.pressed(KeyCode::Space)
}

fn notify() {
    println!("BAM!");
}

/// Timers gotta be ticked
fn tick_mytimer(mut mytimer: ResMut<MyTimer>, time: Res<Time>) {
    mytimer.timer.tick(time.delta());
}

struct MyTimer {
    timer: Timer,
}

impl MyTimer {
    fn new() -> Self {
        Self {
            timer: Timer::from_seconds(1.0, true),
        }
    }
}

#[derive(PartialEq, Eq)]
pub enum GameMode {
    Singleplayer,
    Multiplayer,
}

#[derive(Default)]
struct ServerState {
    // ...
}

impl ServerState {
    fn is_active(&self) -> bool {
        // placeholder
        true
    }
}
