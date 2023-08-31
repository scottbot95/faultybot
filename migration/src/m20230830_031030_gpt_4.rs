use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::{EnumIter, Iterable};
use crate::extension::postgres::Type;
use crate::m20230806_020929_create_personas;
use crate::m20230806_020929_create_personas::Persona;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        rename_type(manager).await?;

        manager.create_type(
            Type::create()
                .as_enum(LLMModel::Table)
                .values(LLMModel::iter().skip(1))
                .to_owned(),
        ).await?;

        flip_type(manager).await?;

        manager.drop_type(
            Type::drop()
                .name(LLMModelOld::Table)
                .to_owned()
        ).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.exec_stmt(
            Query::update()
                .table(Persona::Table)
                .value(Persona::Model,  Expr::cust("'gpt-3.5-turbo'"))
                .and_where(Expr::col(Persona::Model).ne(Expr::val(LLMModel::Gpt35Turbo.to_string()).as_enum(LLMModel::Table)))
                .to_owned()
        ).await?;

        rename_type(manager).await?;

        manager.create_type(
            Type::create()
                .as_enum(m20230806_020929_create_personas::LLMModel::Table)
                .values(m20230806_020929_create_personas::LLMModel::iter().skip(1))
                .to_owned(),
        ).await?;

        flip_type(manager).await?;

        manager.drop_type(
            Type::drop()
                .name(LLMModelOld::Table)
                .to_owned()
        ).await?;

        Ok(())
    }
}

async fn flip_type<'a>(manager: &SchemaManager<'a>) -> Result<(), DbErr> {
    manager.alter_table(
        Table::alter()
            .table(Persona::Table)
            .modify_column(
                ColumnDef::new(Persona::Model)
                    // .enumeration(LLMModel::Table, LLMModel::iter().skip(1))
                    // .default(LLMModel::Gpt35Turbo.to_string())
                    .extra("ALTER COLUMN model DROP DEFAULT")
                    .extra("ALTER COLUMN model TYPE llm_model USING model::text::llm_model")
                    .default(LLMModel::Gpt35Turbo.to_string())
            )
            .to_owned()
    ).await
}

async fn rename_type<'a>(manager: &SchemaManager<'a>) -> Result<(), DbErr> {
    let db = manager.get_connection();

    // manager.alter_type(
    //     Type::alter()
    //         .name(LLMModel::Table)
    //         .rename_to(LLMModelOld::Table)
    // ).await?;
    // Can't use SeaQuery cuz it's adding extra quotes that break things
    db.execute_unprepared(
        format!("ALTER TYPE {} RENAME TO {}", LLMModel::Table.to_string(), LLMModelOld::Table.to_string()).as_str()
    ).await?;

    Ok(())
}

#[derive(DeriveIden, EnumIter)]
enum LLMModel {
    Table,
    #[sea_orm(iden = "gpt-3.5-turbo")]
    Gpt35Turbo,
    #[sea_orm(iden = "gpt-4")]
    Gpt4
}

#[derive(DeriveIden)]
enum LLMModelOld {
    Table,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let stmt = Table::alter()
            .table(Persona::Table)
            .modify_column(
                ColumnDef::new(Persona::Model)
                    // .enumeration(LLMModel::Table, LLMModel::iter().skip(1))
                    // .default(None)
                    .extra("ALTER COLUMN model DROP DEFAULT")
                    // .extra("ALTER COLUMN model TYPE llm_model USING model::text::status_enum")
            )
            .to_string(PostgresQueryBuilder);

        println!("{}", stmt);

        let stmt = Query::update()
            .table(Persona::Table)
            .value(Persona::Model,  Expr::cust("'gpt-3.5-turbo'"))
            .and_where(Expr::col(Persona::Model).ne(Expr::col(LLMModel::Gpt35Turbo).as_enum(LLMModel::Table)))
            .to_string(PostgresQueryBuilder);

        println!("{}", stmt);
    }
}