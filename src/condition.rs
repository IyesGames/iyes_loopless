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

use std::borrow::Cow;

use bevy_ecs::{
    event::Events,
    archetype::{Archetype, ArchetypeComponentId},
    component::ComponentId,
    query::Access,
    system::{ConfigurableSystem, IntoSystem, System, Local, Res, Resource},
    world::{FromWorld, World},
};

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

    /// Helper: add a condition to run if there are events of the given type
    pub fn run_on_event<T: Send + Sync + 'static>(self) -> Self {
        self.run_if(on_event::<T>)
    }

    /// Helper: add a condition to run if a resource of a given type exists
    pub fn run_if_resource_exists<T: Resource>(self) -> Self {
        self.run_if(if_resource_exists::<T>)
    }

    /// Helper: add a condition to run if a resource of a given type does not exist
    pub fn run_unless_resource_exists<T: Resource>(self) -> Self {
        self.run_if(unless_resource_exists::<T>)
    }

    /// Helper: add a condition to run if a resource equals the given value
    pub fn run_if_resource_equals<T: Resource + PartialEq + FromWorld>(self, value: T) -> Self {
        self.run_if(if_resource_equals::<T>.config(|c| c.0 = Some(value)))
    }

    /// Helper: add a condition to run if a resource does not equal the given value
    pub fn run_unless_resource_equals<T: Resource + PartialEq + FromWorld>(self, value: T) -> Self {
        self.run_if(unless_resource_equals::<T>.config(|c| c.0 = Some(value)))
    }
}

/// Extension trait allowing any system to be converted into a `ConditionalSystem`
pub trait IntoConditionalSystem<In, Out, Params>: IntoSystem<In, Out, Params> {
    fn into_conditional(self) -> ConditionalSystem<Self::System>;
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

/// Condition for `.run_on_event`
fn on_event<T: Send + Sync + 'static>(events: Res<Events<T>>) -> bool {
    !events.is_empty()
}

/// Condition for `.run_if_resource_exists`
fn if_resource_exists<T: Resource>(res: Option<Res<T>>) -> bool {
    res.is_some()
}

/// Condition for `.run_unless_resource_exists`
fn unless_resource_exists<T: Resource>(res: Option<Res<T>>) -> bool {
    res.is_none()
}

/// Condition for `.run_if_resource_equals`
fn if_resource_equals<T: Resource + PartialEq + FromWorld>(value: Local<T>, res: Option<Res<T>>) -> bool {
    if let Some(res) = res {
        *res == *value
    } else {
        false
    }
}

/// Condition for `.run_unless_resource_equals`
fn unless_resource_equals<T: Resource + PartialEq + FromWorld>(value: Local<T>, res: Option<Res<T>>) -> bool {
    if let Some(res) = res {
        *res != *value
    } else {
        false
    }
}
