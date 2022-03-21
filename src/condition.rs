//! Conditional systems supporting multiple run conditions
//!
//! This is an alternative to Bevy's Run Criteria, inspired by Bevy's `ChainSystem` and the Stageless RFC.
//!
//! Run Conditions are systems that return `bool`.
//!
//! Any system can be converted into a `ConditionalSystem` (by calling `.into_conditional()`), allowing
//! run conditions to be added to it. You can add as many run conditions as you want to a `ConditionalSystem`,
//! by using the `.run_if(condition)` builder method.
//!
//! The `ConditionalSystem` with all its conditions behaves like a single system at runtime.
//! All of the data access (from all the system params) will be combined together.
//! When it runs, it will run each condition, and abort if any of them returns `false`.
//! The main system will only run if all conditions return `true`.
//!
//! It is highly recommended that all your conditions only access data
//! immutably. Avoid mutable access or locals in condition systems, unless are
//! really sure about what you are doing. If you add the same condition to many
//! systems, it *will run with each one*.
//!

use std::borrow::Cow;

use bevy_ecs::{
    event::Events,
    archetype::{Archetype, ArchetypeComponentId},
    component::ComponentId,
    query::Access,
    system::{IntoSystem, IntoChainSystem, In, System, Res, Resource},
    world::World,
};

#[cfg(feature = "states")]
use crate::state::CurrentState;

/// Represents a [`System`](bevy_ecs::system::System) that runs conditionally, based on any number of Run Condition systems.
///
/// Each conditions system must return `bool`.
///
/// This system considers the combined data access of the main system it is based on + all the condition systems.
/// When ran, it runs as a single aggregate system (similar to Bevy's [`ChainSystem`](bevy_ecs::system::ChainSystem)).
/// It runs every condition system first, and aborts if any of them return `false`.
/// The main system will only run if all the conditions return `true`.
pub struct ConditionalSystem<S: System> {
    system: S,
    conditions: Vec<Box<dyn System<In = (), Out = bool>>>,
    component_access: Access<ComponentId>,
    archetype_component_access: Access<ArchetypeComponentId>,
}

// Based on the implementation of Bevy's ChainSystem
impl<Out: Default, S: System<Out = Out>> System for ConditionalSystem<S> {
    type In = S::In;
    type Out = Out;

    fn name(&self) -> Cow<'static, str> {
        self.system.name()
    }

    fn new_archetype(&mut self, archetype: &Archetype) {
        for condition_system in self.conditions.iter_mut() {
            condition_system.new_archetype(archetype);
            self.archetype_component_access
                .extend(condition_system.archetype_component_access());
        }
        self.system.new_archetype(archetype);
        self.archetype_component_access
            .extend(self.system.archetype_component_access());
    }

    fn component_access(&self) -> &Access<ComponentId> {
        &self.component_access
    }

    fn archetype_component_access(&self) -> &Access<ArchetypeComponentId> {
        &self.archetype_component_access
    }

    fn is_send(&self) -> bool {
        let conditions_are_send = self.conditions.iter().all(|system| system.is_send());
        self.system.is_send() && conditions_are_send
    }

    unsafe fn run_unsafe(&mut self, input: Self::In, world: &World) -> Self::Out {
        for condition_system in self.conditions.iter_mut() {
            if !condition_system.run_unsafe((), world) {
                return Out::default();
            }
        }
        self.system.run_unsafe(input, world)
    }

    fn apply_buffers(&mut self, world: &mut World) {
        for condition_system in self.conditions.iter_mut() {
            condition_system.apply_buffers(world);
        }
        self.system.apply_buffers(world);
    }

    fn initialize(&mut self, world: &mut World) {
        for condition_system in self.conditions.iter_mut() {
            condition_system.initialize(world);
            self.component_access
                .extend(condition_system.component_access());
        }
        self.system.initialize(world);
        self.component_access.extend(self.system.component_access());
    }

    fn check_change_tick(&mut self, change_tick: u32) {
        for condition_system in self.conditions.iter_mut() {
            condition_system.check_change_tick(change_tick);
        }
        self.system.check_change_tick(change_tick);
    }
}

impl<S: System> ConditionalSystem<S> {
    /// Builder method for adding more run conditions to a `ConditionalSystem`
    pub fn run_if<Condition, Params>(mut self, condition: Condition) -> Self
        where Condition: IntoSystem<(), bool, Params>,
    {
        let condition_system = condition.system();
        self.conditions.push(Box::new(condition_system));
        self
    }

    /// Helper: add a condition, but flip its result
    pub fn run_if_not<Condition, Params>(self, condition: Condition) -> Self
        where Condition: IntoSystem<(), bool, Params>,
    {
        // PERF: is using system chaining here inefficient?
        self.run_if(condition.chain(move |In(x): In<bool>,| !x))
    }

    /// Helper: add a condition to run if there are events of the given type
    pub fn run_on_event<T: Send + Sync + 'static>(self) -> Self {
        self.run_if(move | ev: Res<Events<T>> | !ev.is_empty())
    }

    /// Helper: add a condition to run if a resource of a given type exists
    pub fn run_if_resource_exists<T: Resource>(self) -> Self {
        self.run_if(move | res: Option<Res<T>> | res.is_some())
    }

    /// Helper: add a condition to run if a resource of a given type does not exist
    pub fn run_unless_resource_exists<T: Resource>(self) -> Self {
        self.run_if(move | res: Option<Res<T>> | res.is_none())
    }

    /// Helper: add a condition to run if a resource equals the given value
    pub fn run_if_resource_equals<T: Resource + PartialEq>(self, value: T) -> Self {
        self.run_if(move | res: Option<Res<T>> | {
            if let Some(res) = res {
                *res == value
            } else {
                false
            }
        })
    }

    /// Helper: add a condition to run if a resource does not equal the given value
    pub fn run_unless_resource_equals<T: Resource + PartialEq>(self, value: T) -> Self {
        self.run_if(move | res: Option<Res<T>> | {
            if let Some(res) = res {
                *res != value
            } else {
                false
            }
        })
    }

    #[cfg(feature = "states")]
    /// Helper: run in a specific state (checks the [`CurrentState`] resource)
    pub fn run_in_state<T: bevy_ecs::schedule::StateData>(self, state: T) -> Self {
        self.run_if_resource_equals(CurrentState(state))
    }

    #[cfg(feature = "states")]
    /// Helper: run when not in a specific state (checks the [`CurrentState`] resource)
    pub fn run_not_in_state<T: bevy_ecs::schedule::StateData>(self, state: T) -> Self {
        self.run_unless_resource_equals(CurrentState(state))
    }
}

/// Extension trait allowing any system to be converted into a `ConditionalSystem`
pub trait IntoConditionalSystem<In, Out, Params>: IntoSystem<In, Out, Params> + Sized {
    fn into_conditional(self) -> ConditionalSystem<Self::System>;

    fn run_if<Condition, CondParams>(self, condition: Condition) -> ConditionalSystem<Self::System>
        where Condition: IntoSystem<(), bool, CondParams>,
    {
        self.into_conditional()
            .run_if(condition)
    }

    fn run_if_not<Condition, CondParams>(self, condition: Condition) -> ConditionalSystem<Self::System>
        where Condition: IntoSystem<(), bool, CondParams>,
    {
        self.into_conditional()
            .run_if_not(condition)
    }

    fn run_on_event<T: Send + Sync + 'static>(self) -> ConditionalSystem<Self::System>
    {
        self.into_conditional()
            .run_on_event::<T>()
    }

    fn run_if_resource_exists<T: Resource>(self) -> ConditionalSystem<Self::System>
    {
        self.into_conditional()
            .run_if_resource_exists::<T>()
    }

    fn run_unless_resource_exists<T: Resource>(self) -> ConditionalSystem<Self::System>
    {
        self.into_conditional()
            .run_unless_resource_exists::<T>()
    }

    fn run_if_resource_equals<T: Resource + PartialEq>(self, value: T) -> ConditionalSystem<Self::System>
    {
        self.into_conditional()
            .run_if_resource_equals(value)
    }

    fn run_unless_resource_equals<T: Resource + PartialEq>(self, value: T) -> ConditionalSystem<Self::System>
    {
        self.into_conditional()
            .run_unless_resource_equals(value)
    }

    #[cfg(feature = "states")]
    fn run_in_state<T: bevy_ecs::schedule::StateData>(self, state: T) -> ConditionalSystem<Self::System>
    {
        self.into_conditional()
            .run_in_state(state)
    }

    #[cfg(feature = "states")]
    fn run_not_in_state<T: bevy_ecs::schedule::StateData>(self, state: T) -> ConditionalSystem<Self::System>
    {
        self.into_conditional()
            .run_not_in_state(state)
    }

}

impl<S, In, Out, Params> IntoConditionalSystem<In, Out, Params> for S
    where S: IntoSystem<In, Out, Params>,
{
    fn into_conditional(self) -> ConditionalSystem<Self::System> {
        ConditionalSystem {
            system: self.system(),
            conditions: Vec::new(),
            archetype_component_access: Default::default(),
            component_access: Default::default(),
        }
    }
}
