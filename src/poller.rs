//! Background polling loop: fetch each configured project's device active-monitor status,
//! diff against the last known `device_status` row, and dispatch notifications on change.

use std::collections::HashMap;
use std::time::Duration;

use anyhow::Result;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, Set};

use crate::entities::{device_status, endpoint, project};
use crate::models::{Device, EdgeEndpoint, Endpoint, GeneralEndpoint};
use crate::notify;
use crate::state::AppState;
use crate::utils::ApiHelper;

const POLL_INTERVAL: Duration = Duration::from_secs(60);

pub async fn poll_loop(state: AppState) -> Result<()> {
    tracing::info!("poll loop starting, interval={POLL_INTERVAL:?}");
    loop {
        if let Err(e) = poll_once(&state).await {
            tracing::error!("poll cycle failed: {e:#}");
        }
        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

async fn poll_once(state: &AppState) -> Result<()> {
    let endpoints_by_id: HashMap<i32, endpoint::Model> = endpoint::Entity::find()
        .all(&state.db)
        .await?
        .into_iter()
        .map(|e| (e.id, e))
        .collect();

    let projects = project::Entity::find()
        .filter(project::Column::PollEnabled.eq(true))
        .all(&state.db)
        .await?;
    let project_count = projects.len();
    tracing::debug!("poll cycle starting, {project_count} project(s) enabled");

    let futs = projects.into_iter().filter_map(|p| {
        let endpoint_row = endpoints_by_id.get(&p.endpoint_id)?.clone();
        Some(poll_project(state, endpoint_row, p))
    });

    let mut failed = 0;
    for result in futures_util::future::join_all(futs).await {
        if let Err(e) = result {
            failed += 1;
            tracing::error!("project poll failed: {e:#}");
        }
    }
    tracing::debug!("poll cycle finished, {project_count} project(s), {failed} failed");

    Ok(())
}

fn endpoint_model_to_api(row: &endpoint::Model) -> Endpoint {
    match row.kind.as_str() {
        "edge" => Endpoint::Edge(EdgeEndpoint {
            base_url: row.base_url.clone(),
        }),
        _ => Endpoint::General(GeneralEndpoint {
            base_url: row.base_url.clone(),
        }),
    }
}

async fn poll_project(
    state: &AppState,
    endpoint_row: endpoint::Model,
    project_row: project::Model,
) -> Result<()> {
    let endpoint = endpoint_model_to_api(&endpoint_row);
    let devices =
        ApiHelper::req_project_meta(&state.client, &endpoint, &project_row.project_key).await?;

    for device in &devices {
        if let Err(e) = poll_device(state, &endpoint, &project_row, device).await {
            tracing::error!("device {} poll failed: {e:#}", device.id);
        }
    }

    Ok(())
}

async fn poll_device(
    state: &AppState,
    endpoint: &Endpoint,
    project_row: &project::Model,
    device: &Device,
) -> Result<()> {
    let Some(active_info) =
        ApiHelper::fetch_active_info(&state.client, endpoint, &device.id, &project_row.project_key)
            .await?
    else {
        return Ok(()); // no active monitor configured for this device
    };

    let new_status = active_info.status.to_string();
    let now = Utc::now();

    let existing = device_status::Entity::find()
        .filter(device_status::Column::ProjectId.eq(project_row.id))
        .filter(device_status::Column::DeviceId.eq(device.id.clone()))
        .one(&state.db)
        .await?;

    match diff_status(existing.as_ref().map(|e| e.status.as_str()), &new_status) {
        DiffResult::New => {
            device_status::ActiveModel {
                project_id: Set(project_row.id),
                device_id: Set(device.id.clone()),
                device_name: Set(device.name.clone()),
                status: Set(new_status),
                last_data_time: Set(active_info.last_data_time.clone()),
                last_checked_at: Set(now),
                last_changed_at: Set(now),
                ..Default::default()
            }
            .insert(&state.db)
            .await?;
        }
        DiffResult::Unchanged => {
            let mut am = existing.unwrap().into_active_model();
            am.last_checked_at = Set(now);
            am.last_data_time = Set(active_info.last_data_time.clone());
            am.update(&state.db).await?;
        }
        DiffResult::Changed { old_status } => {
            tracing::info!(
                "device {} ({}) status changed: {old_status} -> {new_status}",
                device.id,
                device.name,
            );

            let mut am = existing.unwrap().into_active_model();
            am.status = Set(new_status.clone());
            am.last_data_time = Set(active_info.last_data_time.clone());
            am.last_checked_at = Set(now);
            am.last_changed_at = Set(now);
            am.update(&state.db).await?;

            notify::dispatch_notifications(
                state,
                endpoint,
                project_row,
                &device.id,
                &device.name,
                Some(&old_status),
                &new_status,
            )
            .await?;
        }
    }

    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
enum DiffResult {
    /// First time this device has been seen — seed the row without notifying.
    New,
    Unchanged,
    Changed { old_status: String },
}

fn diff_status(existing: Option<&str>, new_status: &str) -> DiffResult {
    match existing {
        None => DiffResult::New,
        Some(old) if old == new_status => DiffResult::Unchanged,
        Some(old) => DiffResult::Changed {
            old_status: old.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_new_when_no_existing_row() {
        assert_eq!(diff_status(None, "online"), DiffResult::New);
    }

    #[test]
    fn diff_unchanged_when_status_matches() {
        assert_eq!(diff_status(Some("online"), "online"), DiffResult::Unchanged);
    }

    #[test]
    fn diff_changed_when_status_differs() {
        assert_eq!(
            diff_status(Some("online"), "abnormal"),
            DiffResult::Changed {
                old_status: "online".to_string()
            }
        );
    }
}
