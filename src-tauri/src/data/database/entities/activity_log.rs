use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "activity_log")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub timestamp: i64,
    pub operation: String,
    pub item_type: String,
    pub item_name: String,
    pub details: Option<String>,
    pub success: bool,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        panic!("No relations defined")
    }
}

impl ActiveModelBehavior for ActiveModel {}
