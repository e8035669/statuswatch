use sea_orm::entity::prelude::*;

/// A monitored project (project_key) under an [`super::endpoint::Model`].
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "project")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub endpoint_id: i32,
    pub project_key: String,
    pub name: String,
    /// "remote" (use platform's own ActiveNotify config) or "local" (use `notify_target` rows).
    pub notify_source: String,
    pub poll_enabled: bool,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::endpoint::Entity",
        from = "Column::EndpointId",
        to = "super::endpoint::Column::Id"
    )]
    Endpoint,
    #[sea_orm(has_many = "super::device_status::Entity")]
    DeviceStatus,
    #[sea_orm(has_many = "super::notify_target::Entity")]
    NotifyTarget,
    #[sea_orm(has_many = "super::notify_history::Entity")]
    NotifyHistory,
}

impl Related<super::endpoint::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Endpoint.def()
    }
}

impl Related<super::device_status::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::DeviceStatus.def()
    }
}

impl Related<super::notify_target::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::NotifyTarget.def()
    }
}

impl Related<super::notify_history::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::NotifyHistory.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum NotifySource {
    #[default]
    Remote,
    Local,
}

impl NotifySource {
    pub fn as_str(self) -> &'static str {
        match self {
            NotifySource::Remote => "remote",
            NotifySource::Local => "local",
        }
    }
}

impl std::str::FromStr for NotifySource {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "remote" => Ok(NotifySource::Remote),
            "local" => Ok(NotifySource::Local),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for NotifySource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
