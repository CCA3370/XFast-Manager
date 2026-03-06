use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(AddonPresets::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AddonPresets::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(AddonPresets::Name)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(AddonPresets::Description).string())
                    .col(
                        ColumnDef::new(AddonPresets::CreatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AddonPresets::UpdatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AddonPresets::Snapshot)
                            .string()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AddonPresets::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(Iden)]
enum AddonPresets {
    Table,
    Id,
    Name,
    Description,
    CreatedAt,
    UpdatedAt,
    Snapshot,
}
