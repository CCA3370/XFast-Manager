use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ActivityLog::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ActivityLog::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ActivityLog::Timestamp)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(ActivityLog::Operation).string().not_null())
                    .col(ColumnDef::new(ActivityLog::ItemType).string().not_null())
                    .col(ColumnDef::new(ActivityLog::ItemName).string().not_null())
                    .col(ColumnDef::new(ActivityLog::Details).string())
                    .col(
                        ColumnDef::new(ActivityLog::Success)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_activity_log_timestamp")
                    .table(ActivityLog::Table)
                    .col(ActivityLog::Timestamp)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ActivityLog::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(Iden)]
enum ActivityLog {
    Table,
    Id,
    Timestamp,
    Operation,
    ItemType,
    ItemName,
    Details,
    Success,
}
