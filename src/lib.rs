pub mod condition;
#[cfg(feature = "fixedtimestep")]
pub mod fixedtimestep;
#[cfg(feature = "states")]
pub mod state;

pub mod prelude {
    pub use crate::condition::{ConditionHelpers, IntoConditionalSystem, ConditionSet, AddConditionalToSet};

    #[cfg(feature = "fixedtimestep")]
    pub use crate::fixedtimestep::{FixedTimestepInfo, FixedTimestepStage};
    #[cfg(feature = "fixedtimestep")]
    pub use crate::fixedtimestep::schedule::ScheduleLooplessFixedTimestepExt;
    #[cfg(all(feature = "fixedtimestep", feature = "app"))]
    pub use crate::fixedtimestep::app::AppLooplessFixedTimestepExt;

    #[cfg(feature = "states")]
    pub use crate::state::{CurrentState, NextState, StateTransitionStage};
    #[cfg(feature = "states")]
    pub use crate::state::schedule::ScheduleLooplessStateExt;
    #[cfg(all(feature = "states", feature = "app"))]
    pub use crate::state::app::AppLooplessStateExt;
}
