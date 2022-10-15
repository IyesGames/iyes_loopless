use bevy::prelude::*;
use iyes_loopless::prelude::*;
use rand::prelude::*;

use std::time::Duration;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)

        // add fixed timestep stage to the default location (before Update)
        .add_fixed_timestep(
            Duration::from_millis(250),
            // give it a label
            "my_fixed_update",
        )

        // add an additional child "sub-stage" under the fixed timestep;
        // this will let us apply Commands within one fixed timestep run
        .add_fixed_timestep_child_stage("my_fixed_update")

        // add a system to our fixed timestep (first sub-stage)
        .add_fixed_timestep_system("my_fixed_update", 0, debug_fixed_timestep)

        // to showcase use of Commands, we will spawn entities in one sub-stage (0) ...
        .add_fixed_timestep_system("my_fixed_update", 0, spawn_entities)
        // ... and mutate their transform in another (1)
        .add_fixed_timestep_system("my_fixed_update", 1, reposition_entities)

        .add_startup_system(setup_camera)
        .add_system(debug_new_count)
        .add_system(random_hiccups)
        .add_system(kbd_control_timestep)
        .add_system(clear_entities)
        .run();
}

#[derive(Component)]
struct MySprite;

/// Spawn a MySprite entity
fn spawn_entities(mut commands: Commands) {
    let mut rng = thread_rng();

    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                color: Color::rgba(rng.gen(), rng.gen(), rng.gen(), 0.5),
                custom_size: Some(Vec2::new(64., 64.)),
                ..Default::default()
            },
            // the `reposition_entities` system will take care of X and Y ;)
            transform: Transform::from_xyz(0.0, 0.0, rng.gen_range(0.0..100.0)),
            ..Default::default()
        })
        .insert(MySprite);
}

/// Move each sprite to a random X,Y position
fn reposition_entities(mut q: Query<&mut Transform, With<MySprite>>) {
    let mut rng = thread_rng();

    for mut transform in q.iter_mut() {
        transform.translation.x = rng.gen_range(-420.0..420.0);
        transform.translation.y = rng.gen_range(-420.0..420.0);
    }
}

/// Every fixed timestep, print info about the timestep parameters
/// (shows how to get it from FixedTimesteps)
fn debug_fixed_timestep(timesteps: Res<FixedTimesteps>) {
    // unwrap: this system will run inside of the fixed timestep
    let info = timesteps.get_current().unwrap();
    println!("Fixed timestep duration: {:?} ({} Hz).", info.timestep(), info.rate());
    println!("Overstepped by {:.2?} ({:.2}%).", info.remaining(), info.overstep() * 100.0);
}

/// Every frame, print if new MySprites have been spawned
fn debug_new_count(q: Query<(), Added<MySprite>>) {
    let new = q.iter().count();
    if new > 0 {
        println!("{:?} new sprites spawned this frame", new);
        println!();
    }
}

/// Randomly decide to sleep for a while to simulate "lag spikes"
///
/// Used to showcase fixed timestep behavior in such situations
fn random_hiccups() {
    let mut rng = rand::thread_rng();

    if rng.gen::<u8>() == 0 {
        std::thread::sleep(Duration::from_millis(500));
    }

    if rng.gen::<u8>() == 255 {
        std::thread::sleep(Duration::from_millis(1000));
    }
}

/// Keypresses for speeding up / slowing down / pausing the fixed timestep
/// (by mutating the FixedTimestepInfo from FixedTimesteps)
fn kbd_control_timestep(
    kbd: Res<Input<KeyCode>>,
    mut timesteps: ResMut<FixedTimesteps>,
) {
    // this system runs outside of the fixed timestep, so we need
    // to get the fixed timestep info by label
    let info = timesteps.get_mut("my_fixed_update").unwrap();

    if kbd.any_just_pressed([KeyCode::Minus, KeyCode::Underline]) {
        info.step = Duration::from_secs_f32(info.step.as_secs_f32() * 0.75);
    }
    if kbd.any_just_pressed([KeyCode::Plus, KeyCode::Equals]) {
        info.step = Duration::from_secs_f32(info.step.as_secs_f32() * 1.25);
    }
    if kbd.just_pressed(KeyCode::Space) {
        info.toggle_pause();
    }
}

/// Clear entities with keypress
fn clear_entities(
    mut commands: Commands,
    kbd: Res<Input<KeyCode>>,
    q: Query<Entity, With<MySprite>>
) {
    if kbd.any_just_pressed([KeyCode::Delete, KeyCode::Back]) {
        for e in q.iter() {
            commands.entity(e).despawn();
        }
    }
}

fn setup_camera(mut commands: Commands) {
    commands.spawn_bundle(Camera2dBundle::default());
}
