use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "gateway_installs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub xplane_path: String,
    pub airport_icao: String,
    pub airport_name: String,
    pub scenery_id: i64,
    pub folder_name: String,
    pub artist: Option<String>,
    pub approved_date: Option<String>,
    pub installed_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        panic!("No relations defined")
    }
}

impl ActiveModelBehavior for ActiveModel {}
