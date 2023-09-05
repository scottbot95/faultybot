use entities::sea_orm_active_enums::LlmModel;

macro_rules! resource_kind {
    ($($variant:ident),+) => {
        pub enum ResourceKind {
            $($variant($variant)),+
        }

        impl ResourceKind {
            pub fn matches(&self, other: &Self) -> bool {
                match (self, other) {

                    (_, _) => false
                }
            }
        }

        pub enum ResourceActionKind {
            $($variant(ResourceAction<$variant>)),+
        }
    }
}

resource_kind!(Persona, Permission, Setting, Feedback, Model);

pub struct ResourceAction<R: Resource> {
    pub resource: R,
    pub action: R::Action,
}

// pub enum ResourceKind {
//     Persona(Persona),
//     Permission(Permission),
//     Setting(Setting),
//     Feedback(Feedback),
//     Model(Model),
// }
//
// impl ResourceKind {
//     pub fn matches(&self, other: &ResourceKind) -> bool {
//         match (self, other) {
//             (ResourceKind::Persona(base), ResourceKind::Persona(other)) => false,
//             (_, _) => false
//         }
//     }
// }

fn resource_matches<R: Resource>(base: &R, other: &R) -> bool {
    match (base.specifier(), other.specifier()) {
        (None, _) => true,
        (Some(specifier), Some(other)) => specifier == other,
        (_, _) => false
    }
}

pub trait Resource {
    type Action;
    type Specifier: Eq;

    fn specifier(&self) -> &Option<Self::Specifier>;

    // fn matches<R: Resource>(&self, other: &R) -> bool
    //     where
    //         Self::Specifier: PartialEq<R::Specifier>
    // {
    //     if TypeId::of::<Self>() != TypeId::of::<R>() {
    //         return false;
    //     }
    //
    //     let specifier = self.specifier();
    //     match (specifier, other.specifier()) {
    //         (None, _) => true,
    //         (Some(specifier), Some(other)) => specifier == other,
    //         (_, _) => false
    //     }
    // }
}


#[derive(macros::Resource)]
#[actions(Chat, Create, Edit, Delete)]
pub struct Persona {
    #[specifier]
    name: Option<String>,
}

#[derive(macros::Resource)]
#[actions(Set, Get)]
pub struct Permission {
    #[specifier]
    action: Option<String>, // TODO can we make this use an action type?
}

#[derive(macros::Resource)]
#[actions(Set, Get)]
pub struct Setting {
    #[specifier]
    name: Option<String>, // TODO can we make this use an action type?
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum FeedbackKind {
    Bug,
    Suggestion,
}

#[derive(macros::Resource)]
#[actions(Send)]
pub struct Feedback {
    #[specifier]
    kind: Option<FeedbackKind>,
}

#[derive(macros::Resource)]
#[actions(Use)]
pub struct Model {
    #[specifier]
    kind: Option<LlmModel>,
}
