use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::StatementBuilder;
use crate::m20230806_020929_create_personas::{ActivePersona, Persona};

#[derive(DeriveMigrationName)]
pub struct Migration;

const SASSY_PROMPT: &str =
    "You are {bot_name}, a helpful assistant built into a Discord bot.
     You are helpful, but your responses are always sassy and sometimes rude.";
const SASSY_DESCRIPTION: &str = "A somewhat snarky persona";

const CLEAN_PROMPT: &str =
    "You are {bot_name}, a helpful assistant built into a Discord bot.
     You are always kinda and polite with your users.";
const CLEAN_DESCRIPTION: &str = "A clean and polite persona";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let insert = Query::insert()
            .into_table(Persona::Table)
            .columns([Persona::Name, Persona::Builtin, Persona::Description, Persona::Prompt])
            .values_panic(["Sassy".into(), true.into(), SASSY_DESCRIPTION.into(), SASSY_PROMPT.into()])
            .returning_col(Persona::Id)
            .to_owned();

        let stmt = StatementBuilder::build(&insert, &manager.get_database_backend());
        let result = manager.get_connection()
            .query_one(stmt)
            .await?
            .ok_or_else(|| DbErr::Custom("Failed to create default persona".to_string()))?;

        let default_id: i32 = result.try_get("", Persona::Id.to_string().as_str())?;

        let insert_default_active = Query::insert()
            .into_table(ActivePersona::Table)
            .columns([ActivePersona::PersonaId])
            .values_panic([default_id.into()])
            .to_owned();

        manager.exec_stmt(insert_default_active).await?;

        let insert_other_personas = Query::insert()
            .into_table(Persona::Table)
            .columns([Persona::Name, Persona::Builtin, Persona::Description, Persona::Prompt])
            .values_panic(["Clean".into(), true.into(), CLEAN_DESCRIPTION.into(), CLEAN_PROMPT.into()])
            .to_owned();

        manager.exec_stmt(insert_other_personas).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let stmt = Query::delete()
            .from_table(Persona::Table)
            .and_where(Expr::col(Persona::Builtin).eq(false))
            .to_owned();

        manager.exec_stmt(stmt).await?;

        Ok(())
    }
}
