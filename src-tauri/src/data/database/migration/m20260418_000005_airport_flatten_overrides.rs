use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(AirportFlattenOverrides::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AirportFlattenOverrides::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(AirportFlattenOverrides::XplanePath)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AirportFlattenOverrides::Icao)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AirportFlattenOverrides::SourcePath)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AirportFlattenOverrides::SourceKind)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AirportFlattenOverrides::SourceLabel)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(AirportFlattenOverrides::FolderName).string())
                    .col(
                        ColumnDef::new(AirportFlattenOverrides::AirportName)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AirportFlattenOverrides::DesiredFlattened)
                            .boolean()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AirportFlattenOverrides::UpdatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("uniq_airport_flatten_override")
                    .table(AirportFlattenOverrides::Table)
                    .col(AirportFlattenOverrides::XplanePath)
                    .col(AirportFlattenOverrides::SourcePath)
                    .col(AirportFlattenOverrides::Icao)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_airport_flatten_override_root")
                    .table(AirportFlattenOverrides::Table)
                    .col(AirportFlattenOverrides::XplanePath)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(AirportFlattenOverrides::Table)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}

#[derive(Iden)]
enum AirportFlattenOverrides {
    Table,
    Id,
    XplanePath,
    Icao,
    SourcePath,
    SourceKind,
    SourceLabel,
    FolderName,
    AirportName,
    DesiredFlattened,
    UpdatedAt,
}
