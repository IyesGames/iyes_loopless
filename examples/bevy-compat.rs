use bevy::prelude::*;
use iyes_loopless::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum BevyState {
    A,
    B,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum IyesState {
    C,
    D,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_state(BevyState::B)
        .add_stage_after(
            CoreStage::PreUpdate,
            "IyesState",
            StateTransitionStage::new(IyesState::D)
        )
        .add_system(
            ping.run_not_in_bevy_state(BevyState::A).run_in_state(IyesState::D)
        )
        .run();
}

fn ping() {
    println!("ping");
}
