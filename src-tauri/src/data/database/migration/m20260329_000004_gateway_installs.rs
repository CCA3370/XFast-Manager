use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(GatewayInstalls::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(GatewayInstalls::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(GatewayInstalls::XplanePath)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(GatewayInstalls::AirportIcao)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(GatewayInstalls::AirportName)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(GatewayInstalls::SceneryId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(GatewayInstalls::FolderName)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(GatewayInstalls::Artist).string())
                    .col(ColumnDef::new(GatewayInstalls::ApprovedDate).string())
                    .col(
                        ColumnDef::new(GatewayInstalls::InstalledAt)
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
                    .name("uniq_gateway_install_airport")
                    .table(GatewayInstalls::Table)
                    .col(GatewayInstalls::XplanePath)
                    .col(GatewayInstalls::AirportIcao)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("uniq_gateway_install_folder")
                    .table(GatewayInstalls::Table)
                    .col(GatewayInstalls::XplanePath)
                    .col(GatewayInstalls::FolderName)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_gateway_install_scenery")
                    .table(GatewayInstalls::Table)
                    .col(GatewayInstalls::XplanePath)
                    .col(GatewayInstalls::SceneryId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(GatewayInstalls::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(Iden)]
enum GatewayInstalls {
    Table,
    Id,
    XplanePath,
    AirportIcao,
    AirportName,
    SceneryId,
    FolderName,
    Artist,
    ApprovedDate,
    InstalledAt,
}
