use sea_orm_migration::{prelude::*, schema::*, sea_orm::{sqlx::RawSql, DbBackend, Statement, StatementBuilder}};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Network::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Network::Id).uuid().not_null().primary_key())
                    .col(
                        ColumnDef::new(Network::VrfRouteTableId)
                            .integer()
                            .auto_increment()
                            .not_null()
                            .unique_key(),
                    )
                    .to_owned(),
            )
            .await?;
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Network::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Network {
    Table,
    Id,
    VrfRouteTableId,
}
