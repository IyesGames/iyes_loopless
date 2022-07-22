pub mod condition;
#[cfg(feature = "fixedtimestep")]
pub mod fixedtimestep;
#[cfg(feature = "states")]
pub mod state;

pub mod prelude {
    pub use crate::condition::{
        AddConditionalToSet, ConditionHelpers, ConditionSet, IntoConditionalSystem,
    };
    #[cfg(feature = "fixedtimestep")]
    pub use crate::fixedtimestep::{FixedTimestepInfo, FixedTimestepStage};
    #[cfg(feature = "app")]
    pub use crate::state::app::AppLooplessStateExt;
    pub use crate::state::schedule::ScheduleLooplessStateExt;
    #[cfg(feature = "states")]
    pub use crate::state::{
        CurrentState, NextState, StateTransitionStage, StateTransitionStageLabel,
    };
}
