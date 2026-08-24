use anyhow::Result;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, Schema};

use crate::entities::{device_status, endpoint, notify_history, notify_target, project};

pub async fn connect() -> Result<DatabaseConnection> {
    let db = Database::connect("sqlite://statuswatch.db?mode=rwc").await?;
    Ok(db)
}

/// Creates any missing tables. Idempotent — safe to call on every startup instead of
/// maintaining versioned migrations.
pub async fn ensure_schema(db: &DatabaseConnection) -> Result<()> {
    let backend = db.get_database_backend();
    let schema = Schema::new(backend);

    macro_rules! create_table {
        ($entity:expr) => {{
            let mut stmt = schema.create_table_from_entity($entity);
            stmt.if_not_exists();
            db.execute(&stmt).await?;
        }};
    }

    create_table!(endpoint::Entity);
    create_table!(project::Entity);
    create_table!(device_status::Entity);
    create_table!(notify_target::Entity);
    create_table!(notify_history::Entity);

    Ok(())
}
