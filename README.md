# Composable Alternatives to Bevy's RunCriteria, States, FixedTimestep

This crate offers alternatives to the Run Criteria, States, and FixedTimestep
scheduling features currently offered by the Bevy game engine.

The ones provided by this crate do not use "looping stages", and can therefore
be combined/composed together elegantly, solving some of the most annoying
usability limitations of the respective APIs in Bevy.

Version Compatibility Table:

|Bevy Version|Crate Version      |
|------------|-------------------|
|`main`      |`bevy_main`        |
|`0.9`       |`main`             |
|`0.9`       |`0.9`              |
|`0.8`       |`0.7`, `0.8`       |
|`0.7`       |`0.4`, `0.5`, `0.6`|
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

## Dependencies and Cargo Feature Flags

The "run conditions" functionality is always enabled, and depends only on
`bevy_ecs`.

The "fixed timestep" functionality is optional (`"fixedtimestep"` cargo
feature) and adds these dependencies:
 - `bevy_time`
 - `bevy_utils`

The "states" functionality is optional (`"states"` cargo feature) and adds
these dependencies:
 - `bevy_utils`

The `"app"` cargo feature enables extension traits that add new builder
methods to `App`, allowing more ergonomic access to the features of this
crate. Adds a dependency on `bevy_app`.

The `"bevy-compat"` feature adds Run Conditions for compatibility with
Bevy's legacy states implementation.

All of the optional cargo features are enabled by default.

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
piping](https://bevy-cheatbook.github.io/programming/system-chaining.html)),
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
immutably. Avoid mutable access or locals in condition systems, unless you are
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

You can use Bevy labels for system ordering, as usual.

**Note:** conditional systems currently only support explicit labels, you cannot use
Bevy's "ordering by function name" syntax. E.g: `.after(another_system)` does *not* work,
you need to create a label.

There is also `ConditionSet` (similar to Bevy `SystemSet`): syntax sugar for
easily applying conditions and labels that are common to many systems:

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
                // use bevy labels for ordering, as usual :)
                // (must be added at the end, after the conditions)
                .label("thing")
                .before("thing2")
        )
        // You can easily apply many conditions to many systems
        // using a `ConditionSet`:
        .add_system_set(ConditionSet::new()
            // all the conditions, and any labels/ordering
            // must be added before adding the systems
            // (helps avoid confusion and accidents)
            // (makes it clear they apply to all systems in the set)
            .run_if(in_multiplayer)
            .run_if(other_condition)
            .label("thing2")
            .after("stuff")
            .with_system(system1)
            .with_system(system2)
            .with_system(system3)
            .into() // Converts into Bevy `SystemSet` (to add to App)
        )
        .run();
}
```

**NOTE:** Due to some limitations with Bevy, `label`/`before`/`after` are
*not* supported on individual systems within a `ConditionSet`. You can only
use labels and ordering on the entire set, to apply them to all member
systems. If some systems need different ordering, just add them individually
with `.add_system`.

## Fixed Timestep

This crate offers a fixed timestep implementation that runs as a separate
Stage in the Bevy schedule. This way, it does not conflict with any other
functionality. You can easily use [run conditions](#run-conditions) and
[states](#states) to control your fixed timestep systems.

It is possible to add multiple "sub-stages" within a fixed timestep, allowing
you to apply `Commands` within a single timestep run. For example, if you want
to spawn entities and then do something with them, on the same tick.

It is also possible to have multiple independent fixed timesteps, should you
need to.

(see `examples/fixedtimestep.rs` for a more complex working example)

```rust
use bevy::prelude::*;
use iyes_loopless::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // add the fixed timestep stage:
        // (in the default position, before CoreStage::Update)
        .add_fixed_timestep(
            Duration::from_millis(250),
            // we need to give it a string name, to refer to it
            "my_fixed_update",
        )
        // add fixed timestep systems:
        .add_fixed_timestep_system(
            "my_fixed_update", 0, // fixed timestep name, sub-stage index
            // it can be a conditional system!
            my_simulation
                .run_if(some_condition)
                .run_in_state(AppState::InGame)
                .after("some_label")
        )
        .run();
}
```

Every frame, the `FixedTimestepStage` will accumulate the time delta. When
it goes over the set timestep value, it will run all the child stages. It
will repeat the sequence of child stages multiple times if needed, if
more than one timestep has accumulated.

### Fixed Timestep Control

You can use the `FixedTimesteps` resource (make sure it is the one from this
crate, not the one from Bevy with the same name) to access information about a
fixed timestep and to control its parameters, like the timestep duration.

```rust
fn timestep_control(mut timesteps: ResMut<FixedTimestep>) {
    // we can access our timestep by name
    let info = timesteps.get_mut("my_fixed_update").unwrap();
    // set a different duration
    info.step = Duration::from_millis(125);
    // pause it
    info.paused = true;
}

/// Print info about the fixed timestep this system runs in
fn debug_fixed(timesteps: Res<FixedTimesteps>) {
    // from within a system that runs inside the fixed timestep,
    // you can use `.get_current`, no need for the timestep name:
    let info = timesteps.get_current().unwrap();
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
`.set_enter_stage(state, stage)` and `.set_exit_stage(state, stage)`.

### State Transition

When the `StateTransitionStage` runs, it will check if a `NextState` resource
exists. If yes, it will remove it and perform a transition:
 - run the "exit stage" (if any) for the current state
 - change the value of `CurrentState`
 - run the "enter stage" (if any) for the next state

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
    App::new()
        .add_plugins(DefaultPlugins)
        // Add our state type
        .add_loopless_state(GameState::MainMenu)
        // If we had more state types, we would add them too...

        // Add a FixedTimestep, cuz we can!
        .add_fixed_timestep(
            Duration::from_millis(250),
            "my_fixed_update",
        )
        .add_fixed_timestep_system(
            "my_fixed_update", 0,
            my_simulation
                .run_in_state(AppState::InGame)
        )

        // Add our various systems
        .add_system(menu_stuff.run_in_state(GameState::MainMenu))
        .add_system(animate.run_in_state(GameState::InGame))

        // On states Enter and Exit
        .add_enter_system(GameState::MainMenu, setup_menu)
        .add_exit_system(GameState::MainMenu, despawn_menu)
        .add_enter_system(GameState::InGame, setup_game)

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
