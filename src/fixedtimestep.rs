//! Fixed Timestep implementation as a Bevy Stage
//!
//! This is an alternative to Bevy's FixedTimestep. It does not (ab)use run criteria; instead,
//! it runs in a dedicated stage, separate from your regular update systems. It does not conflict
//! with any other functionality, and can be combined with states, run conditions, etc.
//!
//! It is possible to add multiple "sub-stages" within a fixed timestep, allowing
//! you to apply `Commands` within a single timestep run. For example, if you want
//! to spawn entities and then do something with them, on the same tick.
//!
//! It is also possible to have multiple independent fixed timesteps, should you need to.
//!
//! (see `examples/fixedtimestep.rs` to learn how to use it)
//!
//! Every frame, the [`FixedTimestepStage`] will accumulate the time delta. When
//! it goes over the set timestep value, it will run all the child stages. It
//! will repeat the sequence of child stages multiple times if needed, if
//! more than one timestep has accumulated.
//!
//! You can use the [`FixedTimesteps`] resource (make sure it is the one from this
//! crate, not the one from Bevy with the same name) to access information about a
//! fixed timestep and to control its parameters, like the timestep duration.

use bevy_time::Time;
use bevy_utils::Duration;
use bevy_utils::HashMap;

use bevy_ecs::prelude::*;

/// The "name" of a fixed timestep. Used to manipulate it.
pub type TimestepName = &'static str;

/// Resource type that allows you to get info about and to manipulate fixed timestep state
///
/// If you want to access parameters of your fixed timestep(s), such as the timestep duration,
/// accumulator, and paused state, you can get them from this resource. They are contained
/// in a [`FixedTimestepInfo`] struct, which you can get using the various methods on this type.
///
/// If you mutate the timestep duration or paused state, they will be taken into account
/// from the next run of that fixed timestep.
///
/// From within a fixed timestep system, you can also mutate the accumulator. May be useful
/// for networking or other use cases that need to stretch time.
#[derive(Default)]
#[derive(Resource)]
pub struct FixedTimesteps {
    info: HashMap<TimestepName, FixedTimestepInfo>,
    current: Option<TimestepName>,
}

impl FixedTimesteps {
    /// Returns a reference to the timestep info for a given timestep by name.
    pub fn get(&self, label: TimestepName) -> Option<&FixedTimestepInfo> {
        self.info.get(label)
    }

    /// Returns a reference to the timestep info for the currently running stage.
    ///
    /// Returns [`Some`] only if called inside a fixed timestep stage.
    pub fn get_current(&self) -> Option<&FixedTimestepInfo> {
        self.current.as_ref().and_then(|label| self.info.get(label))
    }

    /// Panicking version of [`get_current`]
    pub fn current(&self) -> &FixedTimestepInfo {
        self.get_current()
            .expect("FixedTimesteps::current can only be used when running inside a fixed timestep.")
    }

    /// Returns a reference to the timestep info, assuming you only have one.
    pub fn get_single(&self) -> Option<&FixedTimestepInfo> {
        if self.info.len() != 1 {
            return None;
        }
        self.info.values().next()
    }

    /// Panicking version of [`get_single`]
    pub fn single(&self) -> &FixedTimestepInfo {
        self.get_single().expect("Expected exactly one fixed timestep.")
    }

    /// Returns a mut reference to the timestep info for a given timestep by name.
    pub fn get_mut(&mut self, label: TimestepName) -> Option<&mut FixedTimestepInfo> {
        self.info.get_mut(label)
    }

    /// Returns a mut reference to the timestep info for the currently running stage.
    ///
    /// Returns [`Some`] only if called inside a fixed timestep stage.
    pub fn get_current_mut(&mut self) -> Option<&mut FixedTimestepInfo> {
        self.current.as_ref().and_then(|label| self.info.get_mut(label))
    }

    /// Panicking version of [`get_current_mut`]
    pub fn current_mut(&mut self) -> &mut FixedTimestepInfo {
        self.get_current_mut()
            .expect("FixedTimesteps::current can only be used when running inside a fixed timestep.")
    }

    /// Returns a mut reference to the timestep info, assuming you only have one.
    pub fn get_single_mut(&mut self) -> Option<&mut FixedTimestepInfo> {
        if self.info.len() != 1 {
            return None;
        }
        self.info.values_mut().next()
    }

    /// Panicking version of [`get_single_mut`]
    pub fn single_mut(&mut self) -> &mut FixedTimestepInfo {
        self.get_single_mut().expect("Expected exactly one fixed timestep.")
    }
}

/// Provides access to the parameters of a fixed timestep
///
/// You can get this using the [`FixedTimesteps`] resource.
pub struct FixedTimestepInfo {
    /// Duration of each fixed timestep tick
    pub step: Duration,
    /// Accumulated time since the last fixed timestep run
    pub accumulator: Duration,
    /// Is the fixed timestep paused?
    pub paused: bool,
}

impl FixedTimestepInfo {
    /// The time duration of each timestep
    pub fn timestep(&self) -> Duration {
        self.step
    }
    /// The number of steps per second (Hz)
    pub fn rate(&self) -> f64 {
        1.0 / self.step.as_secs_f64()
    }
    /// The amount of time left over from the last timestep
    pub fn remaining(&self) -> Duration {
        self.accumulator
    }
    /// How much has the main game update "overstepped" the fixed timestep?
    /// (how many more (fractional) timesteps are left over in the accumulator)
    pub fn overstep(&self) -> f64 {
        self.accumulator.as_secs_f64() / self.step.as_secs_f64()
    }

    /// Pause the fixed timestep
    pub fn pause(&mut self) {
        self.paused = true;
    }

    /// Un-pause (resume) the fixed timestep
    pub fn unpause(&mut self) {
        self.paused = false;
    }

    /// Toggle the paused state
    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }
}

/// A Stage that runs a number of child stages with a fixed timestep
///
/// You can set the timestep duration. Every frame update, the time delta
/// will be accumulated, and the child stages will run when it goes over
/// the timestep threshold. If multiple timesteps have been accumulated,
/// the child stages will be run multiple times.
///
/// You can add multiple child stages, allowing you to use `Commands` in
/// your fixed timestep systems, and have their effects applied.
///
/// A good place to add the `FixedTimestepStage` is usually before
/// `CoreStage::Update`.
pub struct FixedTimestepStage {
    step: Duration,
    accumulator: Duration,
    paused: bool,
    label: TimestepName,
    stages: Vec<Box<dyn Stage>>,
    rate_lock: (u32, f32),
    lock_accum: u32,
}

impl FixedTimestepStage {
    /// Helper to create a `FixedTimestepStage` with a single child stage
    pub fn from_stage<S: Stage>(timestep: Duration, label: TimestepName, stage: S) -> Self {
        Self::new(timestep, label).with_stage(stage)
    }

    /// Create a new empty `FixedTimestepStage` with no child stages
    pub fn new(timestep: Duration, label: TimestepName) -> Self {
        Self {
            step: timestep,
            accumulator: Duration::default(),
            paused: false,
            label,
            stages: Vec::new(),
            rate_lock: (u32::MAX, 0.0),
            lock_accum: 0,
        }
    }

    /// Builder method for starting in a paused state
    pub fn paused(mut self) -> Self {
        self.paused = true;
        self
    }

    /// Add a child stage
    pub fn add_stage<S: Stage>(&mut self, stage: S) {
        self.stages.push(Box::new(stage));
    }

    /// Builder method for adding a child stage
    pub fn with_stage<S: Stage>(mut self, stage: S) -> Self {
        self.add_stage(stage);
        self
    }

    /// Enable EXPERIMENTAL "rate locking" algorithm
    ///
    /// The idea is to detect if the fixed timestep rate is "close enough"
    /// to the actual update rate, and if yes, stop accumulating delta time,
    /// to run cleanly without hickups/jitter (at the real update rate,
    /// instead of the set timestep duration).
    ///
    /// For example, if you set a timestep of `1.0/60.0` seconds, and run with
    /// vsync on a typical 59.97Hz monitor, you might prefer to just get one
    /// fixed update per frame anyway, instead of occasional hickups/jitter
    /// due to the subtle mismatch between the fixed timestep and the real rate.
    ///
    /// The algorithm works as follows: count how many timesteps get executed
    /// each frame, and if the number doesn't change for `n_frames` consecutive
    /// frames, enter "locked mode". While in locked mode, reset the accumulator
    /// to half the step duration at the start of each execution. If, at any
    /// time, there is a frame that causes the accumulator to deviate by more
    /// than `exit_deviation` timestep durations, leave "locked mode".
    ///
    /// Reasonable parameters: `n_frames`: `5` to `15`, `exit_deviation`: `0.05` to `0.1`.
    pub fn set_rate_lock(&mut self, n_frames: u32, exit_deviation: f32) {
        assert!(exit_deviation > 0.0);
        assert!(n_frames > 0);
        self.rate_lock = (n_frames, exit_deviation);
    }

    /// Builder-style method for [`set_rate_lock`]
    pub fn with_rate_lock(mut self, n_frames: u32, exit_deviation: f32) -> Self {
        self.set_rate_lock(n_frames, exit_deviation);
        self
    }

    /// ensure the FixedTimesteps resource exists and contains the latest data
    fn store_fixedtimestepinfo(&self, world: &mut World) {
        if let Some(mut timesteps) = world.get_resource_mut::<FixedTimesteps>() {
            timesteps.current = Some(self.label);
            if let Some(mut info) = timesteps.info.get_mut(&self.label) {
                info.step = self.step;
                info.accumulator = self.accumulator;
                info.paused = self.paused;
            } else {
                timesteps.info.insert(self.label, FixedTimestepInfo {
                    step: self.step,
                    accumulator: self.accumulator,
                    paused: self.paused,
                });
            }
        } else {
            let mut timesteps = FixedTimesteps::default();
            timesteps.current = Some(self.label);
            timesteps.info.insert(self.label, FixedTimestepInfo {
                step: self.step,
                accumulator: self.accumulator,
                paused: self.paused,
            });
            world.insert_resource(timesteps);
        }
    }
}

impl Stage for FixedTimestepStage {
    fn run(&mut self, world: &mut World) {
        if let Some(timesteps) = world.get_resource::<FixedTimesteps>() {
            if let Some(info) = timesteps.info.get(&self.label) {
                self.step = info.step;
                self.paused = info.paused;
                // do not sync accumulator
            }
        }

        if self.paused {
            return;
        }

        self.accumulator += {
            let time = world.get_resource::<Time>();
            if let Some(time) = time {
                time.delta()
            } else {
                return;
            }
        };

        if self.lock_accum >= self.rate_lock.0 {
            let overstep = self.accumulator.as_secs_f32() / self.step.as_secs_f32();
            if (overstep - 1.5).abs() >= self.rate_lock.1 {
                self.lock_accum = 0;
            } else {
                self.accumulator = self.step + self.step / 2;
            }
        }

        let mut n_steps = 0;

        while self.accumulator >= self.step {
            self.accumulator -= self.step;

            self.store_fixedtimestepinfo(world);

            for stage in self.stages.iter_mut() {
                // run user systems
                stage.run(world);

                // if the user modified fixed timestep info, we need to copy it back
                if let Some(timesteps) = world.get_resource::<FixedTimesteps>() {
                    if let Some(info) = timesteps.info.get(&self.label) {
                        // update our actual step duration, in case the user has
                        // modified it in the info resource
                        self.step = info.step;
                        self.accumulator = info.accumulator;
                        self.paused = info.paused;
                    }
                }
            }
            n_steps += 1;
        }

        if let Some(mut timesteps) = world.get_resource_mut::<FixedTimesteps>() {
            timesteps.current = None;
        }

        if n_steps == 0 {
            self.store_fixedtimestepinfo(world);
        }

        if n_steps == 1 {
            if self.lock_accum < self.rate_lock.0 {
                self.lock_accum += 1;
            }
            if self.lock_accum >= self.rate_lock.0 {
                self.accumulator = self.step / 2;
            }
        } else {
            self.lock_accum = 0;
        }
    }
}

/// Type used as a Bevy Stage Label for fixed timestep stages
#[derive(Debug, Clone)]
pub struct FixedTimestepStageLabel(pub TimestepName);

impl StageLabel for FixedTimestepStageLabel {
    fn as_str(&self) -> &'static str {
        self.0
    }
}

/// Extensions to `bevy_app`
#[cfg(feature = "app")]
pub mod app {
    use bevy_utils::Duration;
    use bevy_ecs::prelude::*;
    use bevy_ecs::schedule::IntoSystemDescriptor;
    use bevy_app::{App, CoreStage};

    use super::{FixedTimestepStage, FixedTimestepStageLabel, TimestepName};

    /// Extension trait with the methods to add to Bevy's `App`
    pub trait AppLooplessFixedTimestepExt {
        /// Create a new fixed timestep stage and add it to the schedule in the default position
        ///
        /// You need to provide a name string, which you can use later to do things with the timestep.
        ///
        /// The [`FixedTimestepStage`] is created with one child sub-stage: a Bevy parallel `SystemStage`.
        ///
        /// The new stage is inserted into the default position: before `CoreStage::Update`.
        fn add_fixed_timestep(&mut self, timestep: Duration, label: TimestepName) -> &mut App;
        /// Create a new fixed timestep stage and add it to the schedule before a given stage
        ///
        /// Like [`add_fixed_timestep`], but you control where to add the fixed timestep stage.
        fn add_fixed_timestep_before_stage(&mut self, stage: impl StageLabel, timestep: Duration, label: TimestepName) -> &mut App;
        /// Create a new fixed timestep stage and add it to the schedule after a given stage
        ///
        /// Like [`add_fixed_timestep`], but you control where to add the fixed timestep stage.
        fn add_fixed_timestep_after_stage(&mut self, stage: impl StageLabel, timestep: Duration, label: TimestepName) -> &mut App;
        /// Add a child sub-stage to a fixed timestep stage
        ///
        /// It will be added at the end, after any sub-stages that already exist.
        ///
        /// The new stage will be a Bevy parallel `SystemStage`.
        fn add_fixed_timestep_child_stage(&mut self, timestep_name: TimestepName) -> &mut App;
        /// Add a custom child sub-stage to a fixed timestep stage
        ///
        /// It will be added at the end, after any sub-stages that already exist.
        ///
        /// You can provide any stage type you like.
        fn add_fixed_timestep_custom_child_stage(&mut self, timestep_name: TimestepName, stage: impl Stage) -> &mut App;
        /// Add a system to run under a fixed timestep
        ///
        /// To specify where to add the system, provide the name string of the fixed timestep, and the
        /// numeric index of the sub-stage (`0` if you have not added any additional sub-stages).
        fn add_fixed_timestep_system<Params>(&mut self, timestep_name: TimestepName, substage_i: usize, system: impl IntoSystemDescriptor<Params>) -> &mut App;
        /// Add many systems to run under a fixed timestep
        ///
        /// To specify where to add the systems, provide the name string of the fixed timestep, and the
        /// numeric index of the sub-stage (`0` if you have not added any additional sub-stages).
        fn add_fixed_timestep_system_set(&mut self, timestep_name: TimestepName, substage_i: usize, system_set: SystemSet) -> &mut App;
        /// Get access to the [`FixedTimestepStage`] for the fixed timestep with a given name string
        fn get_fixed_timestep_stage(&self, timestep_name: TimestepName) -> &FixedTimestepStage;
        /// Get mut access to the [`FixedTimestepStage`] for the fixed timestep with a given name string
        fn get_fixed_timestep_stage_mut(&mut self, timestep_name: TimestepName) -> &mut FixedTimestepStage;
        /// Get access to the i-th child sub-stage of the fixed timestep with the given name string
        fn get_fixed_timestep_child_substage<S: Stage>(&self, timestep_name: TimestepName, substage_i: usize) -> &S;
        /// Get mut access to the i-th child sub-stage of the fixed timestep with the given name string
        fn get_fixed_timestep_child_substage_mut<S: Stage>(&mut self, timestep_name: TimestepName, substage_i: usize) -> &mut S;
    }

    impl AppLooplessFixedTimestepExt for App {
        fn add_fixed_timestep(&mut self, timestep: Duration, label: TimestepName) -> &mut App {
            self.add_fixed_timestep_before_stage(CoreStage::Update, timestep, label)
        }

        fn add_fixed_timestep_before_stage(&mut self, stage: impl StageLabel, timestep: Duration, label: TimestepName) -> &mut App {
            let ftstage = FixedTimestepStage::from_stage(timestep, label, SystemStage::parallel());
            ftstage.store_fixedtimestepinfo(&mut self.world);
            self.add_stage_before(
                stage,
                FixedTimestepStageLabel(label),
                ftstage
            )
        }

        fn add_fixed_timestep_after_stage(&mut self, stage: impl StageLabel, timestep: Duration, label: TimestepName) -> &mut App {
            let ftstage = FixedTimestepStage::from_stage(timestep, label, SystemStage::parallel());
            ftstage.store_fixedtimestepinfo(&mut self.world);
            self.add_stage_after(
                stage,
                FixedTimestepStageLabel(label),
                ftstage
            )
        }

        fn add_fixed_timestep_child_stage(&mut self, timestep_name: TimestepName) -> &mut App {
            let stage = self.schedule.get_stage_mut::<FixedTimestepStage>(
                FixedTimestepStageLabel(timestep_name)
            ).expect("Fixed Timestep Stage not found");
            stage.add_stage(SystemStage::parallel());
            self
        }

        fn add_fixed_timestep_custom_child_stage(&mut self, timestep_name: TimestepName, custom_stage: impl Stage) -> &mut App {
            let stage = self.schedule.get_stage_mut::<FixedTimestepStage>(
                FixedTimestepStageLabel(timestep_name)
            ).expect("Fixed Timestep Stage not found");
            stage.add_stage(custom_stage);
            self
        }

        fn add_fixed_timestep_system<Params>(&mut self, timestep_name: TimestepName, substage_i: usize, system: impl IntoSystemDescriptor<Params>) -> &mut App {
            let stage = self.schedule.get_stage_mut::<FixedTimestepStage>(
                FixedTimestepStageLabel(timestep_name)
            ).expect("Fixed Timestep Stage not found");
            let substage = stage.stages.get_mut(substage_i)
                .expect("Fixed Timestep sub-stage not found")
                .downcast_mut::<SystemStage>()
                .expect("Fixed Timestep sub-stage is not a SystemStage");
            substage.add_system(system);
            self
        }

        fn add_fixed_timestep_system_set(&mut self, timestep_name: TimestepName, substage_i: usize, system_set: SystemSet) -> &mut App {
            let stage = self.schedule.get_stage_mut::<FixedTimestepStage>(
                FixedTimestepStageLabel(timestep_name)
            ).expect("Fixed Timestep Stage not found");
            let substage = stage.stages.get_mut(substage_i)
                .expect("Fixed Timestep sub-stage not found")
                .downcast_mut::<SystemStage>()
                .expect("Fixed Timestep sub-stage is not a SystemStage");
            substage.add_system_set(system_set);
            self
        }

        fn get_fixed_timestep_stage(&self, timestep_name: TimestepName) -> &FixedTimestepStage {
            self.schedule.get_stage::<FixedTimestepStage>(
                FixedTimestepStageLabel(timestep_name)
            ).expect("Fixed Timestep Stage not found")
        }

        fn get_fixed_timestep_stage_mut(&mut self, timestep_name: TimestepName) -> &mut FixedTimestepStage {
            self.schedule.get_stage_mut::<FixedTimestepStage>(
                FixedTimestepStageLabel(timestep_name)
            ).expect("Fixed Timestep Stage not found")
        }

        fn get_fixed_timestep_child_substage<S: Stage>(&self, timestep_name: TimestepName, substage_i: usize) -> &S {
            let stage = self.get_fixed_timestep_stage(timestep_name);
            stage.stages.get(substage_i)
                .expect("Fixed Timestep sub-stage not found")
                .downcast_ref::<S>()
                .expect("Fixed Timestep sub-stage is not the requested type")
        }

        fn get_fixed_timestep_child_substage_mut<S: Stage>(&mut self, timestep_name: TimestepName, substage_i: usize) -> &mut S {
            let stage = self.get_fixed_timestep_stage_mut(timestep_name);
            stage.stages.get_mut(substage_i)
                .expect("Fixed Timestep sub-stage not found")
                .downcast_mut::<S>()
                .expect("Fixed Timestep sub-stage is not the requested type")
        }
    }
}

/// Extensions to Bevy Schedule
pub mod schedule {
    use bevy_utils::Duration;
    use bevy_ecs::prelude::*;
    use bevy_ecs::schedule::IntoSystemDescriptor;

    use super::{FixedTimestepStage, FixedTimestepStageLabel, TimestepName};

    /// Extension trait with the methods to add to Bevy's `Schedule`
    pub trait ScheduleLooplessFixedTimestepExt {
        /// Create a new fixed timestep stage and add it to the schedule before a given stage
        ///
        /// You need to provide a name string, which you can use later to do things with the timestep.
        ///
        /// The [`FixedTimestepStage`] is created with one child sub-stage: a Bevy parallel `SystemStage`.
        ///
        /// Like [`add_fixed_timestep`], but you control where to add the fixed timestep stage.
        fn add_fixed_timestep_before_stage(&mut self, stage: impl StageLabel, timestep: Duration, label: TimestepName) -> &mut Schedule;
        /// Create a new fixed timestep stage and add it to the schedule after a given stage
        ///
        /// You need to provide a name string, which you can use later to do things with the timestep.
        ///
        /// The [`FixedTimestepStage`] is created with one child sub-stage: a Bevy parallel `SystemStage`.
        ///
        /// Like [`add_fixed_timestep`], but you control where to add the fixed timestep stage.
        fn add_fixed_timestep_after_stage(&mut self, stage: impl StageLabel, timestep: Duration, label: TimestepName) -> &mut Schedule;
        /// Add a child sub-stage to a fixed timestep stage
        ///
        /// It will be added at the end, after any sub-stages that already exist.
        ///
        /// The new stage will be a Bevy parallel `SystemStage`.
        fn add_fixed_timestep_child_stage(&mut self, timestep_name: TimestepName) -> &mut Schedule;
        /// Add a custom child sub-stage to a fixed timestep stage
        ///
        /// It will be added at the end, after any sub-stages that already exist.
        ///
        /// You can provide any stage type you like.
        fn add_fixed_timestep_custom_child_stage(&mut self, timestep_name: TimestepName, stage: impl Stage) -> &mut Schedule;
        /// Add a system to run under a fixed timestep
        ///
        /// To specify where to add the system, provide the name string of the fixed timestep, and the
        /// numeric index of the sub-stage (`0` if you have not added any additional sub-stages).
        fn add_fixed_timestep_system<Params>(&mut self, timestep_name: TimestepName, substage_i: usize, system: impl IntoSystemDescriptor<Params>) -> &mut Schedule;
        /// Add many systems to run under a fixed timestep
        ///
        /// To specify where to add the systems, provide the name string of the fixed timestep, and the
        /// numeric index of the sub-stage (`0` if you have not added any additional sub-stages).
        fn add_fixed_timestep_system_set(&mut self, timestep_name: TimestepName, substage_i: usize, system_set: SystemSet) -> &mut Schedule;
        /// Get access to the [`FixedTimestepStage`] for the fixed timestep with a given name string
        fn get_fixed_timestep_stage(&self, timestep_name: TimestepName) -> &FixedTimestepStage;
        /// Get mut access to the [`FixedTimestepStage`] for the fixed timestep with a given name string
        fn get_fixed_timestep_stage_mut(&mut self, timestep_name: TimestepName) -> &mut FixedTimestepStage;
        /// Get access to the i-th child sub-stage of the fixed timestep with the given name string
        fn get_fixed_timestep_child_substage<S: Stage>(&self, timestep_name: TimestepName, substage_i: usize) -> &S;
        /// Get mut access to the i-th child sub-stage of the fixed timestep with the given name string
        fn get_fixed_timestep_child_substage_mut<S: Stage>(&mut self, timestep_name: TimestepName, substage_i: usize) -> &mut S;
    }

    impl ScheduleLooplessFixedTimestepExt for Schedule {
        fn add_fixed_timestep_before_stage(&mut self, stage: impl StageLabel, timestep: Duration, label: TimestepName) -> &mut Schedule {
            self.add_stage_before(
                stage,
                FixedTimestepStageLabel(label),
                FixedTimestepStage::from_stage(timestep, label, SystemStage::parallel())
            )
        }

        fn add_fixed_timestep_after_stage(&mut self, stage: impl StageLabel, timestep: Duration, label: TimestepName) -> &mut Schedule {
            self.add_stage_after(
                stage,
                FixedTimestepStageLabel(label),
                FixedTimestepStage::from_stage(timestep, label, SystemStage::parallel())
            )
        }

        fn add_fixed_timestep_child_stage(&mut self, timestep_name: TimestepName) -> &mut Schedule {
            let stage = self.get_stage_mut::<FixedTimestepStage>(
                FixedTimestepStageLabel(timestep_name)
            ).expect("Fixed Timestep Stage not found");
            stage.add_stage(SystemStage::parallel());
            self
        }

        fn add_fixed_timestep_custom_child_stage(&mut self, timestep_name: TimestepName, custom_stage: impl Stage) -> &mut Schedule {
            let stage = self.get_stage_mut::<FixedTimestepStage>(
                FixedTimestepStageLabel(timestep_name)
            ).expect("Fixed Timestep Stage not found");
            stage.add_stage(custom_stage);
            self
        }

        fn add_fixed_timestep_system<Params>(&mut self, timestep_name: TimestepName, substage_i: usize, system: impl IntoSystemDescriptor<Params>) -> &mut Schedule {
            let stage = self.get_stage_mut::<FixedTimestepStage>(
                FixedTimestepStageLabel(timestep_name)
            ).expect("Fixed Timestep Stage not found");
            let substage = stage.stages.get_mut(substage_i)
                .expect("Fixed Timestep sub-stage not found")
                .downcast_mut::<SystemStage>()
                .expect("Fixed Timestep sub-stage is not a SystemStage");
            substage.add_system(system);
            self
        }

        fn add_fixed_timestep_system_set(&mut self, timestep_name: TimestepName, substage_i: usize, system_set: SystemSet) -> &mut Schedule {
            let stage = self.get_stage_mut::<FixedTimestepStage>(
                FixedTimestepStageLabel(timestep_name)
            ).expect("Fixed Timestep Stage not found");
            let substage = stage.stages.get_mut(substage_i)
                .expect("Fixed Timestep sub-stage not found")
                .downcast_mut::<SystemStage>()
                .expect("Fixed Timestep sub-stage is not a SystemStage");
            substage.add_system_set(system_set);
            self
        }

        fn get_fixed_timestep_stage(&self, timestep_name: TimestepName) -> &FixedTimestepStage {
            self.get_stage::<FixedTimestepStage>(
                FixedTimestepStageLabel(timestep_name)
            ).expect("Fixed Timestep Stage not found")
        }

        fn get_fixed_timestep_stage_mut(&mut self, timestep_name: TimestepName) -> &mut FixedTimestepStage {
            self.get_stage_mut::<FixedTimestepStage>(
                FixedTimestepStageLabel(timestep_name)
            ).expect("Fixed Timestep Stage not found")
        }

        fn get_fixed_timestep_child_substage<S: Stage>(&self, timestep_name: TimestepName, substage_i: usize) -> &S {
            let stage = self.get_fixed_timestep_stage(timestep_name);
            stage.stages.get(substage_i)
                .expect("Fixed Timestep sub-stage not found")
                .downcast_ref::<S>()
                .expect("Fixed Timestep sub-stage is not the requested type")
        }

        fn get_fixed_timestep_child_substage_mut<S: Stage>(&mut self, timestep_name: TimestepName, substage_i: usize) -> &mut S {
            let stage = self.get_fixed_timestep_stage_mut(timestep_name);
            stage.stages.get_mut(substage_i)
                .expect("Fixed Timestep sub-stage not found")
                .downcast_mut::<S>()
                .expect("Fixed Timestep sub-stage is not the requested type")
        }
    }
}
