# Composable Alternatives to Bevy's RunCriteria, States, FixedTimestep

This crate offers alternatives to the Run Criteria, States, and FixedTimestep
scheduling features currently offered by the Bevy game engine.

The ones provided by this crate do not use "looping stages", and can therefore
be combined/composed together elegantly, solving some of the most annoying
usability limitations of the respective APIs in Bevy.

Version Compatibility Table:

|Bevy Version|Crate Version      |
|------------|-------------------|
|`0.6`       |`0.1`, `0.2`, `0.3`|

## How does this relate to the Bevy Stageless RFC?

This crate draws *very heavy* inspiration from the ["Stageless
RFC"](https://github.com/bevyengine/rfcs/pull/45) proposal for Bevy.

Big thanks to all the authors that have worked on that RFC and the designs
described there.

I am making this crate, because I believe the APIs currently in Bevy are
sorely in need of a usability improvement.

I figured out a way to implement the ideas from the Stageless RFC in a way
that works within the existing framework of current Bevy, without requiring
the complete scheduling API overhaul that the RFC proposes.

This way we can have something usable *now*, while the remaining Stageless
work is still in progress.

## Dependencies

The "run conditions" functionality is always enabled, and depends only on
`bevy_ecs`.

The "fixed timestep" functionality is optional (`"fixedtimestep"` cargo
feature, enabled by default) and adds a dependency on `bevy_core`
(needed for `Res<Time>`).

The "states" functionality is optional (`"states"` cargo feature, enabled
by default) and adds a dependency on `bevy_utils` (to use Bevy's preferred
`HashMap` implementation).

## Run Conditions

This crate provides an alternative to Bevy Run Criteria, called "Run Conditions".

The different name was chosen to avoid naming conflicts and confusion with
the APIs in Bevy. Bevy Run Criteria are pretty deeply integrated into Bevy's
scheduling model, and this crate does not touch/replace them. They are
technically still there and usable.

### How Run Conditions Work?

You can convert any Bevy system into a "conditional system". This allows you
to add any number of "conditions" on it, by repeatedly calling the `.run_if`
builder method.

Each condition is just a Bevy system that outputs (returns) a `bool`.

The conditional system will present itself to
Bevy as a single big system (similar to Bevy's [system
chaining](https://bevy-cheatbook.github.io/programming/system-chaining.html)),
combining the system it was created from with all the condition systems
attached.

When it runs, it will run each condition, and abort if any of them returns
`false`. The main system will run only if all the conditions return `true`.

(see `examples/conditions.rs` for a more complete example)

```rust
use bevy::prelude::*;
use iyes_loopless::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_system(
            notify_server
                .run_if(in_multiplayer)
                .run_if(on_mytimer)
        )
        .run();
}

/// Condition checking our timer
fn on_mytimer(mytimer: Res<MyTimer>) -> bool {
    mytimer.timer.just_finished()
}

/// Condition checking if we are connected to multiplayer server
fn in_multiplayer(gamemode: Res<GameMode>, connected: Res<ServerState>) -> bool {
    *gamemode == GameMode::Multiplayer &&
    connected.is_active()
}

/// Some system that should only run on a timer in multiplayer
fn notify_server(/* ... */) {
    // ...
}
```

It is highly recommended that all your condition systems only access data
immutably. Avoid mutable access or locals in condition systems, unless are
really sure about what you are doing. If you add the same condition to many
systems, it *will run with each one*.

There are also some helper methods for easily adding common kinds of Run Conditions:
 - `.run_if_not`: invert the output of the condition
 - `.run_on_event::<T>()`: run if there are events of a given type
 - `.run_if_resource_exists::<T>()`: run if a resource of a given type exists
 - `.run_unless_resource_exists::<T>()`: run if a resource of a given type does not exist
 - `.run_if_resource_equals(value)`: run if the value of a resource equals the one provided
 - `.run_unless_resource_equals(value)`: run if the value of a resource does not equal the one provided

And if you are using [States](#states):
 - `.run_in_state(state)`
 - `.run_not_in_state(state)`

If you need to use classic Bevy States, you can use these adapters to check them with run conditions:
 - `.run_in_bevy_state(state)`
 - `.run_not_in_bevy_state(state)`

## Fixed Timestep

This crate offers a fixed timestep implementation that uses the Bevy `Stage`
API. You can add a `FixedTimestepStage` to your `App`, wherever you would
like it to run. Typically, a good place would be before `CoreStage::Update`.

It is a container for multiple child stages. You might want to add multiple
child `SystemStage`s, if you'd like to use `Commands` in your systems and
have them applied between each child stage. Or you can just use one if you
don't care. :)

Every frame, the `FixedTimestepStage` will accumulate the time delta. When
it goes over the set timestep value, it will run all the child stages. It
will repeat the sequence of child stages multiple times if needed, if
more than one timestep has accumulated.

(see `examples/fixedtimestep.rs` for a complete working example)

```rust
use bevy::prelude::*;
use iyes_loopless::prelude::*;

fn main() {
    // prepare our stages for fixed timestep
    // (creating variables to prevent code indentation
    // from drifting too far to the right)

    // can create multiple, to use Commands
    let mut fixed_first = SystemStage::parallel();
    // ... add systems to it ...

    let mut fixed_second = SystemStage::parallel();
    // ... add systems to it ...

    App::new()
        .add_plugins(DefaultPlugins)
        // add the fixed timestep stage:
        .add_stage_before(
            CoreStage::Update,
            "my_fixed_update",
            FixedTimestepStage::new(Duration::from_millis(250))
                .with_stage(fixed_first)
                .with_stage(fixed_second)
        )
        // add normal bevy systems:
        .add_startup_system(setup)
        .add_system(do_thing)
        .run();
}
```

Since this implementation does not use Run Criteria, you are free to use
Run Criteria for other purposes. Or better yet: don't, and use the [Run
Conditions](#run-conditions) from this crate instead! ;)

### Fixed Timestep Info

From within your fixed timestep systems, you can use `Res<FixedTimestepInfo>`
to get info about the current fixed timestep parameters, like the timestep
duration and amount of over-step.

This resource is managed by the `FixedTimestepStage`. It will be inserted
before your systems get run, and removed afterwards.

```rust
fn my_fixed_update(info: Res<FixedTimestepInfo>) {
    println!("Fixed timestep duration: {:?} ({} Hz).", info.timestep(), info.rate());
    println!("Overstepped by {:?} ({}%).", info.remaining(), info.overstep() * 100.0);
}
```

## States

(see `examples/menu.rs` for a complete example)

This crate offers a states abstraction that works as follows:

You create one (or more) state types, usually enums, just like when using
Bevy States.

However, here we track states using two resource types:
 - `CurrentState(T)`: the current state you are in
 - `NextState(T)`: insert this (using `Commands`) whenever you want to change state

### Registering the state type

You need to add the state to your `App` using `.add_loopless_state(value)`
with the initial state value. This helper method adds a special stage type
(`StateTransitionStage`) that is responsible for performing state transitions.
By default, it is added before `CoreStage::Update`. If you would like the
transitions to be executed elsewhere in the app schedule, there are other
helper methods that let you specify the position.

For advanced use cases, you could construct and add the `StateTransitionStage`
manually, without the helper method.

### Enter/Exit Systems

You can add enter/exit systems to be executed on state transitions, using
`.add_enter_system(state, system)` and `.add_exit_system(state, system)`.

For advanced scenarios, you could add a custom stage type instead, using
`.add_enter_stage(state, stage)` and `.add_exit_stage(state, stage)`.

### Triggering a Transition

When the `StateTransitionStage` runs, it will check if a `NextState` resource
exists. If yes, it will remove it and perform a transition:
 - run the "exit stage" (if any) for the current state
 - change the value of `CurrentState`
 - run the "enter stage" (if any) for the next state

Please do not manually insert or remove `CurrentState<T>`. It should be managed
entirely by `StateTransitionStage`. It will insert it when it first runs.

If you want to perform a state transition, simply insert a `NextState<T>`.
If you mutate `CurrentState<T>`, you will effectively change state without
running the exit/enter systems (you probably don't want to do this).

Multiple state transitions can be performed in a single frame, if you insert
a new instance of `NextState` from within an exit/enter stage.

### Update systems

For the systems that you want to run every frame, we provide
a `.run_in_state(state)` and `.run_not_in_state(state)` [run
conditions](#run-conditions).

You can add systems anywhere, to any stage (incl. behind [fixed
timestep](#fixed-timestep)), and make them conditional on one or more states,
using those helper methods.

```rust
use bevy::prelude::*;
use iyes_loopless::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum GameState {
    MainMenu,
    InGame,
}

fn main() {
    // stage for anything we want to do on a fixed timestep
    let mut fixedupdate = SystemStage::parallel();
    fixedupdate.add_system(
        fixed_thing
            // only do it in-game
            .run_in_state(GameState::InGame)
    );

    App::new()
        .add_plugins(DefaultPlugins)
        // Add our state type
        .add_loopless_state(GameState::MainMenu)
        // If we had more state types, we would add them too...

        // Add a FixedTimestep, cuz we can!
        .add_stage_before(
            CoreStage::Update,
            "FixedUpdate",
            FixedTimestepStage::from_stage(Duration::from_millis(125), fixedupdate)
        )

        // Add our various systems
        .add_system(menu_stuff.run_in_state(GameState::MainMenu))
        .add_system(animate.run_in_state(GameState::InGame))

        // On states Enter and Exit
        .add_enter_system(&GameState::MainMenu, setup_menu)
        .add_exit_system(&GameState::MainMenu, despawn_menu)
        .add_enter_system(&GameState::InGame, setup_game)

        .run();
}

```

### State transitions under fixed timestep

If you have a state type that you are using for controlling fixed timestep
stuff, you might want state transitions to happen only on fixed timestep
(not just on any frame).

To accomplish that, you can add the `StateTransitionStage` as a child stage
at the beginning of your `FixedTimestepStage`.

The stage types from this crate are composable like that! :) They accept
any stage type.
