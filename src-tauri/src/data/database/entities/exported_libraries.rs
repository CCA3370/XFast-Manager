use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "exported_libraries")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub package_id: i64,
    pub library_name: String,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    SceneryPackage,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::SceneryPackage => Entity::belongs_to(super::scenery_packages::Entity)
                .from(Column::PackageId)
                .to(super::scenery_packages::Column::Id)
                .into(),
        }
    }
}

impl Related<super::scenery_packages::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SceneryPackage.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
