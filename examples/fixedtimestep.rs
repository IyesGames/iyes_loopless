use bevy::prelude::*;
use iyes_loopless::prelude::*;
use rand::prelude::*;

use std::time::Duration;

/// Stage Label for our fixed update stage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, StageLabel)]
struct MyFixedUpdate;

fn main() {
    // prepare our stages for fixed timestep
    // (creating variables to prevent code indentation
    // from drifting too far to the right)

    // to showcase use of Commands, we will spawn entities in one stage ...
    let mut fixed_spawn_stage = SystemStage::parallel();
    fixed_spawn_stage.add_system(spawn_entities);
    // ... and mutate their transform in another
    let mut post_fixed_spawn_stage = SystemStage::parallel();
    post_fixed_spawn_stage.add_system(reposition_entities);

    App::new()
        .add_plugins(DefaultPlugins)
        .add_stage_before(
            CoreStage::Update,
            MyFixedUpdate,
            FixedTimestepStage::new(Duration::from_millis(250))
                .with_stage(fixed_spawn_stage)
                .with_stage(post_fixed_spawn_stage),
        )
        .add_startup_system(setup_camera)
        .add_system(debug_new_count)
        .add_system(random_hiccups)
        .run();
}

#[derive(Component)]
struct MySprite;

/// Every frame, print if new MySprites have been spawned
fn debug_new_count(q: Query<(), Added<MySprite>>) {
    let new = q.iter().count();
    if new > 0 {
        println!("{:?} new sprites spawned this frame", new);
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

fn setup_camera(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}
