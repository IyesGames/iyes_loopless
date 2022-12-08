# Changelog

Notable user-facing changes with each release version will be described in this file.

## [0.9.1]: 2022-11-20

### Fixed
 - Conditional exclusive systems no longer panic when ran

## [0.9.0]: 2022-11-19

### Changed
 - Bevy 0.9 compatibility

## [0.8.0]: 2022-10-24

### Added
 - All missing API docs
 - Extension traits for a nice API on App/Schedule for working with fixed timesteps, similar to states
 - `FixedTimesteps` resource: allows any system to access the properties of any fixed timestep
 - Fixed timesteps can be paused/unpaused

### Changed
 - `FixedTimestepInfo` is now accessed via a `FixedTimesteps` resource
 - Fixed timestep APIs use string names to identify fixed timesteps
 - Create a conditional exclusive system by calling `.into_conditional_exclusive()`.
   No more conflicting traits. No need for special imports, prelude just works.

### Removed
 - `FixedTimestepInfo` is no longer directly provided as a resource

### Fixed
 - `run_on_event` run condition no longer fires twice under some edge cases
 - WASM compatibility for fixed timestep: use `bevy_utils::Duration` instead of `std::time::Duration`

## [0.7.1]: 2022-08-18

### Added
 - Optional `bevy-inspector-egui` support (thanks @jakobhellermann)

### Changed
 - Using bare system function names with `before`/`after` is now a compile error instead of runtime warning.
   (this was always broken and unsupported)

## [0.7.0]: 2022-07-31

### Added
 - API helper extension methods for `Schedule`, analogous to those for `App`. (thanks @NiklasEi)

### Changed
 - Bevy 0.8 support
 - `FixedTimestepInfo.accumulator` is now `pub`; mutations also affect the internal accumulator

## [0.6.1]: 2022-06-20

### Changed
 - The `step` field in `FixedTimestepInfo` is now `pub`. This was a mistake in 0.6.0.

## [0.6.0]: 2022-06-15

### Added
 - `add_{enter,exit}_system_set` helpers for adding multiple systems to the enter/exit stages of states.
 - `run_if_resource_added` and `run_if_resource_removed` run conditions (thanks @Shatur)

### Changed
 - It is now possible to reconfigure fixed timestep durations at runtime, by modifying the `step` field in `FixedTimestepInfo`.

## [0.5.1]: 2022-04-24

### Added
 - Support for labels/ordering on `ConditionSet`

## [0.5.0]: 2022-04-22

### Added
 - Support for conditional exclusive systems, using the `IntoConditionalExclusiveSystem` trait

### Changed
 - `.add_{enter,exit}_system` App helpers no longer use a `&` reference to the state
 - The `.run_if*` methods are now in trait `ConditionHelpers`, not inherent on the type

## [0.4.0]: 2022-04-16

### Added
 - Extension trait to add ergonomic helpers to `App` for using states.
   - (optional behind `app` feature, adds `bevy_app` dependency)

### Changed
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

[0.9.1]: https://github.com/IyesGames/iyes_loopless/tree/v0.9.1
[0.9.0]: https://github.com/IyesGames/iyes_loopless/tree/v0.9.0
[0.8.0]: https://github.com/IyesGames/iyes_loopless/tree/v0.8.0
[0.7.1]: https://github.com/IyesGames/iyes_loopless/tree/v0.7.1
[0.7.0]: https://github.com/IyesGames/iyes_loopless/tree/v0.7.0
[0.6.1]: https://github.com/IyesGames/iyes_loopless/tree/v0.6.1
[0.6.0]: https://github.com/IyesGames/iyes_loopless/tree/v0.6.0
[0.5.1]: https://github.com/IyesGames/iyes_loopless/tree/v0.5.1
[0.5.0]: https://github.com/IyesGames/iyes_loopless/tree/v0.5.0
[0.4.0]: https://github.com/IyesGames/iyes_loopless/tree/v0.4.0
[0.3.0]: https://github.com/IyesGames/iyes_loopless/tree/v0.3.0
[0.2.1]: https://github.com/IyesGames/iyes_loopless/tree/v0.2.1
[0.2.0]: https://github.com/IyesGames/iyes_loopless/tree/v0.2.0
[0.1.1]: https://github.com/IyesGames/iyes_loopless/tree/v0.1.1
[0.1.0]: https://github.com/IyesGames/iyes_loopless/tree/v0.1.0
