//! Complex example showcasing all the features together.
//!
//! Shows how our states, fixed timestep, and custom run conditions, can all be used together!
//!
//! Also shows how run conditions could be helpful for Bevy UI button handling!
//!
//! This example has a main menu with two buttons: exit the app and enter the game.
//!
//! How to "play the game": hold spacebar to spawn colorful squares, release spacebar to make them spin! <3

use bevy::prelude::*;
use iyes_loopless::prelude::*;

use bevy::app::AppExit;
use bevy::input::system::exit_on_esc_system;

use std::time::Duration;

use rand::prelude::*;

/// Our Application State
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum GameState {
    MainMenu,
    InGame,
}

fn main() {
    // Add things that should happen on state transitions into their own stages

    let mut enter_menu = SystemStage::parallel();
    enter_menu.add_system(setup_menu);
    enter_menu.add_system(setup_ui_camera);

    let mut exit_menu = SystemStage::parallel();
    exit_menu.add_system(despawn_with::<MainMenu>);
    exit_menu.add_system(despawn_with::<UiCamera>);

    let mut enter_game = SystemStage::parallel();
    enter_game.add_system(setup_game_camera);

    let mut exit_game = SystemStage::parallel();
    exit_game.add_system(despawn_with::<MySprite>);
    exit_game.add_system(despawn_with::<GameCamera>);

    // stage for anything we want to do on a fixed timestep
    let mut fixedupdate = SystemStage::parallel();
    fixedupdate.add_system(
        spawn_sprite
            // only in-game!
            .run_in_state(GameState::InGame)
            // only while the spacebar is pressed
            .run_if(spacebar_pressed),
    );

    App::new()
        .add_plugins(DefaultPlugins)
        // Add the "driver" stage that will perform state transitions
        // After `CoreStage::PreUpdate` is a good place to put it, so that
        // all the states are settled before we run any of our own systems.
        .add_stage_after(
            CoreStage::PreUpdate,
            "TransitionStage",
            StateTransitionStage::new(GameState::MainMenu)
                .with_enter_stage(GameState::MainMenu, enter_menu)
                .with_exit_stage(GameState::MainMenu, exit_menu)
                .with_enter_stage(GameState::InGame, enter_game)
                .with_exit_stage(GameState::InGame, exit_game),
        )
        // If we had more state types, we would add transition stages for them too...
        // Add a FixedTimestep, cuz we can!
        .add_stage_before(
            CoreStage::Update,
            "FixedUpdate",
            FixedTimestepStage::from_stage(Duration::from_millis(125), fixedupdate),
        )
        // menu stuff
        .add_system_set(
            ConditionSet::new()
                .run_in_state(GameState::MainMenu)
                .with_system(exit_on_esc_system)
                .with_system(butt_interact_visual)
                // our menu button handlers
                .with_system(butt_exit.run_if(on_butt_interact::<ExitButt>))
                .with_system(butt_game.run_if(on_butt_interact::<EnterButt>))
                .into()
        )
        // in-game stuff
        .add_system_set(
            ConditionSet::new()
                .run_in_state(GameState::InGame)
                .with_system(back_to_menu_on_esc)
                .with_system(clear_on_del)
                .with_system(spin_sprites.run_if_not(spacebar_pressed))
                .into()
        )
        // our other various systems:
        .add_system(debug_current_state)
        .run();
}

/// Marker for our in-game sprites
#[derive(Component)]
struct MySprite;

/// Marker for the main menu entity
#[derive(Component)]
struct MainMenu;

/// Marker for the UI camera entity
#[derive(Component)]
struct UiCamera;

/// Marker for the main game camera entity
#[derive(Component)]
struct GameCamera;

/// Marker for the "Exit App" button
#[derive(Component)]
struct ExitButt;

/// Marker for the "Enter Game" button
#[derive(Component)]
struct EnterButt;

/// Reset the in-game state when pressing delete
fn clear_on_del(mut commands: Commands, kbd: Res<Input<KeyCode>>) {
    if kbd.just_pressed(KeyCode::Delete) || kbd.just_pressed(KeyCode::Back) {
        commands.insert_resource(NextState(GameState::InGame));
    }
}

/// Transition back to menu on pressing Escape
/// (here we show how to perform a state transition using ResMut)
fn back_to_menu_on_esc(mut next: ResMut<NextState<GameState>>, kbd: Res<Input<KeyCode>>) {
    if kbd.just_pressed(KeyCode::Escape) {
        *next = NextState(GameState::MainMenu);
    }
}

/// We can just access the `CurrentState`, and even use change detection!
fn debug_current_state(state: Res<CurrentState<GameState>>) {
    if state.is_changed() {
        println!("Detected state change to {:?}!", state);
    }
}

/// Condition system for holding the space bar
fn spacebar_pressed(kbd: Res<Input<KeyCode>>) -> bool {
    kbd.pressed(KeyCode::Space)
}

/// Despawn all entities with a given component type
fn despawn_with<T: Component>(mut commands: Commands, q: Query<Entity, With<T>>) {
    for e in q.iter() {
        commands.entity(e).despawn_recursive();
    }
}

/// Spawn a MySprite entity
fn spawn_sprite(mut commands: Commands) {
    let mut rng = thread_rng();
    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                color: Color::rgba(rng.gen(), rng.gen(), rng.gen(), 0.5),
                custom_size: Some(Vec2::new(64., 64.)),
                ..Default::default()
            },
            transform: Transform::from_xyz(
                rng.gen_range(-420.0..420.0),
                rng.gen_range(-420.0..420.0),
                rng.gen_range(0.0..100.0),
            ),
            ..Default::default()
        })
        .insert(MySprite);
}

/// Spawn the UI camera
fn setup_ui_camera(mut commands: Commands) {
    commands.spawn_bundle(UiCameraBundle::default());
}

/// Spawn the game camera
fn setup_game_camera(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}

/// Rotate all the sprites
fn spin_sprites(mut q: Query<&mut Transform, With<MySprite>>, t: Res<Time>) {
    for mut transform in q.iter_mut() {
        transform.rotate(Quat::from_rotation_z(1.0 * t.delta_seconds()));
    }
}

/// Change button color on interaction
fn butt_interact_visual(
    mut query: Query<(&Interaction, &mut UiColor), (Changed<Interaction>, With<Button>)>,
) {
    for (interaction, mut color) in query.iter_mut() {
        match interaction {
            Interaction::Clicked => {
                *color = UiColor(Color::rgb(0.75, 0.75, 0.75));
            }
            Interaction::Hovered => {
                *color = UiColor(Color::rgb(0.8, 0.8, 0.8));
            }
            Interaction::None => {
                *color = UiColor(Color::rgb(1.0, 1.0, 1.0));
            }
        }
    }
}

/// Condition to help with handling multiple buttons
///
/// Returns true when a button identified by a given component is clicked.
fn on_butt_interact<B: Component>(
    query: Query<&Interaction, (Changed<Interaction>, With<Button>, With<B>)>,
) -> bool {
    for interaction in query.iter() {
        if *interaction == Interaction::Clicked {
            return true;
        }
    }

    false
}

/// Handler for the Exit Game button
fn butt_exit(mut ev: EventWriter<AppExit>) {
    ev.send(AppExit);
}

/// Handler for the Enter Game button
/// (here we show how to perform a state transition using Commands)
fn butt_game(mut commands: Commands) {
    commands.insert_resource(NextState(GameState::InGame));
}

/// Construct the main menu UI
fn setup_menu(mut commands: Commands, ass: Res<AssetServer>) {
    let butt_style = Style {
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        padding: Rect::all(Val::Px(8.0)),
        margin: Rect::all(Val::Px(4.0)),
        flex_grow: 1.0,
        ..Default::default()
    };
    let butt_textstyle = TextStyle {
        font: ass.load("Sansation-Regular.ttf"),
        font_size: 24.0,
        color: Color::BLACK,
    };

    let menu = commands
        .spawn_bundle(NodeBundle {
            color: UiColor(Color::rgb(0.5, 0.5, 0.5)),
            style: Style {
                size: Size::new(Val::Auto, Val::Auto),
                margin: Rect::all(Val::Auto),
                align_self: AlignSelf::Center,
                flex_direction: FlexDirection::ColumnReverse,
                //align_items: AlignItems::Stretch,
                justify_content: JustifyContent::Center,
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(MainMenu)
        .id();

    let butt_enter = commands
        .spawn_bundle(ButtonBundle {
            style: butt_style.clone(),
            ..Default::default()
        })
        .with_children(|btn| {
            btn.spawn_bundle(TextBundle {
                text: Text::with_section("Enter Game", butt_textstyle.clone(), Default::default()),
                ..Default::default()
            });
        })
        .insert(EnterButt)
        .id();

    let butt_exit = commands
        .spawn_bundle(ButtonBundle {
            style: butt_style.clone(),
            ..Default::default()
        })
        .with_children(|btn| {
            btn.spawn_bundle(TextBundle {
                text: Text::with_section("Exit Game", butt_textstyle.clone(), Default::default()),
                ..Default::default()
            });
        })
        .insert(ExitButt)
        .id();

    commands
        .entity(menu)
        .push_children(&[butt_enter, butt_exit]);
}
