use crate::error::UserError;
use crate::permissions::policy::{Policy, PolicyProvider, Principle};
use crate::util::{Fromi64, Toi64};
use crate::Error;
use entities::sea_orm_active_enums::Effect as DbEffect;
use entities::{channel_policy, guild_policy, member_policy, role_policy};
use poise::serenity_prelude::{ChannelId, GuildId, RoleId, UserId};
use sea_orm::sea_query::OnConflict;
use sea_orm::{
    sea_query, ActiveValue, ColumnTrait, EntityName, EntityTrait, IntoActiveValue, QueryFilter,
};

pub struct PolicyManager {
    db: crate::database::Database,
}

impl PolicyManager {
    pub fn new(db: crate::database::Database) -> Self {
        Self { db }
    }

    pub async fn save_policy(&self, requester: UserId, policy: &Policy) -> Result<(), Error> {
        match policy.principle {
            Principle::Global => {
                let msg = "Changing global bot permissions not currently supported";
                return Err(UserError::invalid_input(requester, msg).into());
            }
            Principle::Guild(guild_id) => self.save_guild_policy(policy.clone(), guild_id).await?,
            Principle::Channel(channel_id) => {
                self.save_channel_policy(policy.clone(), channel_id).await?
            }
            Principle::Role(role_id) => self.save_role_policy(policy.clone(), role_id).await?,
            Principle::Member(guild_id, user_id) => {
                self.save_member_policy(policy.clone(), guild_id, user_id)
                    .await?
            }
        };

        Ok(())
    }

    pub async fn clear_policy(
        &self,
        requester: UserId,
        principle: Principle,
        action: String,
    ) -> Result<(), Error> {
        match principle {
            Principle::Guild(guild_id) => {
                guild_policy::Entity::delete_many()
                    .filter(guild_policy::Column::GuildId.eq(guild_id.to_i64()))
                    .filter(guild_policy::Column::Action.eq(action))
                    .exec(self.db.connection())
                    .await?
            }
            Principle::Channel(channel_id) => {
                channel_policy::Entity::delete_many()
                    .filter(channel_policy::Column::ChannelId.eq(channel_id.to_i64()))
                    .filter(channel_policy::Column::Action.eq(action))
                    .exec(self.db.connection())
                    .await?
            }
            Principle::Role(role_id) => {
                role_policy::Entity::delete_many()
                    .filter(role_policy::Column::RoleId.eq(role_id.to_i64()))
                    .filter(role_policy::Column::Action.eq(action))
                    .exec(self.db.connection())
                    .await?
            }
            Principle::Member(guild_id, user_id) => {
                member_policy::Entity::delete_many()
                    .filter(member_policy::Column::GuildId.eq(guild_id.to_i64()))
                    .filter(member_policy::Column::UserId.eq(user_id.to_i64()))
                    .filter(member_policy::Column::Action.eq(action))
                    .exec(self.db.connection())
                    .await?
            }
            Principle::Global => {
                return Err(
                    UserError::access_denied(requester, "Cannot modify global policies").into(),
                )
            }
        };

        Ok(())
    }

    async fn save_member_policy(
        &self,
        policy: Policy,
        guild_id: GuildId,
        user_id: UserId,
    ) -> Result<(), Error> {
        member_policy::Entity::insert(member_policy::ActiveModel {
            guild_id: guild_id.to_i64().into_active_value(),
            user_id: user_id.to_i64().into_active_value(),
            action: policy.action.into_active_value(),
            effect: ActiveValue::Set(DbEffect::from(policy.effect)),
            until: policy.until.into_active_value(),
            ..Default::default()
        })
        .on_conflict(
            OnConflict::columns([
                member_policy::Column::GuildId,
                member_policy::Column::UserId,
                member_policy::Column::Action,
            ])
            .update_columns([member_policy::Column::Effect, member_policy::Column::Until])
            .to_owned(),
        )
        .exec(self.db.connection())
        .await?;
        Ok(())
    }

    async fn save_role_policy(&self, policy: Policy, role_id: RoleId) -> Result<(), Error> {
        role_policy::Entity::insert(role_policy::ActiveModel {
            role_id: role_id.to_i64().into_active_value(),
            action: policy.action.into_active_value(),
            effect: ActiveValue::Set(DbEffect::from(policy.effect)),
            until: policy.until.into_active_value(),
            ..Default::default()
        })
        .on_conflict(
            OnConflict::columns([role_policy::Column::RoleId, role_policy::Column::Action])
                .update_columns([role_policy::Column::Effect, role_policy::Column::Until])
                .to_owned(),
        )
        .exec(self.db.connection())
        .await?;
        Ok(())
    }

    async fn save_channel_policy(
        &self,
        policy: Policy,
        channel_id: ChannelId,
    ) -> Result<(), Error> {
        channel_policy::Entity::insert(channel_policy::ActiveModel {
            channel_id: channel_id.to_i64().into_active_value(),
            action: policy.action.into_active_value(),
            effect: ActiveValue::Set(DbEffect::from(policy.effect)),
            until: policy.until.into_active_value(),
            ..Default::default()
        })
        .on_conflict(
            OnConflict::columns([
                channel_policy::Column::ChannelId,
                channel_policy::Column::Action,
            ])
            .update_columns([
                channel_policy::Column::Effect,
                channel_policy::Column::Until,
            ])
            .to_owned(),
        )
        .exec(self.db.connection())
        .await?;
        Ok(())
    }

    async fn save_guild_policy(&self, policy: Policy, guild_id: GuildId) -> Result<(), Error> {
        guild_policy::Entity::insert(guild_policy::ActiveModel {
            guild_id: guild_id.to_i64().into_active_value(),
            action: policy.action.into_active_value(),
            effect: ActiveValue::Set(DbEffect::from(policy.effect)),
            until: policy.until.into_active_value(),
            ..Default::default()
        })
        .on_conflict(
            OnConflict::columns([guild_policy::Column::GuildId, guild_policy::Column::Action])
                .update_columns([guild_policy::Column::Effect, guild_policy::Column::Until])
                .to_owned(),
        )
        .exec(self.db.connection())
        .await?;
        Ok(())
    }
}

#[poise::async_trait]
impl PolicyProvider<Error> for PolicyManager {
    async fn guild_policies(
        &self,
        guild_id: GuildId,
        action: String,
    ) -> Result<Vec<Policy>, Error> {
        let policies = guild_policy::Entity::find()
            .filter(build_like(guild_policy::Entity, action))
            .filter(guild_policy::Column::GuildId.eq(guild_id.to_i64()))
            .all(self.db.connection())
            .await?
            .into_iter()
            .map(Policy::from)
            .collect();

        Ok(policies)
    }

    async fn channel_policies(
        &self,
        channel_id: ChannelId,
        action: String,
    ) -> Result<Vec<Policy>, Error> {
        let policies = channel_policy::Entity::find()
            .filter(build_like(channel_policy::Entity, action))
            .filter(channel_policy::Column::ChannelId.eq(channel_id.to_i64()))
            .all(self.db.connection())
            .await?
            .into_iter()
            .map(Policy::from)
            .collect();

        Ok(policies)
    }

    async fn role_policies(&self, role_id: RoleId, action: String) -> Result<Vec<Policy>, Error> {
        let policies = role_policy::Entity::find()
            .filter(build_like(role_policy::Entity, action))
            .filter(role_policy::Column::RoleId.eq(role_id.to_i64()))
            .all(self.db.connection())
            .await?
            .into_iter()
            .map(Policy::from)
            .collect();

        Ok(policies)
    }

    async fn member_policies(
        &self,
        guild_id: GuildId,
        user_id: UserId,
        action: String,
    ) -> Result<Vec<Policy>, Error> {
        let policies = member_policy::Entity::find()
            .filter(build_like(member_policy::Entity, action))
            .filter(member_policy::Column::GuildId.eq(guild_id.to_i64()))
            .filter(member_policy::Column::UserId.eq(user_id.to_i64()))
            .all(self.db.connection())
            .await?
            .into_iter()
            .map(Policy::from)
            .collect();

        Ok(policies)
    }
}

fn build_like(entity: impl EntityName, action: String) -> sea_query::SimpleExpr {
    let expr = format!(r#"$1 LIKE "{}"."action" || '%'"#, entity.table_name());

    sea_query::Expr::cust_with_values(expr, [action])
}

impl From<super::policy::Effect> for entities::sea_orm_active_enums::Effect {
    fn from(value: super::policy::Effect) -> Self {
        match value {
            super::policy::Effect::Allow => Self::Allow,
            super::policy::Effect::Deny => Self::Deny,
        }
    }
}

impl From<entities::sea_orm_active_enums::Effect> for super::policy::Effect {
    fn from(value: entities::sea_orm_active_enums::Effect) -> Self {
        use entities::sea_orm_active_enums::Effect;
        match value {
            Effect::Allow => Self::Allow,
            Effect::Deny => Self::Deny,
        }
    }
}

impl From<guild_policy::Model> for Policy {
    fn from(model: guild_policy::Model) -> Self {
        Self {
            principle: super::policy::Principle::Guild(GuildId::from_i64(model.guild_id)),
            action: model.action,
            effect: super::policy::Effect::from(model.effect),
            until: model.until,
        }
    }
}

impl From<role_policy::Model> for Policy {
    fn from(model: role_policy::Model) -> Self {
        Self {
            principle: super::policy::Principle::Role(RoleId::from_i64(model.role_id)),
            action: model.action,
            effect: super::policy::Effect::from(model.effect),
            until: model.until,
        }
    }
}

impl From<channel_policy::Model> for Policy {
    fn from(model: channel_policy::Model) -> Self {
        Self {
            principle: super::policy::Principle::Channel(ChannelId::from_i64(model.channel_id)),
            action: model.action,
            effect: super::policy::Effect::from(model.effect),
            until: model.until,
        }
    }
}

impl From<member_policy::Model> for Policy {
    fn from(model: member_policy::Model) -> Self {
        Self {
            principle: super::policy::Principle::Member(
                GuildId::from_i64(model.guild_id),
                UserId::from_i64(model.user_id),
            ),
            action: model.action,
            effect: super::policy::Effect::from(model.effect),
            until: model.until,
        }
    }
}
