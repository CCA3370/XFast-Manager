use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "airport_flatten_overrides")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub xplane_path: String,
    pub icao: String,
    pub source_path: String,
    pub source_kind: String,
    pub source_label: String,
    pub folder_name: Option<String>,
    pub airport_name: String,
    pub desired_flattened: bool,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        panic!("No relations defined")
    }
}

impl ActiveModelBehavior for ActiveModel {}
