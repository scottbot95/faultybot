//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.1

use super::sea_orm_active_enums::Effect;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "guild_policy")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub guild_id: i64,
    pub action: String,
    pub effect: Effect,
    pub until: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
