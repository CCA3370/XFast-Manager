use sea_orm_migration::prelude::*;

mod m20260220_000001_init;
mod m20260306_000002_activity_log;
mod m20260306_000003_presets;
mod m20260329_000004_gateway_installs;
mod m20260418_000005_airport_flatten_overrides;
mod m20261124_000006_installation_imports;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260220_000001_init::Migration),
            Box::new(m20260306_000002_activity_log::Migration),
            Box::new(m20260306_000003_presets::Migration),
            Box::new(m20260329_000004_gateway_installs::Migration),
            Box::new(m20260418_000005_airport_flatten_overrides::Migration),
            Box::new(m20261124_000006_installation_imports::Migration),
        ]
    }
}
