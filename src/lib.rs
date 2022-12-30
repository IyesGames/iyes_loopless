//! Composable Alternatives to Bevy's RunCriteria, States, FixedTimestep
//!
//! This crate offers alternatives to the Run Criteria, States, and FixedTimestep
//! scheduling features currently offered by the Bevy game engine.
//!
//! The ones provided by this crate do not use "looping stages", and can therefore
//! be combined/composed together elegantly, solving some of the most annoying
//! usability limitations of the respective APIs in Bevy.

#![warn(missing_docs)]

pub mod condition;
#[cfg(feature = "fixedtimestep")]
pub mod fixedtimestep;
#[cfg(feature = "states")]
pub mod state;

/// Prelude: convenient import for all the user-facing APIs provided by the crate
pub mod prelude {
    pub use crate::condition::{
        AddConditionalToSet, ConditionHelpers, ConditionSet, IntoConditionalSystem,
    };

    #[cfg(all(feature = "fixedtimestep", feature = "app"))]
    pub use crate::fixedtimestep::app::AppLooplessFixedTimestepExt;
    #[cfg(feature = "fixedtimestep")]
    pub use crate::fixedtimestep::schedule::ScheduleLooplessFixedTimestepExt;
    #[cfg(feature = "fixedtimestep")]
    pub use crate::fixedtimestep::{FixedTimestepStage, FixedTimesteps};

    #[cfg(all(feature = "states", feature = "app"))]
    pub use crate::state::app::AppLooplessStateExt;
    #[cfg(feature = "states")]
    pub use crate::state::schedule::ScheduleLooplessStateExt;
    #[cfg(feature = "states")]
    pub use crate::state::{CurrentState, NextState, QueuedState, StateTransitionStage};
}
