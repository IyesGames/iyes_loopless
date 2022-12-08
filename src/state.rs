//! States implementation based on Run Conditions and a transitions Stage
//!
//! This is an alternative to Bevy's States. The current state is represented as a
//! resource and can be checked using Run Conditions. Transitions (running exit/enter
//! systems) are performed in a dedicated Stage. You can combine all of this
//! functionality with other scheduling functionality from this crate: run conditions,
//! fixed timesteps, etc. You can have multiple state types (to represent orthogonal
//! aspects of your application) and combine them trivially.
//!
//! (see `examples/menu.rs` for a full example)
use bevy_ecs::schedule::{Stage, StateData, StageLabel, IntoSystemDescriptor, SystemSet, SystemStage};
use bevy_ecs::world::World;
use bevy_ecs::system::Resource;
use bevy_utils::HashMap;

use std::any::TypeId;

/// This will be available as a resource, indicating the current state
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Resource)]
pub struct CurrentState<T>(pub T);

/// When you want to change state, insert this as a resource
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Resource)]
pub struct NextState<T>(pub T);

#[cfg(feature = "bevy-inspector-egui")]
impl<T: bevy_inspector_egui::Inspectable> bevy_inspector_egui::Inspectable for CurrentState<T> {
    type Attributes = T::Attributes;
    fn ui(&mut self, ui: &mut bevy_inspector_egui::egui::Ui, options: Self::Attributes, cx: &mut bevy_inspector_egui::Context) -> bool {
        self.0.ui(ui, options, cx)
    }
}
#[cfg(feature = "bevy-inspector-egui")]
impl<T: bevy_inspector_egui::Inspectable> bevy_inspector_egui::Inspectable for NextState<T> {
    type Attributes = T::Attributes;
    fn ui(&mut self, ui: &mut bevy_inspector_egui::egui::Ui, options: Self::Attributes, cx: &mut bevy_inspector_egui::Context) -> bool {
        self.0.ui(ui, options, cx)
    }
}


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
/// doesn't exist, and update it on state transitions.
///
/// If you mutate the value of [`CurrentState`] directly, instead of using [`NextState`],
/// then the state will be changed without running any exit/enter systems!
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

    /// Provide the stage to run when exiting the given state
    pub fn set_exit_stage<S: Stage>(&mut self, state: T, stage: S) {
        self.exit_stages.insert(state, Box::new(stage));
    }

    /// Builder version of `set_enter_stage`
    pub fn with_enter_stage<S: Stage>(mut self, state: T, stage: S) -> Self {
        self.set_enter_stage(state, stage);
        self
    }

    /// Builder version of `set_exit_stage`
    pub fn with_exit_stage<S: Stage>(mut self, state: T, stage: S) -> Self {
        self.set_exit_stage(state, stage);
        self
    }

    /// Add a system to run when entering the given state
    ///
    /// Does not work if you have set a custom enter stage
    /// of type other than `SystemStage`.
    ///
    /// Will create the enter `SystemStage` if it does not exist.
    pub fn add_enter_system<Params>(&mut self, state: T, system: impl IntoSystemDescriptor<Params>) {
        if !self.enter_stages.contains_key(&state) {
            self.set_enter_stage(state.clone(), SystemStage::parallel());
        }

        let stage = self.enter_stages.get_mut(&state)
            .expect("No enter stage for state.")
            .downcast_mut::<SystemStage>()
            .expect("State enter stage is not a SystemStage");

        stage.add_system(system);
    }

    /// Add a system to run when exiting the given state
    ///
    /// Does not work if you have set a custom exit stage
    /// of type other than `SystemStage`.
    ///
    /// Will create the exit `SystemStage` if it does not exist.
    pub fn add_exit_system<Params>(&mut self, state: T, system: impl IntoSystemDescriptor<Params>) {
        if !self.exit_stages.contains_key(&state) {
            self.set_exit_stage(state.clone(), SystemStage::parallel());
        }

        let stage = self.exit_stages.get_mut(&state)
            .expect("No exit stage for state.")
            .downcast_mut::<SystemStage>()
            .expect("State exit stage is not a SystemStage");

        stage.add_system(system);
    }

    /// Add a system set with multiple systems to run when entering the given state
    ///
    /// In practice, you probably want to use [`ConditionSet`] to construct this,
    /// and not use Bevy's builtin run criteria, etc.
    ///
    /// Does not work if you have set a custom enter stage
    /// of type other than `SystemStage`.
    ///
    /// Will create the enter `SystemStage` if it does not exist.
    pub fn add_enter_system_set(&mut self, state: T, system_set: SystemSet) {
        if !self.enter_stages.contains_key(&state) {
            self.set_enter_stage(state.clone(), SystemStage::parallel());
        }

        let stage = self.enter_stages.get_mut(&state)
            .expect("No enter stage for state.")
            .downcast_mut::<SystemStage>()
            .expect("State enter stage is not a SystemStage");

        stage.add_system_set(system_set);
    }

    /// Add a system set with multiple systems to run when exiting the given state
    ///
    /// In practice, you probably want to use [`ConditionSet`] to construct this,
    /// and not use Bevy's builtin run criteria, etc.
    ///
    /// Does not work if you have set a custom exit stage
    /// of type other than `SystemStage`.
    ///
    /// Will create the exit `SystemStage` if it does not exist.
    pub fn add_exit_system_set(&mut self, state: T, system_set: SystemSet) {
        if !self.exit_stages.contains_key(&state) {
            self.set_exit_stage(state.clone(), SystemStage::parallel());
        }

        let stage = self.exit_stages.get_mut(&state)
            .expect("No exit stage for state.")
            .downcast_mut::<SystemStage>()
            .expect("State exit stage is not a SystemStage");

        stage.add_system_set(system_set);
    }

    /// Builder version of `add_enter_system`
    pub fn with_enter_system<Params>(mut self, state: T, system: impl IntoSystemDescriptor<Params>) -> Self {
        self.add_enter_system(state, system);
        self
    }

    /// Builder version of `add_exit_system`
    pub fn with_exit_system<Params>(mut self, state: T, system: impl IntoSystemDescriptor<Params>) -> Self {
        self.add_exit_system(state, system);
        self
    }

    /// Builder version of `add_enter_system_set`
    pub fn with_enter_system_set(mut self, state: T, system_set: SystemSet) -> Self {
        self.add_enter_system_set(state, system_set);
        self
    }

    /// Builder version of `add_exit_system_set`
    pub fn with_exit_system_set(mut self, state: T, system_set: SystemSet) -> Self {
        self.add_exit_system_set(state, system_set);
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

/// Type used as a Bevy Stage Label for state transition stages
#[derive(Debug, Clone)]
pub struct StateTransitionStageLabel(TypeId, String);

impl StageLabel for StateTransitionStageLabel {
    fn as_str(&self) -> &'static str {
        let s = format!("{:?}{}", self.0, self.1);
        Box::leak(s.into_boxed_str())
    }
}

impl StateTransitionStageLabel {
    /// Construct the label for a stage to drive the state type T
    pub fn from_type<T: StateData>() -> Self {
        use std::any::type_name;
        StateTransitionStageLabel(TypeId::of::<T>(), type_name::<T>().to_owned())
    }
}

/// Extensions to `bevy_app`
#[cfg(feature = "app")]
pub mod app {
    use bevy_ecs::schedule::{StageLabel, Stage, StateData, IntoSystemDescriptor, SystemSet};
    use bevy_app::{App, CoreStage};

    use super::{StateTransitionStage, StateTransitionStageLabel};

    /// Extension trait with the methods to add to Bevy's `App`
    pub trait AppLooplessStateExt {
        /// Add a `StateTransitionStage` in the default position
        ///
        /// (before `CoreStage::Update`)
        fn add_loopless_state<T: StateData>(&mut self, init: T) -> &mut App;
        /// Add a `StateTransitionStage` after the specified stage
        fn add_loopless_state_after_stage<T: StateData>(&mut self, stage: impl StageLabel, init: T) -> &mut App;
        /// Add a `StateTransitionStage` before the specified stage
        fn add_loopless_state_before_stage<T: StateData>(&mut self, stage: impl StageLabel, init: T) -> &mut App;
        /// Add an enter system for the given state
        ///
        /// Requires the stage to be labeled with a `StateTransitionStageLabel`
        /// (as done by the `add_loopless_state*` methods).
        fn add_enter_system<T: StateData, Params>(&mut self, state: T, system: impl IntoSystemDescriptor<Params>) -> &mut App;
        /// Add an exit system for the given state
        ///
        /// Requires the stage to be labeled with a `StateTransitionStageLabel`
        /// (as done by the `add_loopless_state*` methods).
        fn add_exit_system<T: StateData, Params>(&mut self, state: T, system: impl IntoSystemDescriptor<Params>) -> &mut App;
        /// Add an enter system set for the given state
        ///
        /// Requires the stage to be labeled with a `StateTransitionStageLabel`
        /// (as done by the `add_loopless_state*` methods).
        fn add_enter_system_set<T: StateData>(&mut self, state: T, system_set: SystemSet) -> &mut App;
        /// Add an exit system set for the given state
        ///
        /// Requires the stage to be labeled with a `StateTransitionStageLabel`
        /// (as done by the `add_loopless_state*` methods).
        fn add_exit_system_set<T: StateData>(&mut self, state: T, system_set: SystemSet) -> &mut App;
        /// Add a custom stage to execute for the given state
        ///
        /// Requires the stage to be labeled with a `StateTransitionStageLabel`
        /// (as done by the `add_loopless_state*` methods).
        ///
        /// Cannot be used together with `add_enter_system`.
        fn set_enter_stage<T: StateData>(&mut self, state: T, stage: impl Stage) -> &mut App;
        /// Add a custom stage to execute for the given state
        ///
        /// Requires the stage to be labeled with a `StateTransitionStageLabel`
        /// (as done by the `add_loopless_state*` methods).
        ///
        /// Cannot be used together with `add_enter_system`.
        fn set_exit_stage<T: StateData>(&mut self, state: T, stage: impl Stage) -> &mut App;
    }

    impl AppLooplessStateExt for App {
        fn add_loopless_state<T: StateData>(&mut self, init: T) -> &mut App {
            self.add_loopless_state_before_stage(CoreStage::Update, init)
        }
        fn add_loopless_state_after_stage<T: StateData>(&mut self, stage: impl StageLabel, init: T) -> &mut App {
            self.add_stage_after(
                stage,
                StateTransitionStageLabel::from_type::<T>(),
                StateTransitionStage::new(init)
            )
        }
        fn add_loopless_state_before_stage<T: StateData>(&mut self, stage: impl StageLabel, init: T) -> &mut App {
            self.add_stage_before(
                stage,
                StateTransitionStageLabel::from_type::<T>(),
                StateTransitionStage::new(init)
            )
        }
        fn add_enter_system<T: StateData, Params>(&mut self, state: T, system: impl IntoSystemDescriptor<Params>) -> &mut App {
            let stage = self.schedule.get_stage_mut::<StateTransitionStage<T>>(StateTransitionStageLabel::from_type::<T>())
                .expect("State Transition Stage not found (assuming auto-added label)");
            stage.add_enter_system(state, system);
            self
        }
        fn add_exit_system<T: StateData, Params>(&mut self, state: T, system: impl IntoSystemDescriptor<Params>) -> &mut App {
            let stage = self.schedule.get_stage_mut::<StateTransitionStage<T>>(StateTransitionStageLabel::from_type::<T>())
                .expect("State Transition Stage not found (assuming auto-added label)");
            stage.add_exit_system(state, system);
            self
        }
        fn add_enter_system_set<T: StateData>(&mut self, state: T, system_set: SystemSet) -> &mut App {
            let stage = self.schedule.get_stage_mut::<StateTransitionStage<T>>(StateTransitionStageLabel::from_type::<T>())
                .expect("State Transition Stage not found (assuming auto-added label)");
            stage.add_enter_system_set(state, system_set);
            self
        }
        fn add_exit_system_set<T: StateData>(&mut self, state: T, system_set: SystemSet) -> &mut App {
            let stage = self.schedule.get_stage_mut::<StateTransitionStage<T>>(StateTransitionStageLabel::from_type::<T>())
                .expect("State Transition Stage not found (assuming auto-added label)");
            stage.add_exit_system_set(state, system_set);
            self
        }
        fn set_enter_stage<T: StateData>(&mut self, state: T, enter_stage: impl Stage) -> &mut App {
            let stage = self.schedule.get_stage_mut::<StateTransitionStage<T>>(StateTransitionStageLabel::from_type::<T>())
                .expect("State Transition Stage not found (assuming auto-added label)");
            stage.set_enter_stage(state, enter_stage);
            self
        }
        fn set_exit_stage<T: StateData>(&mut self, state: T, exit_stage: impl Stage) -> &mut App {
            let stage = self.schedule.get_stage_mut::<StateTransitionStage<T>>(StateTransitionStageLabel::from_type::<T>())
                .expect("State Transition Stage not found (assuming auto-added label)");
            stage.set_exit_stage(state, exit_stage);
            self
        }
    }
}

/// Extensions to Bevy Schedule
pub mod schedule {
    use bevy_ecs::schedule::{StageLabel, Stage, StateData, IntoSystemDescriptor, SystemSet, Schedule};

    use super::{StateTransitionStage, StateTransitionStageLabel};

    /// Extension trait with the methods to add to Bevy's `Schedule`
    pub trait ScheduleLooplessStateExt {
        /// Add a `StateTransitionStage` after the specified stage
        fn add_loopless_state_after_stage<T: StateData>(&mut self, stage: impl StageLabel, init: T) -> &mut Schedule;
        /// Add a `StateTransitionStage` before the specified stage
        fn add_loopless_state_before_stage<T: StateData>(&mut self, stage: impl StageLabel, init: T) -> &mut Schedule;
        /// Add an enter system for the given state
        ///
        /// Requires the stage to be labeled with a `StateTransitionStageLabel`
        /// (as done by the `add_loopless_state*` methods).
        fn add_enter_system<T: StateData, Params>(&mut self, state: T, system: impl IntoSystemDescriptor<Params>) -> &mut Schedule;
        /// Add an exit system for the given state
        ///
        /// Requires the stage to be labeled with a `StateTransitionStageLabel`
        /// (as done by the `add_loopless_state*` methods).
        fn add_exit_system<T: StateData, Params>(&mut self, state: T, system: impl IntoSystemDescriptor<Params>) -> &mut Schedule;
        /// Add an enter system set for the given state
        ///
        /// Requires the stage to be labeled with a `StateTransitionStageLabel`
        /// (as done by the `add_loopless_state*` methods).
        fn add_enter_system_set<T: StateData>(&mut self, state: T, system_set: SystemSet) -> &mut Schedule;
        /// Add an exit system set for the given state
        ///
        /// Requires the stage to be labeled with a `StateTransitionStageLabel`
        /// (as done by the `add_loopless_state*` methods).
        fn add_exit_system_set<T: StateData>(&mut self, state: T, system_set: SystemSet) -> &mut Schedule;
        /// Add a custom stage to execute for the given state
        ///
        /// Requires the stage to be labeled with a `StateTransitionStageLabel`
        /// (as done by the `add_loopless_state*` methods).
        ///
        /// Cannot be used together with `add_enter_system`.
        fn set_enter_stage<T: StateData>(&mut self, state: T, stage: impl Stage) -> &mut Schedule;
        /// Add a custom stage to execute for the given state
        ///
        /// Requires the stage to be labeled with a `StateTransitionStageLabel`
        /// (as done by the `add_loopless_state*` methods).
        ///
        /// Cannot be used together with `add_enter_system`.
        fn set_exit_stage<T: StateData>(&mut self, state: T, stage: impl Stage) -> &mut Schedule;
    }

    impl ScheduleLooplessStateExt for Schedule {
        fn add_loopless_state_after_stage<T: StateData>(&mut self, stage: impl StageLabel, init: T) -> &mut Schedule {
            self.add_stage_after(
                stage,
                StateTransitionStageLabel::from_type::<T>(),
                StateTransitionStage::new(init)
            )
        }
        fn add_loopless_state_before_stage<T: StateData>(&mut self, stage: impl StageLabel, init: T) -> &mut Schedule {
            self.add_stage_before(
                stage,
                StateTransitionStageLabel::from_type::<T>(),
                StateTransitionStage::new(init)
            )
        }
        fn add_enter_system<T: StateData, Params>(&mut self, state: T, system: impl IntoSystemDescriptor<Params>) -> &mut Schedule {
            let stage = self.get_stage_mut::<StateTransitionStage<T>>(StateTransitionStageLabel::from_type::<T>())
                .expect("State Transition Stage not found (assuming auto-added label)");
            stage.add_enter_system(state, system);
            self
        }
        fn add_exit_system<T: StateData, Params>(&mut self, state: T, system: impl IntoSystemDescriptor<Params>) -> &mut Schedule {
            let stage = self.get_stage_mut::<StateTransitionStage<T>>(StateTransitionStageLabel::from_type::<T>())
                .expect("State Transition Stage not found (assuming auto-added label)");
            stage.add_exit_system(state, system);
            self
        }
        fn add_enter_system_set<T: StateData>(&mut self, state: T, system_set: SystemSet) -> &mut Schedule {
            let stage = self.get_stage_mut::<StateTransitionStage<T>>(StateTransitionStageLabel::from_type::<T>())
                .expect("State Transition Stage not found (assuming auto-added label)");
            stage.add_enter_system_set(state, system_set);
            self
        }
        fn add_exit_system_set<T: StateData>(&mut self, state: T, system_set: SystemSet) -> &mut Schedule {
            let stage = self.get_stage_mut::<StateTransitionStage<T>>(StateTransitionStageLabel::from_type::<T>())
                .expect("State Transition Stage not found (assuming auto-added label)");
            stage.add_exit_system_set(state, system_set);
            self
        }
        fn set_enter_stage<T: StateData>(&mut self, state: T, enter_stage: impl Stage) -> &mut Schedule {
            let stage = self.get_stage_mut::<StateTransitionStage<T>>(StateTransitionStageLabel::from_type::<T>())
                .expect("State Transition Stage not found (assuming auto-added label)");
            stage.set_enter_stage(state, enter_stage);
            self
        }
        fn set_exit_stage<T: StateData>(&mut self, state: T, exit_stage: impl Stage) -> &mut Schedule {
            let stage = self.get_stage_mut::<StateTransitionStage<T>>(StateTransitionStageLabel::from_type::<T>())
                .expect("State Transition Stage not found (assuming auto-added label)");
            stage.set_exit_stage(state, exit_stage);
            self
        }
    }
}
