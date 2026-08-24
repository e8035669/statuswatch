use sea_orm::entity::prelude::*;

/// A record of one attempted Discord notification, for the history page.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "notify_history")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub project_id: i32,
    pub device_id: String,
    pub device_name: String,
    pub old_status: Option<String>,
    pub new_status: String,
    /// "remote" or "local", see [`super::project::NotifySource`].
    pub source: String,
    /// Webhook URL (remote) or notify target name (local), kept even if the source is deleted.
    pub target_label: String,
    pub message: String,
    pub success: bool,
    pub error: Option<String>,
    pub sent_at: DateTimeUtc,
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
