use sea_orm_migration::prelude::*;

mod m20260220_000001_init;
mod m20260306_000002_activity_log;
mod m20260306_000003_presets;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260220_000001_init::Migration),
            Box::new(m20260306_000002_activity_log::Migration),
            Box::new(m20260306_000003_presets::Migration),
        ]
    }
}
