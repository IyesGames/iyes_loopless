use bevy_ecs::schedule::{Stage, StateData};
use bevy_ecs::system::{lifetimeless::SRes, SystemState};
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
/// When this stage runs, it will check if the [`NextState`] resource has been
/// changed. If yes, this stage will perform a state transition:
///  2. run the exit stage (if any) for the current state
///  3. change the value of `CurrentState`
///  4. run the enter stage (if any) for the next stage
///
/// This stage manages the [`CurrentState`]/[`NextState`] resource. It will insert
/// them if they don't exist, and update the value on state transitions.
///
/// To trigger a state transition, change the [`NextState`] value. You can either mutate it
/// directly or re-insert it (using `Commands` or direct world access). Either will work.
///
/// It is possible to "transition" to the current state (useful if you want to "reset" it).
/// If you set [`NextState`] to the same value as before, the exit/enter systems will run.
///
/// If you change the [`CurrentState`] value yourself, you will bypass the state transitions
/// (change to a different "active state" without running any exit/enter systems). You probably
/// don't want to do this; use [`NextState`] to change state in normal circumstances.
///
/// A single run of this stage can execute multiple transitions, if you update
/// `NextState` from within an exit or enter system.
pub struct StateTransitionStage<T: StateData> {
    /// The enter schedules of each state
    enter_stages: HashMap<T, Box<dyn Stage>>,
    /// The exit schedules of each state
    exit_stages: HashMap<T, Box<dyn Stage>>,
    /// The starting state value
    init_state: T,
    /// State used for resource access and change detection
    next_access: Option<SystemState<Option<SRes<NextState<T>>>>>,
}

impl<T: StateData> StateTransitionStage<T> {
    /// Create a new transitions stage for the given state type
    ///
    /// The provided value is the one that will be used to initialize the
    /// `CurrentState<T>` resource if it is missing.
    pub fn new(init_state: T) -> Self {
        Self {
            enter_stages: Default::default(),
            exit_stages: Default::default(),
            init_state,
            next_access: None,
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
        if let Some(mut next_access) = self.next_access.take() {
            loop {
                // Robustness: if someone removed the `CurrentState` resource,
                // we will re-add it. We remember the last known value it had.
                let current = world.get_resource_or_insert_with(|| CurrentState(self.init_state.clone())).0.clone();
                self.init_state = current.clone();

                // Access `NextState` with change detection. Robustness: re-insert if missing.
                if let Some((next, changed)) = next_access.get(world).map(|x| (x.0.clone(), x.is_changed())) {
                    if changed {
                        // perform transition
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
                } else {
                    world.insert_resource(NextState(current.clone()));
                    break;
                }
            }
            // take and reinsert to workaround borrow checker
            self.next_access = Some(next_access);
        } else {
            // First run; we gotta init stuff
            world.insert_resource(CurrentState(self.init_state.clone()));
            world.insert_resource(NextState(self.init_state.clone()));
            self.next_access = Some(SystemState::new(world));
            // run any initial enter stage
            if let Some(stage) = self.enter_stages.get_mut(&self.init_state) {
                stage.run(world);
            }
        }
    }
}
