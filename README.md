# Composable Alternatives to Bevy's RunCriteria, States, FixedTimestep

This crate offers alternatives to the Run Criteria, States, and FixedTimestep
scheduling features currently offered by the Bevy game engine.

The ones provided by this crate do not use "looping stages", and can therefore
be combined/composed together elegantly, solving some of the most annoying
usability limitations of the respective APIs in Bevy.

## How does this relate to the Bevy Stageless RFC?

This crate draws *very heavy* inspiration from the ["Stageless
RFC"](https://github.com/bevyengine/rfcs/pull/45) proposal for Bevy.

Big thanks to all the authors that have worked on that RFC and the designs
described there.

I am making this crate, because I believe the APIs currently in Bevy are
sorely in need of a usability improvement.

I figured out a way to implement the ideas from the Stageless RFC in a way
that works within the existing framework of current Bevy, without requiring
the complete scheduling API overhaul that the RFC proposes.

This way we can have something usable *now*, while the remaining Stageless
work is still in progress.
