//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.1

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "active_persona")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub guild_id: Option<i64>,
    pub channel_id: Option<i64>,
    pub persona_id: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::persona::Entity",
        from = "Column::PersonaId",
        to = "super::persona::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Persona,
}

impl Related<super::persona::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Persona.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
