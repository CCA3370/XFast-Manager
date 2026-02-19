use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "scenery_packages")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub folder_name: String,
    pub category: String,
    pub sub_priority: i32,
    pub last_modified: i64,
    pub indexed_at: i64,
    pub has_apt_dat: bool,
    pub airport_id: Option<String>,
    pub has_dsf: bool,
    pub has_library_txt: bool,
    pub has_textures: bool,
    pub has_objects: bool,
    pub texture_count: i32,
    pub earth_nav_tile_count: i32,
    pub enabled: bool,
    pub sort_order: i32,
    pub actual_path: Option<String>,
    pub continent: Option<String>,
    pub original_category: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    RequiredLibraries,
    MissingLibraries,
    ExportedLibraries,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::RequiredLibraries => Entity::has_many(super::required_libraries::Entity).into(),
            Self::MissingLibraries => Entity::has_many(super::missing_libraries::Entity).into(),
            Self::ExportedLibraries => Entity::has_many(super::exported_libraries::Entity).into(),
        }
    }
}

impl Related<super::required_libraries::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::RequiredLibraries.def()
    }
}

impl Related<super::missing_libraries::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::MissingLibraries.def()
    }
}

impl Related<super::exported_libraries::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ExportedLibraries.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
