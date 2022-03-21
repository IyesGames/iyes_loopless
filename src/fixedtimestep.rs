use std::time::Duration;

use bevy_core::Time;
use bevy_ecs::prelude::*;

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
}

impl FixedTimestepStage {
    /// Helper to create a `FixedTimestepStage` with a single child stage
    pub fn from_stage<S: Stage>(timestep: Duration, stage: S) -> Self {
        Self {
            step: timestep,
            accumulator: Duration::default(),
            stages: vec![Box::new(stage)],
        }
    }

    /// Create a new empty `FixedTimestepStage` with no child stages
    pub fn new(timestep: Duration) -> Self {
        Self {
            step: timestep,
            accumulator: Duration::default(),
            stages: Vec::new(),
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

        while self.accumulator >= self.step {
            self.accumulator -= self.step;
            for stage in self.stages.iter_mut() {
                stage.run(world);
            }
        }
    }
}
