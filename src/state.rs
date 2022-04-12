use bevy_ecs::schedule::{Stage, StateData};
use bevy_ecs::world::World;
use bevy_utils::HashMap;

/// This will be available as a resource, indicating the current state
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentState<T>(pub T);

/// When you want to change state, insert this as a resource
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NextState<T>(pub T);

/// This stage serves as the "driver" for states of a given type
///
/// It will perform state transitions, based on the values of [`NextState`]
/// and [`CurrentState`]. You can provide enter/exit stages, to specify what
/// to do when entering or exiting a given state. You do not have to provide
/// an enter or exit stage for every state value, just the ones you care about.
///
/// When this stage runs, it will check if a [`NextState`] resource exists.
/// If it does, and its value is different from what's in [`CurrentState`],
/// this stage will perform a state transition:
///  1. remove the `NextState` resource
///  2. run the exit stage (if any) for the current state
///  3. change the value of `CurrentState`
///  4. run the enter stage (if any) for the next stage
///
/// This stage manages the [`CurrentState`] resource. It will initialize it if it
/// doesn't exist, and update it on state transitions. Please don't mutate that
/// resource manually. Insert a `NextState` resource (you can do it via `Commands`)
/// to change state.
///
/// A single run of this stage can execute multiple transitions, if you insert a
/// new instance of `NextState` from within the exit or enter stages.
pub struct StateTransitionStage<T: StateData> {
    /// The enter schedules of each state
    enter_stages: HashMap<T, Box<dyn Stage>>,
    /// The exit schedules of each state
    exit_stages: HashMap<T, Box<dyn Stage>>,
    /// The starting state value
    default: T,
}

impl<T: StateData> StateTransitionStage<T> {
    /// Create a new transitions stage for the given state type
    ///
    /// The provided value is the one that will be used to initialize the
    /// `CurrentState<T>` resource if it is missing.
    pub fn new(default: T) -> Self {
        Self {
            enter_stages: Default::default(),
            exit_stages: Default::default(),
            default,
        }
    }

    /// Provide the stage to run when entering the given state
    pub fn set_enter_stage<S: Stage>(&mut self, state: T, stage: S) {
        self.enter_stages.insert(state, Box::new(stage));
    }

    pub fn set_exit_stage<S: Stage>(&mut self, state: T, stage: S) {
        self.exit_stages.insert(state, Box::new(stage));
    }

    pub fn with_enter_stage<S: Stage>(mut self, state: T, stage: S) -> Self {
        self.set_enter_stage(state, stage);
        self
    }

    pub fn with_exit_stage<S: Stage>(mut self, state: T, stage: S) -> Self {
        self.set_exit_stage(state, stage);
        self
    }
}

impl<T: StateData> Stage for StateTransitionStage<T> {
    fn run(&mut self, world: &mut World) {
        loop {
            let current = if let Some(res) = world.get_resource::<CurrentState<T>>() {
                res.0.clone()
            } else {
                // first run; gotta run the initial enter stage
                world.insert_resource(CurrentState(self.default.clone()));
                if let Some(stage) = self.enter_stages.get_mut(&self.default) {
                    stage.run(world);
                }
                world
                    .get_resource_or_insert_with(|| CurrentState(self.default.clone()))
                    .0
                    .clone()
            };

            let next = world.remove_resource::<NextState<T>>();

            if let Some(NextState(next)) = next {
                if let Some(stage) = self.exit_stages.get_mut(&current) {
                    stage.run(world);
                }

                world.insert_resource(CurrentState(next.clone()));

                if let Some(stage) = self.enter_stages.get_mut(&next) {
                    stage.run(world);
                }
            } else {
                break;
            }
        }
    }
}
