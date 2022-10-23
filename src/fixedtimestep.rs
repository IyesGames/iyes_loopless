use bevy_time::Time;
use bevy_utils::Duration;
use bevy_utils::HashMap;

use bevy_ecs::prelude::*;

pub type TimestepLabel = &'static str;

#[derive(Default)]
pub struct FixedTimesteps {
    info: HashMap<TimestepLabel, FixedTimestepInfo>,
    current: Option<TimestepLabel>,
}

impl FixedTimesteps {
    /// Returns a reference to the timestep info for a given timestep by name.
    pub fn get(&self, label: TimestepLabel) -> Option<&FixedTimestepInfo> {
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
    pub fn get_mut(&mut self, label: TimestepLabel) -> Option<&mut FixedTimestepInfo> {
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

/// Provides info about the current status of the fixed timestep
///
/// This type will be available for each fixed timestep stage inside [`FixedTimesteps`] resource
/// starting from first timestep run.
///
/// If you modify the step value, the fixed timestep driver stage will
/// reconfigure itself to respect it. Your new timestep duration will be
/// used starting from the next update cycle.
pub struct FixedTimestepInfo {
    pub step: Duration,
    pub accumulator: Duration,
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

    pub fn pause(&mut self) {
        self.paused = true;
    }

    pub fn unpause(&mut self) {
        self.paused = false;
    }

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
    label: TimestepLabel,
    stages: Vec<Box<dyn Stage>>,
    rate_lock: (u32, f32),
    lock_accum: u32,
}

impl FixedTimestepStage {
    /// Helper to create a `FixedTimestepStage` with a single child stage
    pub fn from_stage<S: Stage>(timestep: Duration, label: TimestepLabel, stage: S) -> Self {
        Self::new(timestep, label).with_stage(stage)
    }

    /// Create a new empty `FixedTimestepStage` with no child stages
    pub fn new(timestep: Duration, label: TimestepLabel) -> Self {
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

#[derive(Debug, Clone)]
pub struct FixedTimestepStageLabel(TimestepLabel);

impl StageLabel for FixedTimestepStageLabel {
    fn as_str(&self) -> &'static str {
        self.0
    }
}

#[cfg(feature = "app")]
pub mod app {
    use bevy_utils::Duration;
    use bevy_ecs::prelude::*;
    use bevy_ecs::schedule::IntoSystemDescriptor;
    use bevy_app::{App, CoreStage};

    use super::{FixedTimestepStage, FixedTimestepStageLabel, TimestepLabel};

    pub trait AppLooplessFixedTimestepExt {
        fn add_fixed_timestep(&mut self, timestep: Duration, label: TimestepLabel) -> &mut App;
        fn add_fixed_timestep_before_stage(&mut self, stage: impl StageLabel, timestep: Duration, label: TimestepLabel) -> &mut App;
        fn add_fixed_timestep_after_stage(&mut self, stage: impl StageLabel, timestep: Duration, label: TimestepLabel) -> &mut App;
        fn add_fixed_timestep_child_stage(&mut self, timestep_label: TimestepLabel) -> &mut App;
        fn add_fixed_timestep_custom_child_stage(&mut self, timestep_label: TimestepLabel, stage: impl Stage) -> &mut App;
        fn add_fixed_timestep_system<Params>(&mut self, timestep_label: TimestepLabel, substage_i: usize, system: impl IntoSystemDescriptor<Params>) -> &mut App;
        fn add_fixed_timestep_system_set(&mut self, timestep_label: TimestepLabel, substage_i: usize, system_set: SystemSet) -> &mut App;
    }

    impl AppLooplessFixedTimestepExt for App {
        fn add_fixed_timestep(&mut self, timestep: Duration, label: TimestepLabel) -> &mut App {
            self.add_fixed_timestep_before_stage(CoreStage::Update, timestep, label)
        }

        fn add_fixed_timestep_before_stage(&mut self, stage: impl StageLabel, timestep: Duration, label: TimestepLabel) -> &mut App {
            let ftstage = FixedTimestepStage::from_stage(timestep, label, SystemStage::parallel());
            ftstage.store_fixedtimestepinfo(&mut self.world);
            self.add_stage_before(
                stage,
                FixedTimestepStageLabel(label),
                ftstage
            )
        }

        fn add_fixed_timestep_after_stage(&mut self, stage: impl StageLabel, timestep: Duration, label: TimestepLabel) -> &mut App {
            let ftstage = FixedTimestepStage::from_stage(timestep, label, SystemStage::parallel());
            ftstage.store_fixedtimestepinfo(&mut self.world);
            self.add_stage_after(
                stage,
                FixedTimestepStageLabel(label),
                ftstage
            )
        }

        fn add_fixed_timestep_child_stage(&mut self, timestep_label: TimestepLabel) -> &mut App {
            let stage = self.schedule.get_stage_mut::<FixedTimestepStage>(
                &FixedTimestepStageLabel(timestep_label)
            ).expect("Fixed Timestep Stage not found");
            stage.add_stage(SystemStage::parallel());
            self
        }

        fn add_fixed_timestep_custom_child_stage(&mut self, timestep_label: TimestepLabel, custom_stage: impl Stage) -> &mut App {
            let stage = self.schedule.get_stage_mut::<FixedTimestepStage>(
                &FixedTimestepStageLabel(timestep_label)
            ).expect("Fixed Timestep Stage not found");
            stage.add_stage(custom_stage);
            self
        }

        fn add_fixed_timestep_system<Params>(&mut self, timestep_label: TimestepLabel, substage_i: usize, system: impl IntoSystemDescriptor<Params>) -> &mut App {
            let stage = self.schedule.get_stage_mut::<FixedTimestepStage>(
                &FixedTimestepStageLabel(timestep_label)
            ).expect("Fixed Timestep Stage not found");
            let substage = stage.stages.get_mut(substage_i)
                .expect("Fixed Timestep sub-stage not found")
                .downcast_mut::<SystemStage>()
                .expect("Fixed Timestep sub-stage is not a SystemStage");
            substage.add_system(system);
            self
        }

        fn add_fixed_timestep_system_set(&mut self, timestep_label: TimestepLabel, substage_i: usize, system_set: SystemSet) -> &mut App {
            let stage = self.schedule.get_stage_mut::<FixedTimestepStage>(
                &FixedTimestepStageLabel(timestep_label)
            ).expect("Fixed Timestep Stage not found");
            let substage = stage.stages.get_mut(substage_i)
                .expect("Fixed Timestep sub-stage not found")
                .downcast_mut::<SystemStage>()
                .expect("Fixed Timestep sub-stage is not a SystemStage");
            substage.add_system_set(system_set);
            self
        }
    }
}

pub mod schedule {
    use bevy_utils::Duration;
    use bevy_ecs::prelude::*;
    use bevy_ecs::schedule::IntoSystemDescriptor;

    use super::{FixedTimestepStage, FixedTimestepStageLabel, TimestepLabel};

    pub trait ScheduleLooplessFixedTimestepExt {
        fn add_fixed_timestep_before_stage(&mut self, stage: impl StageLabel, timestep: Duration, label: TimestepLabel) -> &mut Schedule;
        fn add_fixed_timestep_after_stage(&mut self, stage: impl StageLabel, timestep: Duration, label: TimestepLabel) -> &mut Schedule;
        fn add_fixed_timestep_child_stage(&mut self, timestep_label: TimestepLabel) -> &mut Schedule;
        fn add_fixed_timestep_custom_child_stage(&mut self, timestep_label: TimestepLabel, stage: impl Stage) -> &mut Schedule;
        fn add_fixed_timestep_system<Params>(&mut self, timestep_label: TimestepLabel, substage_i: usize, system: impl IntoSystemDescriptor<Params>) -> &mut Schedule;
        fn add_fixed_timestep_system_set(&mut self, timestep_label: TimestepLabel, substage_i: usize, system_set: SystemSet) -> &mut Schedule;
    }

    impl ScheduleLooplessFixedTimestepExt for Schedule {
        fn add_fixed_timestep_before_stage(&mut self, stage: impl StageLabel, timestep: Duration, label: TimestepLabel) -> &mut Schedule {
            self.add_stage_before(
                stage,
                FixedTimestepStageLabel(label),
                FixedTimestepStage::from_stage(timestep, label, SystemStage::parallel())
            )
        }

        fn add_fixed_timestep_after_stage(&mut self, stage: impl StageLabel, timestep: Duration, label: TimestepLabel) -> &mut Schedule {
            self.add_stage_after(
                stage,
                FixedTimestepStageLabel(label),
                FixedTimestepStage::from_stage(timestep, label, SystemStage::parallel())
            )
        }

        fn add_fixed_timestep_child_stage(&mut self, timestep_label: TimestepLabel) -> &mut Schedule {
            let stage = self.get_stage_mut::<FixedTimestepStage>(
                &FixedTimestepStageLabel(timestep_label)
            ).expect("Fixed Timestep Stage not found");
            stage.add_stage(SystemStage::parallel());
            self
        }

        fn add_fixed_timestep_custom_child_stage(&mut self, timestep_label: TimestepLabel, custom_stage: impl Stage) -> &mut Schedule {
            let stage = self.get_stage_mut::<FixedTimestepStage>(
                &FixedTimestepStageLabel(timestep_label)
            ).expect("Fixed Timestep Stage not found");
            stage.add_stage(custom_stage);
            self
        }

        fn add_fixed_timestep_system<Params>(&mut self, timestep_label: TimestepLabel, substage_i: usize, system: impl IntoSystemDescriptor<Params>) -> &mut Schedule {
            let stage = self.get_stage_mut::<FixedTimestepStage>(
                &FixedTimestepStageLabel(timestep_label)
            ).expect("Fixed Timestep Stage not found");
            let substage = stage.stages.get_mut(substage_i)
                .expect("Fixed Timestep sub-stage not found")
                .downcast_mut::<SystemStage>()
                .expect("Fixed Timestep sub-stage is not a SystemStage");
            substage.add_system(system);
            self
        }

        fn add_fixed_timestep_system_set(&mut self, timestep_label: TimestepLabel, substage_i: usize, system_set: SystemSet) -> &mut Schedule {
            let stage = self.get_stage_mut::<FixedTimestepStage>(
                &FixedTimestepStageLabel(timestep_label)
            ).expect("Fixed Timestep Stage not found");
            let substage = stage.stages.get_mut(substage_i)
                .expect("Fixed Timestep sub-stage not found")
                .downcast_mut::<SystemStage>()
                .expect("Fixed Timestep sub-stage is not a SystemStage");
            substage.add_system_set(system_set);
            self
        }
    }
}
