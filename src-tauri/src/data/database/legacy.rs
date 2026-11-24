use crate::error::ApiError;
use sea_orm::sea_query::OnConflict;
use sea_orm::{
    ActiveValue, ColumnTrait, ConnectionTrait, DatabaseBackend, DatabaseConnection, EntityTrait,
    QueryFilter, QueryOrder, Set, Statement, TransactionTrait,
};
use sea_orm_migration::SchemaManager;
use std::collections::HashMap;
use std::path::Path;

use super::entities::{airport_flatten_overrides as flatten, gateway_installs as gateway};
use super::{connection, root_identity};

pub(super) async fn mark_imported(conn: &impl ConnectionTrait) -> Result<(), ApiError> {
    conn.execute_unprepared(
        "INSERT OR IGNORE INTO installation_imports (name) VALUES ('root_records')",
    )
    .await?;
    Ok(())
}

fn owned_by_root(path: &str, identity: &str) -> bool {
    // Gateway stored a canonical Windows path with forward slashes and folded case.
    #[cfg(windows)]
    let windows_path = path.replace('/', "\\");
    #[cfg(windows)]
    let path = windows_path.as_str();
    Path::new(path).is_absolute() && root_identity(path).is_ok_and(|root| root == identity)
}

pub(super) async fn import_root_records(
    conn: &DatabaseConnection,
    directory: &Path,
    identity: &str,
) -> Result<(), ApiError> {
    let imported = conn
        .query_one(Statement::from_string(
            DatabaseBackend::Sqlite,
            "SELECT name FROM installation_imports WHERE name = 'root_records'",
        ))
        .await?
        .is_some();
    if imported {
        return Ok(());
    }

    let mut previous_gateway = Vec::new();
    let mut previous_flatten = Vec::new();
    let legacy_path = directory.join("scenery.db");
    if legacy_path.is_file() {
        let legacy = connection::open_read_only_at(&legacy_path).await?;
        let schema = SchemaManager::new(&legacy);
        if schema.has_table("gateway_installs").await? {
            let key = crate::gateway::normalize_xplane_key(Path::new(identity));
            if owned_by_root(&key, identity) {
                previous_gateway = gateway::Entity::find()
                    .filter(gateway::Column::XplanePath.eq(key))
                    .all(&legacy)
                    .await?;
            }
        }
        if schema.has_table("airport_flatten_overrides").await? {
            previous_flatten = flatten::Entity::find()
                .order_by_desc(flatten::Column::UpdatedAt)
                .order_by_desc(flatten::Column::Id)
                .all(&legacy)
                .await?;
            let mut ownership = HashMap::new();
            previous_flatten.retain(|row| {
                *ownership
                    .entry(row.xplane_path.clone())
                    .or_insert_with(|| owned_by_root(&row.xplane_path, identity))
            });
        }
    }

    let txn = conn.begin().await?;
    let scoped_flatten = flatten::Entity::find()
        .order_by_desc(flatten::Column::UpdatedAt)
        .order_by_desc(flatten::Column::Id)
        .all(&txn)
        .await?;
    flatten::Entity::delete_many().exec(&txn).await?;
    for row in scoped_flatten.into_iter().chain(previous_flatten) {
        let source_path =
            crate::airport_flatten::canonical_source_path(Path::new(&row.source_path));
        let mut entry: flatten::ActiveModel = row.into();
        entry.id = ActiveValue::NotSet;
        entry.xplane_path = Set(identity.to_owned());
        entry.source_path = Set(source_path);
        flatten::Entity::insert(entry)
            .on_conflict(OnConflict::new().do_nothing().to_owned())
            .exec_without_returning(&txn)
            .await?;
    }

    for row in previous_gateway {
        let mut entry: gateway::ActiveModel = row.into();
        entry.id = ActiveValue::NotSet;
        gateway::Entity::insert(entry)
            .on_conflict(OnConflict::new().do_nothing().to_owned())
            .exec_without_returning(&txn)
            .await?;
    }
    mark_imported(&txn).await?;
    txn.commit().await?;
    Ok(())
}
