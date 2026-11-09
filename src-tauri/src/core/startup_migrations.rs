use serde_json::Value;
use std::path::Path;
use tauri::Runtime;
use tauri_plugin_store::{Store, StoreExt};

const SETTINGS_STORE_PATH: &str = "settings.json";
const SETTINGS_MIGRATION_VERSION_KEY: &str = "settingsMigrationVersion";
const CURRENT_SETTINGS_MIGRATION_VERSION: u64 = 1;
const RETIRED_FLIGHT_FILE: &str = "xfast-manager-flight.json";
const RETIRED_MAP_SETTINGS: [&str; 8] = [
    "mapStyleUrl",
    "mapNavRadiusNm",
    "mapVatsimRefreshInterval",
    "mapLayerVisibility",
    "mapAirportFilters",
    "mapSimbriefPilotId",
    "mapFollowPlane",
    "mapWeightUnit",
];

#[derive(Debug, Default, PartialEq, Eq)]
pub struct MigrationOutcome {
    pub applied: bool,
    pub settings_removed: usize,
    pub temp_file_removed: bool,
}

trait MigrationStore {
    fn get_value(&self, key: &str) -> Option<Value>;
    fn delete_value(&self, key: &str) -> bool;
    fn set_value(&self, key: &str, value: Value);
    fn save_values(&self) -> Result<(), String>;
}

impl<R: Runtime> MigrationStore for Store<R> {
    fn get_value(&self, key: &str) -> Option<Value> {
        Store::get(self, key)
    }

    fn delete_value(&self, key: &str) -> bool {
        Store::delete(self, key)
    }

    fn set_value(&self, key: &str, value: Value) {
        Store::set(self, key, value);
    }

    fn save_values(&self) -> Result<(), String> {
        Store::save(self).map_err(|error| error.to_string())
    }
}

pub fn run<R: Runtime>(app: &tauri::App<R>) -> Result<MigrationOutcome, String> {
    let store = app
        .store(SETTINGS_STORE_PATH)
        .map_err(|error| format!("Failed to open settings store: {error}"))?;
    apply(store.as_ref(), &std::env::temp_dir())
}

fn apply(store: &impl MigrationStore, temp_dir: &Path) -> Result<MigrationOutcome, String> {
    let current_version = store
        .get_value(SETTINGS_MIGRATION_VERSION_KEY)
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    if current_version >= CURRENT_SETTINGS_MIGRATION_VERSION {
        return Ok(MigrationOutcome::default());
    }

    let retired_flight_file = temp_dir.join(RETIRED_FLIGHT_FILE);
    let temp_file_removed = match std::fs::remove_file(&retired_flight_file) {
        Ok(()) => true,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(error) => {
            return Err(format!(
                "Failed to remove retired flight file {}: {error}",
                retired_flight_file.display()
            ));
        }
    };

    let settings_removed = RETIRED_MAP_SETTINGS
        .iter()
        .filter(|key| store.delete_value(key))
        .count();
    store.set_value(
        SETTINGS_MIGRATION_VERSION_KEY,
        Value::from(CURRENT_SETTINGS_MIGRATION_VERSION),
    );

    if let Err(error) = store.save_values() {
        store.delete_value(SETTINGS_MIGRATION_VERSION_KEY);
        return Err(format!("Failed to save migrated settings: {error}"));
    }

    Ok(MigrationOutcome {
        applied: true,
        settings_removed,
        temp_file_removed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::collections::HashMap;

    #[derive(Default)]
    struct FakeStore {
        values: RefCell<HashMap<String, Value>>,
        save_calls: Cell<usize>,
        fail_save: Cell<bool>,
    }

    impl FakeStore {
        fn insert(&self, key: &str, value: Value) {
            self.values.borrow_mut().insert(key.to_string(), value);
        }
    }

    impl MigrationStore for FakeStore {
        fn get_value(&self, key: &str) -> Option<Value> {
            self.values.borrow().get(key).cloned()
        }

        fn delete_value(&self, key: &str) -> bool {
            self.values.borrow_mut().remove(key).is_some()
        }

        fn set_value(&self, key: &str, value: Value) {
            self.values.borrow_mut().insert(key.to_string(), value);
        }

        fn save_values(&self) -> Result<(), String> {
            self.save_calls.set(self.save_calls.get() + 1);
            if self.fail_save.get() {
                Err("save failed".to_string())
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn removes_retired_map_data_and_preserves_unrelated_settings() {
        let store = FakeStore::default();
        for key in RETIRED_MAP_SETTINGS {
            store.insert(key, Value::String("legacy".to_string()));
        }
        store.insert("theme", Value::String("dark".to_string()));

        let temp_dir = tempfile::tempdir().expect("create temp directory");
        let retired_file = temp_dir.path().join(RETIRED_FLIGHT_FILE);
        std::fs::write(&retired_file, "legacy flight").expect("write retired file");

        let outcome = apply(&store, temp_dir.path()).expect("migration succeeds");

        assert_eq!(
            outcome,
            MigrationOutcome {
                applied: true,
                settings_removed: RETIRED_MAP_SETTINGS.len(),
                temp_file_removed: true,
            }
        );
        assert!(!retired_file.exists());
        for key in RETIRED_MAP_SETTINGS {
            assert_eq!(store.get_value(key), None);
        }
        assert_eq!(
            store.get_value("theme"),
            Some(Value::String("dark".to_string()))
        );
        assert_eq!(
            store.get_value(SETTINGS_MIGRATION_VERSION_KEY),
            Some(Value::from(CURRENT_SETTINGS_MIGRATION_VERSION))
        );
        assert_eq!(store.save_calls.get(), 1);
    }

    #[test]
    fn clean_install_is_marked_once_and_subsequent_runs_are_noops() {
        let store = FakeStore::default();
        let temp_dir = tempfile::tempdir().expect("create temp directory");

        let first = apply(&store, temp_dir.path()).expect("first migration succeeds");
        let second = apply(&store, temp_dir.path()).expect("second migration succeeds");

        assert_eq!(
            first,
            MigrationOutcome {
                applied: true,
                settings_removed: 0,
                temp_file_removed: false,
            }
        );
        assert_eq!(second, MigrationOutcome::default());
        assert_eq!(store.save_calls.get(), 1);
    }

    #[test]
    fn temp_cleanup_failure_keeps_settings_unmigrated_for_retry() {
        let store = FakeStore::default();
        store.insert(RETIRED_MAP_SETTINGS[0], Value::Bool(true));
        let temp_dir = tempfile::tempdir().expect("create temp directory");
        std::fs::create_dir(temp_dir.path().join(RETIRED_FLIGHT_FILE))
            .expect("create conflicting directory");

        let error = apply(&store, temp_dir.path()).expect_err("migration must fail");

        assert!(error.contains("Failed to remove retired flight file"));
        assert_eq!(
            store.get_value(RETIRED_MAP_SETTINGS[0]),
            Some(Value::Bool(true))
        );
        assert_eq!(store.get_value(SETTINGS_MIGRATION_VERSION_KEY), None);
        assert_eq!(store.save_calls.get(), 0);
    }

    #[test]
    fn save_failure_does_not_leave_a_completed_version_marker() {
        let store = FakeStore::default();
        store.insert(RETIRED_MAP_SETTINGS[0], Value::Bool(true));
        store.fail_save.set(true);
        let temp_dir = tempfile::tempdir().expect("create temp directory");

        let error = apply(&store, temp_dir.path()).expect_err("migration must fail");

        assert_eq!(error, "Failed to save migrated settings: save failed");
        assert_eq!(store.get_value(SETTINGS_MIGRATION_VERSION_KEY), None);
        assert_eq!(store.save_calls.get(), 1);
    }
}
