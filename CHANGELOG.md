# Changelog

Notable user-facing changes with each release version will be described in this file.

## [0.4.0]: 2022-04-16

## Added
 - Extension trait to add ergonomic helpers to `App` for using states.
   - (optional behind `app` feature, adds `bevy_app` dependency)

## Changed
 - Updated for Bevy 0.7

## [0.3.0]: 2022-04-13

### Changed
 - Reverted the `NextState` behavior to how it was in `0.1.x`. The resource has to be inserted/removed.
   In retrospect, this is better UX and avoids bugs.
   - However, support transitioning to the same state as the current.

## [0.2.1]: 2022-04-11

### Added

 - Fixed Timestep: optional EXPERIMENTAL "rate lock" algorithm (see api docs)

## [0.2.0]: 2022-04-06

### Added

 - `ConditionSet`: makes it easy to add run conditions to many systems at once.
 - `FixedTimestepInfo` resource: allows your fixed timestep systems to know about the parameters of the current fixed timestep.

### Changed
 - Behavior of `NextState`: Checked using Bevy Change Detection
   - Present at all times, not removed on state transition.
   - No longer required to be inserted using `Commands`; you can also mutate it directly. Either way works.
   - Supports "transitioning" to the same state as the current, to "reset" it.
 - Conditional systems are now boxed, not generic.

### Removed
 - Conditional systems no longer support `In` and `Out` parameters.

## [0.1.1]: 2022-03-23

### Added
 - Run Condition adapters for compatibility with legacy Bevy States (`.run_in_bevy_state()`/`.run_not_in_bevy_state()`)

### Changed
 - Manually calling `.into_conditional()` on systems, to add conditions, is no longer required.

## [0.1.0]: 2022-03-21

Initial Release

[0.2.0]: https://github.com/IyesGames/iyes_loopless/tree/v0.2.0
[0.1.1]: https://github.com/IyesGames/iyes_loopless/tree/v0.1.1
[0.1.0]: https://github.com/IyesGames/iyes_loopless/tree/v0.1.0
