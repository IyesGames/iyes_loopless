use std::time::Duration;

use bevy_core::Time;
use bevy_ecs::prelude::*;

/// This type will be available as a resource, while a fixed timestep stage
/// runs, to provide info about the current status of the fixed timestep.
pub struct FixedTimestepInfo {
    step: Duration,
    accumulator: Duration,
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
    stages: Vec<Box<dyn Stage>>,
    rate_lock: (u32, f32),
    lock_accum: u32,
}

impl FixedTimestepStage {
    /// Helper to create a `FixedTimestepStage` with a single child stage
    pub fn from_stage<S: Stage>(timestep: Duration, stage: S) -> Self {
        Self::new(timestep).with_stage(stage)
    }

    /// Create a new empty `FixedTimestepStage` with no child stages
    pub fn new(timestep: Duration) -> Self {
        Self {
            step: timestep,
            accumulator: Duration::default(),
            stages: Vec::new(),
            rate_lock: (u32::MAX, 0.0),
            lock_accum: 0,
        }
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
}

impl Stage for FixedTimestepStage {
    fn run(&mut self, world: &mut World) {
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
            for stage in self.stages.iter_mut() {
                world.insert_resource(FixedTimestepInfo {
                    step: self.step,
                    accumulator: self.accumulator,
                });
                stage.run(world);
                world.remove_resource::<FixedTimestepInfo>();
            }
            n_steps += 1;
        }

        if n_steps == 1 && self.lock_accum < self.rate_lock.0 {
            self.lock_accum += 1;
            self.accumulator = self.step / 2;
        }
    }
}
