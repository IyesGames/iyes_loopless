pub mod condition;
#[cfg(feature = "fixedtimestep")]
pub mod fixedtimestep;
#[cfg(feature = "states")]
pub mod state;

pub mod prelude {
    pub use crate::condition::{IntoConditionalSystem, ConditionSet, AddIntoConditional};
    #[cfg(feature = "fixedtimestep")]
    pub use crate::fixedtimestep::FixedTimestepStage;
    #[cfg(feature = "states")]
    pub use crate::state::{CurrentState, NextState, StateTransitionStage};
}
