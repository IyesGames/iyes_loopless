# Composable Alternatives to Bevy's RunCriteria, States, FixedTimestep

This crate offers alternatives to the Run Criteria, States, and FixedTimestep
scheduling features currently offered by the Bevy game engine.

The ones provided by this crate do not use "looping stages", and can therefore
be combined/composed together elegantly, solving some of the most annoying
usability limitations of the respective APIs in Bevy.

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

## Run Conditions

This crate provides an alternative to Bevy Run Criteria, called "Run Conditions".

The different name was chosen to avoid naming conflicts and confusion with
the APIs in Bevy. Bevy Run Criteria are pretty deeply integrated into Bevy's
scheduling model, and this crate does not touch/replace them. They are
technically still there and usable.

### How Run Conditions Work?

You can convert any Bevy system into a "conditional system", by calling
`.into_conditional()`. This allows you to add any number of "conditions" on it,
by repeatedly calling the `.run_if` builder method.

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
                .into_conditional()
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
 - `.run_on_event::<T>()`: run if there are events of a given type
 - `.run_if_resource_exists::<T>()`: run if a resource of a given type exists
 - `.run_unless_resource_exists::<T>()`: run if a resource of a given type does not exist
 - `.run_if_resource_equals(value)`: run if the value of a resource equals the one provided
 - `.run_unless_resource_equals(value)`: run if the value of a resource does not equal the one provided

## Fixed Timestep

TODO WIP: Coming soon!

## States

TODO WIP: Coming soon!
