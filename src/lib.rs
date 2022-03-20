pub mod condition;
#[cfg(feature = "fixedtimestep")]
pub mod fixedtimestep;

pub mod prelude {
    pub use crate::condition::IntoConditionalSystem;
    #[cfg(feature = "fixedtimestep")]
    pub use crate::fixedtimestep::FixedTimestepStage;
}
