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
    archetype::ArchetypeComponentId,
    component::ComponentId,
    event::EventReader,
    prelude::Local,
    query::Access,
    schedule::{SystemSet, IntoSystemDescriptor, SystemLabel, ParallelSystemDescriptorCoercion, ParallelSystemDescriptor},
    system::{In, IntoChainSystem, IntoSystem, Res, Resource, System, BoxedSystem, ExclusiveSystem, IntoExclusiveSystem},
    world::World,
};

#[cfg(feature = "states")]
use crate::state::CurrentState;

type BoxedCondition = Box<dyn System<In = (), Out = bool>>;

type SystemLabelApplicator = Box<dyn FnOnce(BevyDescriptorWorkaround) -> BevyDescriptorWorkaround>;

enum BevyDescriptorWorkaround {
    System(ConditionalSystem),
    Descriptor(ParallelSystemDescriptor),
}

impl From<ConditionalSystem> for BevyDescriptorWorkaround {
    fn from(system: ConditionalSystem) -> Self {
        Self::System(system)
    }
}

impl From<ParallelSystemDescriptor> for BevyDescriptorWorkaround {
    fn from(system: ParallelSystemDescriptor) -> Self {
        Self::Descriptor(system)
    }
}

pub struct ConditionalSystemDescriptor {
    system: BoxedSystem,
    conditions: Vec<BoxedCondition>,
    label_shits: Vec<SystemLabelApplicator>,
}

impl ConditionalSystemDescriptor {
    pub fn add_label(&mut self, label: impl SystemLabel) {
        self.label_shits.push(Box::new(move |wa| {
            match wa {
                BevyDescriptorWorkaround::Descriptor(x) => {
                    BevyDescriptorWorkaround::Descriptor(x.label(label))
                }
                BevyDescriptorWorkaround::System(x) => {
                    BevyDescriptorWorkaround::Descriptor(x.label(label))
                }
            }
        }))
    }
    pub fn add_before(&mut self, label: impl SystemLabel) {
        self.label_shits.push(Box::new(move |wa| {
            match wa {
                BevyDescriptorWorkaround::Descriptor(x) => {
                    BevyDescriptorWorkaround::Descriptor(x.before(label))
                }
                BevyDescriptorWorkaround::System(x) => {
                    BevyDescriptorWorkaround::Descriptor(x.before(label))
                }
            }
        }))
    }
    pub fn add_after(&mut self, label: impl SystemLabel) {
        self.label_shits.push(Box::new(move |wa| {
            match wa {
                BevyDescriptorWorkaround::Descriptor(x) => {
                    BevyDescriptorWorkaround::Descriptor(x.after(label))
                }
                BevyDescriptorWorkaround::System(x) => {
                    BevyDescriptorWorkaround::Descriptor(x.after(label))
                }
            }
        }))
    }

    pub fn label(mut self, label: impl SystemLabel) -> Self {
        self.add_label(label);
        self
    }

    pub fn before(mut self, label: impl SystemLabel) -> Self {
        self.add_before(label);
        self
    }

    pub fn after(mut self, label: impl SystemLabel) -> Self {
        self.add_after(label);
        self
    }
}

impl IntoSystemDescriptor<()> for ConditionalSystemDescriptor {
    fn into_descriptor(mut self) -> bevy_ecs::schedule::SystemDescriptor {
        let conditional = ConditionalSystem {
            system: self.system,
            conditions: self.conditions,
            component_access: Default::default(),
            archetype_component_access: Default::default(),
        };

        // NOTE: Bevy has gone to great lengths to make my life painful.
        // We cannot construct a system descriptor directly, because
        // the fields in `bevy_ecs` are not `pub`, and all the traits are
        // crafted in a way that doesn't let us do it easily.
        // We work around this by going in a round-about way via `IntoSystem`.
        // The only way is via `ParallelSystemDescriptorCoercion`, and it does
        // not allow us to create an empty descriptor with just the system,
        // it can only do the conversion by adding a label or something.
        // Hence, this abomination of an implementation:
        let mut bevy_wa;

        // Try pulling out one label from somewhere
        if let Some(appl) = self.label_shits.pop() {
            bevy_wa = appl(conditional.into());
        } else {
            return conditional.into_descriptor();
        }

        for appl in self.label_shits.drain(..) {
            bevy_wa = appl(bevy_wa);
        }

        match bevy_wa {
            BevyDescriptorWorkaround::System(system) => system.into_descriptor(),
            BevyDescriptorWorkaround::Descriptor(descriptor) => descriptor.into_descriptor(),
        }
    }
}

/// Represents an [`ExclusiveSystem`](bevy_ecs::system::ExclusiveSystem) that is governed by Run Condition systems.
///
/// Each condition is a regular system that must return `bool`.
///
/// When ran, it runs as a single aggregate system (similar to Bevy's [`ChainSystem`](bevy_ecs::system::ChainSystem)).
/// It runs every condition system first, and aborts if any of them return `false`.
/// The main system will only run if all the conditions return `true`.
pub struct ConditionalExclusiveSystem {
    system: Box<dyn ExclusiveSystem>,
    conditions: Vec<BoxedCondition>,
}

impl ExclusiveSystem for ConditionalExclusiveSystem {
    fn name(&self) -> Cow<'static, str> {
        self.system.name()
    }

    fn initialize(&mut self, world: &mut World) {
        for condition_system in self.conditions.iter_mut() {
            condition_system.initialize(world);
        }
        self.system.initialize(world);
    }

    fn run(&mut self, world: &mut World) {
        for condition_system in self.conditions.iter_mut() {
            let condition_output;

            // SAFETY: we are in an exclusive system
            // nothing else is running on the World
            unsafe {
                condition_output = condition_system.run_unsafe((), world);
                condition_system.apply_buffers(world);
            };

            if !condition_output {
                return;
            }
        }

        self.system.run(world)
    }

    fn check_change_tick(&mut self, change_tick: u32) {
        for condition_system in self.conditions.iter_mut() {
            condition_system.check_change_tick(change_tick);
        }
        self.system.check_change_tick(change_tick);
    }
}

/// Represents a [`System`](bevy_ecs::system::System) that is governed by Run Condition systems.
///
/// Each condition system must return `bool`.
///
/// This system considers the combined data access of the main system it is based on + all the condition systems.
/// When ran, it runs as a single aggregate system (similar to Bevy's [`ChainSystem`](bevy_ecs::system::ChainSystem)).
/// It runs every condition system first, and aborts if any of them return `false`.
/// The main system will only run if all the conditions return `true`.
pub struct ConditionalSystem {
    system: BoxedSystem,
    conditions: Vec<BoxedCondition>,
    component_access: Access<ComponentId>,
    archetype_component_access: Access<ArchetypeComponentId>,
}

// Based on the implementation of Bevy's ChainSystem
impl System for ConditionalSystem {
    type In = ();
    type Out = ();

    fn name(&self) -> Cow<'static, str> {
        self.system.name()
    }

    fn update_archetype_component_access(&mut self, world: &World) {
        for condition_system in self.conditions.iter_mut() {
            condition_system.update_archetype_component_access(world);
            self.archetype_component_access
                .extend(condition_system.archetype_component_access());
        }
        self.system.update_archetype_component_access(world);
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
                return;
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

impl ConditionHelpers for ConditionalSystemDescriptor {
    /// Builder method for adding more run conditions to a `ConditionalSystem`
    fn run_if<Condition, Params>(mut self, condition: Condition) -> Self
    where
        Condition: IntoSystem<(), bool, Params>,
    {
        let condition_system = <Condition as IntoSystem<(), bool, Params>>::into_system(condition);
        self.conditions.push(Box::new(condition_system));
        self
    }
}

impl ConditionHelpers for ConditionalExclusiveSystem {
    /// Builder method for adding more run conditions to a `ConditionalSystem`
    fn run_if<Condition, Params>(mut self, condition: Condition) -> Self
    where
        Condition: IntoSystem<(), bool, Params>,
    {
        let condition_system = <Condition as IntoSystem<(), bool, Params>>::into_system(condition);
        self.conditions.push(Box::new(condition_system));
        self
    }
}

pub trait ConditionHelpers: Sized {
    fn run_if<Condition, Params>(self, condition: Condition) -> Self
    where
        Condition: IntoSystem<(), bool, Params>;

    /// Helper: add a condition, but flip its result
    fn run_if_not<Condition, Params>(self, condition: Condition) -> Self
    where
        Condition: IntoSystem<(), bool, Params>,
    {
        // PERF: is using system chaining here inefficient?
        self.run_if(condition.chain(move |In(x): In<bool>| !x))
    }

    /// Helper: add a condition to run if there are events of the given type
    fn run_on_event<T: Send + Sync + 'static>(self) -> Self {
        self.run_if(move |mut evr: EventReader<T>| evr.iter().count() > 0)
    }

    /// Helper: add a condition to run if a resource of a given type exists
    fn run_if_resource_exists<T: Resource>(self) -> Self {
        self.run_if(move |res: Option<Res<T>>| res.is_some())
    }

    /// Helper: add a condition to run if a resource of a given type does not exist
    fn run_unless_resource_exists<T: Resource>(self) -> Self {
        self.run_if(move |res: Option<Res<T>>| res.is_none())
    }

    /// Helper: add a condition to run if a resource was added
    fn run_if_resource_added<T: Resource>(self) -> Self {
        self.run_if(move |res: Option<Res<T>>| {
            if let Some(res) = res {
                res.is_added()
            } else {
                false
            }
        })
    }

    /// Helper: add a condition to run if a resource was removed
    fn run_if_resource_removed<T: Resource>(self) -> Self {
        self.run_if(move |mut existed: Local<bool>, res: Option<Res<T>>| {
            if res.is_some() {
                *existed = true;
                false
            } else if *existed {
                *existed = false;
                true
            } else {
                false
            }
        })
    }

    /// Helper: add a condition to run if a resource equals the given value
    fn run_if_resource_equals<T: Resource + PartialEq>(self, value: T) -> Self {
        self.run_if(move |res: Option<Res<T>>| {
            if let Some(res) = res {
                *res == value
            } else {
                false
            }
        })
    }

    /// Helper: add a condition to run if a resource does not equal the given value
    fn run_unless_resource_equals<T: Resource + PartialEq>(self, value: T) -> Self {
        self.run_if(move |res: Option<Res<T>>| {
            if let Some(res) = res {
                *res != value
            } else {
                false
            }
        })
    }

    #[cfg(feature = "states")]
    /// Helper: run in a specific state (checks the [`CurrentState`] resource)
    fn run_in_state<T: bevy_ecs::schedule::StateData>(self, state: T) -> Self {
        self.run_if_resource_equals(CurrentState(state))
    }

    #[cfg(feature = "states")]
    /// Helper: run when not in a specific state (checks the [`CurrentState`] resource)
    fn run_not_in_state<T: bevy_ecs::schedule::StateData>(self, state: T) -> Self {
        self.run_unless_resource_equals(CurrentState(state))
    }

    #[cfg(feature = "bevy-compat")]
    /// Helper: run in a specific Bevy state (checks the `State<T>` resource)
    fn run_in_bevy_state<T: bevy_ecs::schedule::StateData>(self, state: T) -> Self {
        self.run_if(move |res: Option<Res<bevy_ecs::schedule::State<T>>>| {
            if let Some(res) = res {
                res.current() == &state
            } else {
                false
            }
        })
    }

    #[cfg(feature = "bevy-compat")]
    /// Helper: run when not in a specific Bevy state (checks the `State<T>` resource)
    fn run_not_in_bevy_state<T: bevy_ecs::schedule::StateData>(self, state: T) -> Self {
        self.run_if(move |res: Option<Res<bevy_ecs::schedule::State<T>>>| {
            if let Some(res) = res {
                res.current() != &state
            } else {
                false
            }
        })
    }
}

/// Extension trait allowing any system to be converted into a `ConditionalSystem`
pub trait IntoConditionalSystem<Params>: IntoSystem<(), (), Params> + Sized {
    fn into_conditional(self) -> ConditionalSystemDescriptor;

    fn run_if<Condition, CondParams>(self, condition: Condition) -> ConditionalSystemDescriptor
    where
        Condition: IntoSystem<(), bool, CondParams>,
    {
        self.into_conditional().run_if(condition)
    }

    fn run_if_not<Condition, CondParams>(
        self,
        condition: Condition,
    ) -> ConditionalSystemDescriptor
    where
        Condition: IntoSystem<(), bool, CondParams>,
    {
        self.into_conditional().run_if_not(condition)
    }

    fn run_on_event<T: Send + Sync + 'static>(self) -> ConditionalSystemDescriptor {
        self.into_conditional().run_on_event::<T>()
    }

    fn run_if_resource_exists<T: Resource>(self) -> ConditionalSystemDescriptor {
        self.into_conditional().run_if_resource_exists::<T>()
    }

    fn run_unless_resource_exists<T: Resource>(self) -> ConditionalSystemDescriptor {
        self.into_conditional().run_unless_resource_exists::<T>()
    }

    fn run_if_resource_added<T: Resource>(self) -> ConditionalSystemDescriptor {
        self.into_conditional().run_if_resource_added::<T>()
    }

    fn run_if_resource_removed<T: Resource>(self) -> ConditionalSystemDescriptor {
        self.into_conditional().run_if_resource_removed::<T>()
    }

    fn run_if_resource_equals<T: Resource + PartialEq>(
        self,
        value: T,
    ) -> ConditionalSystemDescriptor {
        self.into_conditional().run_if_resource_equals(value)
    }

    fn run_unless_resource_equals<T: Resource + PartialEq>(
        self,
        value: T,
    ) -> ConditionalSystemDescriptor {
        self.into_conditional().run_unless_resource_equals(value)
    }

    #[cfg(feature = "states")]
    fn run_in_state<T: bevy_ecs::schedule::StateData>(
        self,
        state: T,
    ) -> ConditionalSystemDescriptor {
        self.into_conditional().run_in_state(state)
    }

    #[cfg(feature = "states")]
    fn run_not_in_state<T: bevy_ecs::schedule::StateData>(
        self,
        state: T,
    ) -> ConditionalSystemDescriptor {
        self.into_conditional().run_not_in_state(state)
    }

    #[cfg(feature = "bevy-compat")]
    fn run_in_bevy_state<T: bevy_ecs::schedule::StateData>(
        self,
        state: T,
    ) -> ConditionalSystemDescriptor {
        self.into_conditional().run_in_bevy_state(state)
    }

    #[cfg(feature = "bevy-compat")]
    fn run_not_in_bevy_state<T: bevy_ecs::schedule::StateData>(
        self,
        state: T,
    ) -> ConditionalSystemDescriptor {
        self.into_conditional().run_not_in_bevy_state(state)
    }
}

impl<S, Params> IntoConditionalSystem<Params> for S
where
    S: IntoSystem<(), (), Params>,
{
    fn into_conditional(self) -> ConditionalSystemDescriptor {
        ConditionalSystemDescriptor {
            system: Box::new(<Self as IntoSystem<(), (), Params>>::into_system(self)),
            conditions: Vec::new(),
            label_shits: Vec::new(),
        }
    }
}

/// Extension trait for conditional exclusive systems
pub trait IntoConditionalExclusiveSystem<Params, SystemType>: IntoExclusiveSystem<Params, SystemType> + Sized {
    fn into_conditional(self) -> ConditionalExclusiveSystem;

    fn run_if<Condition, CondParams>(self, condition: Condition) -> ConditionalExclusiveSystem
    where
        Condition: IntoSystem<(), bool, CondParams>,
    {
        self.into_conditional().run_if(condition)
    }

    fn run_if_not<Condition, CondParams>(
        self,
        condition: Condition,
    ) -> ConditionalExclusiveSystem
    where
        Condition: IntoSystem<(), bool, CondParams>,
    {
        self.into_conditional().run_if_not(condition)
    }

    fn run_on_event<T: Send + Sync + 'static>(self) -> ConditionalExclusiveSystem {
        self.into_conditional().run_on_event::<T>()
    }

    fn run_if_resource_exists<T: Resource>(self) -> ConditionalExclusiveSystem {
        self.into_conditional().run_if_resource_exists::<T>()
    }

    fn run_unless_resource_exists<T: Resource>(self) -> ConditionalExclusiveSystem {
        self.into_conditional().run_unless_resource_exists::<T>()
    }

    fn run_if_resource_added<T: Resource>(self) -> ConditionalExclusiveSystem {
        self.into_conditional().run_if_resource_added::<T>()
    }

    fn run_if_resource_removed<T: Resource>(self) -> ConditionalExclusiveSystem {
        self.into_conditional().run_if_resource_removed::<T>()
    }

    fn run_if_resource_equals<T: Resource + PartialEq>(
        self,
        value: T,
    ) -> ConditionalExclusiveSystem {
        self.into_conditional().run_if_resource_equals(value)
    }

    fn run_unless_resource_equals<T: Resource + PartialEq>(
        self,
        value: T,
    ) -> ConditionalExclusiveSystem {
        self.into_conditional().run_unless_resource_equals(value)
    }

    #[cfg(feature = "states")]
    fn run_in_state<T: bevy_ecs::schedule::StateData>(
        self,
        state: T,
    ) -> ConditionalExclusiveSystem {
        self.into_conditional().run_in_state(state)
    }

    #[cfg(feature = "states")]
    fn run_not_in_state<T: bevy_ecs::schedule::StateData>(
        self,
        state: T,
    ) -> ConditionalExclusiveSystem {
        self.into_conditional().run_not_in_state(state)
    }

    #[cfg(feature = "bevy-compat")]
    fn run_in_bevy_state<T: bevy_ecs::schedule::StateData>(
        self,
        state: T,
    ) -> ConditionalExclusiveSystem {
        self.into_conditional().run_in_bevy_state(state)
    }

    #[cfg(feature = "bevy-compat")]
    fn run_not_in_bevy_state<T: bevy_ecs::schedule::StateData>(
        self,
        state: T,
    ) -> ConditionalExclusiveSystem {
        self.into_conditional().run_not_in_bevy_state(state)
    }
}

impl<S, Params, SystemType> IntoConditionalExclusiveSystem<Params, SystemType> for S
where
    S: IntoExclusiveSystem<Params, SystemType>,
    SystemType: ExclusiveSystem,
{
    fn into_conditional(self) -> ConditionalExclusiveSystem {
        ConditionalExclusiveSystem {
            system: Box::new(self.exclusive_system()),
            conditions: Vec::new(),
        }
    }
}

pub struct ConditionSet {
    /// "applicator": closure that adds the condition to the system
    conditions: Vec<Box<dyn Fn(&mut ConditionalSystemDescriptor)>>,
    /// label applicator
    labellers: Vec<Box<dyn FnOnce(SystemSet) -> SystemSet>>,
}

pub struct ConditionSystemSet {
    systems: Vec<ConditionalSystemDescriptor>,
    conditions: ConditionSet,
}

impl ConditionSet {
    pub fn new() -> Self {
        Self {
            conditions: Vec::new(),
            labellers: Vec::new(),
        }
    }

    pub fn with_system<S, P>(self, system: S) -> ConditionSystemSet
    where
        S: AddConditionalToSet<ConditionSystemSet, P>,
    {
        let mut csset: ConditionSystemSet = self.into();
        csset.add_system(system);
        csset
    }

    pub fn label(mut self, label: impl SystemLabel) -> Self {
        self.labellers.push(Box::new(move |set: SystemSet| set.label(label)));
        self
    }

    pub fn before(mut self, label: impl SystemLabel) -> Self {
        self.labellers.push(Box::new(move |set: SystemSet| set.before(label)));
        self
    }

    pub fn after(mut self, label: impl SystemLabel) -> Self {
        self.labellers.push(Box::new(move |set: SystemSet| set.after(label)));
        self
    }
}

impl ConditionSystemSet {
    pub fn add_system<S, P>(&mut self, system: S)
    where
        S: AddConditionalToSet<ConditionSystemSet, P>,
    {
        system.add_to_set(self);
    }
    pub fn with_system<S, P>(mut self, system: S) -> Self
    where
        S: AddConditionalToSet<ConditionSystemSet, P>,
    {
        system.add_to_set(&mut self);
        self
    }
}

impl From<ConditionSet> for ConditionSystemSet {
    fn from(cset: ConditionSet) -> ConditionSystemSet {
        ConditionSystemSet {
            systems: Vec::new(),
            conditions: cset,
        }
    }
}

impl From<ConditionSet> for SystemSet {
    fn from(_: ConditionSet) -> SystemSet {
        SystemSet::new()
    }
}

impl From<ConditionSystemSet> for SystemSet {
    fn from(mut csset: ConditionSystemSet) -> SystemSet {
        let mut sset = SystemSet::new();
        for labeller in csset.conditions.labellers.into_iter() {
            sset = labeller(sset);
        }
        for mut system in csset.systems.drain(..) {
            for cond in csset.conditions.conditions.iter() {
                cond(&mut system);
            }
            sset = sset.with_system(system);
        }
        sset
    }
}

pub trait AddConditionalToSet<Set, Params> {
    fn add_to_set(self, set: &mut Set);
}

impl AddConditionalToSet<ConditionSystemSet, ()> for ConditionalSystemDescriptor {
    fn add_to_set(self, set: &mut ConditionSystemSet) {
        set.systems.push(self);
    }
}

impl<System, Params> AddConditionalToSet<ConditionSystemSet, Params> for System
where System: IntoConditionalSystem<Params>,
{
    fn add_to_set(self, set: &mut ConditionSystemSet) {
        set.systems.push(self.into_conditional());
    }
}

impl ConditionSet {
    /// Add a condition to this set, to be applied to all systems
    pub fn run_if<Condition, Params>(mut self, condition: Condition) -> Self
    where
        Condition: IntoSystem<(), bool, Params> + Clone + 'static,
    {
        // create an "applicator" closure, that we can call many times
        // to add the condition to each system
        self.conditions.push(Box::new(move |system| {
            let condition_clone = condition.clone();
            let condition_system = <Condition as IntoSystem<(), bool, Params>>::into_system(condition_clone);
            system.conditions.insert(0, Box::new(condition_system))
        }));
        self
    }

    /// Helper: add a condition, but flip its result
    pub fn run_if_not<Condition, Params>(mut self, condition: Condition) -> Self
    where
        Condition: IntoSystem<(), bool, Params> + Clone + 'static,
    {
        self.conditions.push(Box::new(move |system| {
            let condition_clone = condition.clone();
            // PERF: is using system chaining here inefficient?
            let condition_inverted = condition_clone.chain(move |In(x): In<bool>| !x);
            system.conditions.insert(0, Box::new(condition_inverted))
        }));
        self
    }

    /// Helper: add a condition to run if there are events of the given type
    pub fn run_on_event<T: Send + Sync + 'static>(self) -> Self {
        self.run_if(move |mut evr: EventReader<T>| evr.iter().count() > 0)
    }

    /// Helper: add a condition to run if a resource of a given type exists
    pub fn run_if_resource_exists<T: Resource>(self) -> Self {
        self.run_if(move |res: Option<Res<T>>| res.is_some())
    }

    /// Helper: add a condition to run if a resource of a given type does not exist
    pub fn run_unless_resource_exists<T: Resource>(self) -> Self {
        self.run_if(move |res: Option<Res<T>>| res.is_none())
    }

    /// Helper: add a condition to run if a resource was added
    pub fn run_if_resource_added<T: Resource>(self) -> Self {
        self.run_if(move |res: Option<Res<T>>| {
            if let Some(res) = res {
                res.is_added()
            } else {
                false
            }
        })
    }

    /// Helper: add a condition to run if a resource was removed
    pub fn run_if_resource_removed<T: Resource>(self) -> Self {
        self.run_if(move |mut existed: Local<bool>, res: Option<Res<T>>| {
            if res.is_some() {
                *existed = true;
                false
            } else if *existed {
                *existed = false;
                true
            } else {
                false
            }
        })
    }

    /// Helper: add a condition to run if a resource equals the given value
    pub fn run_if_resource_equals<T: Resource + PartialEq + Clone>(self, value: T) -> Self {
        self.run_if(move |res: Option<Res<T>>| {
            if let Some(res) = res {
                *res == value
            } else {
                false
            }
        })
    }

    /// Helper: add a condition to run if a resource does not equal the given value
    pub fn run_unless_resource_equals<T: Resource + PartialEq + Clone>(self, value: T) -> Self {
        self.run_if(move |res: Option<Res<T>>| {
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

    #[cfg(feature = "bevy-compat")]
    /// Helper: run in a specific Bevy state (checks the `State<T>` resource)
    pub fn run_in_bevy_state<T: bevy_ecs::schedule::StateData>(self, state: T) -> Self {
        self.run_if(move |res: Option<Res<bevy_ecs::schedule::State<T>>>| {
            if let Some(res) = res {
                res.current() == &state
            } else {
                false
            }
        })
    }

    #[cfg(feature = "bevy-compat")]
    /// Helper: run when not in a specific Bevy state (checks the `State<T>` resource)
    pub fn run_not_in_bevy_state<T: bevy_ecs::schedule::StateData>(self, state: T) -> Self {
        self.run_if(move |res: Option<Res<bevy_ecs::schedule::State<T>>>| {
            if let Some(res) = res {
                res.current() != &state
            } else {
                false
            }
        })
    }
}
