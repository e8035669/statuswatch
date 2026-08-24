use sea_orm::entity::prelude::*;

/// Last known status of one device within a project, used to detect changes between polls.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "device_status")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub project_id: i32,
    pub device_id: String,
    pub device_name: String,
    /// Mirrors [`crate::models::ActiveStatus`] as text.
    pub status: String,
    pub last_data_time: Option<String>,
    pub last_checked_at: DateTimeUtc,
    pub last_changed_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::project::Entity",
        from = "Column::ProjectId",
        to = "super::project::Column::Id"
    )]
    Project,
}

impl Related<super::project::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Project.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
