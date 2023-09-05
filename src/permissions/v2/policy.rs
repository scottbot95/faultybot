use crate::error::UserError;
use crate::permissions::v2::Principle;
use crate::permissions::v2::Resource;
use crate::permissions::v2::resource::*;

pub struct Policy {
    pub principle: Principle,
    pub effect: Effect,
    pub kind: PolicyKind,
}

impl Policy {
    pub fn test(&self, resource: ResourceKind) -> bool {
        todo!()
    }
}

#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct ResourcePolicy<T: Resource> {
    pub resource: T,
    pub actions: Vec<T::Action>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Effect {
    Allow,
    Deny
}

impl Default for Effect {
    fn default() -> Self {
        Self::Deny
    }
}

pub enum AuthDecision {
    Allow,
    Deny,
    NoOpinion,
}

impl<T: Resource> ResourcePolicy<T> {
    // pub fn assert_access(&self, resource: T, action: T::Action) -> Result<(), UserError> {
    //     match self.test(resource, action) {
    //         AuthDecision::Allow => Ok(()),
    //         AuthDecision::Deny => {
    //             let msg = format!("Access denied to {}", "thing");
    //             Err(UserError::access_denied(msg))
    //         }
    //         AuthDecision::NoOpinion => Ok(())
    //     }
    // }
}

macro_rules! policy_kind {
    ($($variant:ident),+) => {
        pub enum PolicyKind {
            $($variant(ResourcePolicy<$variant>)),*
        }

        impl PolicyKind {
            fn matches(&self, other: &Self) -> bool {
                match (self, other) {
                    // $((Self::$variant(_self), Self::$variant(_other)) => _self.matches(_other)),*
                    (_, _) => false,
                }
            }
        }
    }
}

policy_kind!(Persona, Permission, Setting, Feedback, Model);
