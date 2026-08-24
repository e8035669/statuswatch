//! Rendering + sending of Discord webhook notifications, and recording the attempt in
//! `notify_history` regardless of source (remote platform config vs local `notify_target`).

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use reqwest::Client;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use std::str::FromStr;

use crate::entities::{
    notify_history, notify_target,
    project::{self, NotifySource},
};
use crate::models::Endpoint;
use crate::state::AppState;
use crate::utils::ApiHelper;

/// Remote `ActiveNotify` entries are meant for LINE, but this app's operator repurposes a
/// "LINE" slot's `to` field to hold a Discord webhook URL instead.
pub const DISCORD_WEBHOOK_PREFIX: &str = "https://discord.com/api/webhooks/";

const DEFAULT_TEMPLATE: &str = "Alert: Device {device_name} ({device_id}) status changed to {status}";

/// Substitutes `{device_id} {device_name} {status} {old_status} {time}` into `template`,
/// falling back to a default alert message when `template` is `None`/empty.
pub fn render_template(
    template: Option<&str>,
    device_id: &str,
    device_name: &str,
    old_status: Option<&str>,
    new_status: &str,
    time: &DateTime<Utc>,
) -> String {
    let template = template.filter(|t| !t.is_empty()).unwrap_or(DEFAULT_TEMPLATE);
    template
        .replace("{device_id}", device_id)
        .replace("{device_name}", device_name)
        .replace("{status}", new_status)
        .replace("{old_status}", old_status.unwrap_or("unset"))
        .replace("{time}", &time.to_rfc3339())
}

pub async fn send_discord_webhook(client: &Client, webhook_url: &str, message: &str) -> Result<()> {
    let payload = serde_json::json!({ "content": message });
    let response = client.post(webhook_url).json(&payload).send().await?;

    if response.status().is_success() {
        Ok(())
    } else {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        Err(anyhow!("Discord webhook failed ({status}): {body}"))
    }
}

/// Sends to every applicable target for `project` (remote `ActiveNotify` config or local
/// `notify_target` rows, per `project.notify_source`) and records each attempt in
/// `notify_history`.
pub async fn dispatch_notifications(
    state: &AppState,
    endpoint: &Endpoint,
    project: &project::Model,
    device_id: &str,
    device_name: &str,
    old_status: Option<&str>,
    new_status: &str,
) -> Result<()> {
    let now = Utc::now();
    let source = NotifySource::from_str(&project.notify_source).unwrap_or_default();

    match source {
        NotifySource::Remote => {
            let notifies = ApiHelper::fetch_active_notifies(
                &state.client,
                endpoint,
                device_id,
                &project.project_key,
            )
            .await?;
            for notify in notifies.iter().filter(|n| {
                n.enable && n.kind == "LINE" && n.setting.to.starts_with(DISCORD_WEBHOOK_PREFIX)
            }) {
                let message = render_template(
                    notify.setting.message.as_deref(),
                    device_id,
                    device_name,
                    old_status,
                    new_status,
                    &now,
                );
                record_attempt(
                    state,
                    project.id,
                    device_id,
                    device_name,
                    old_status,
                    new_status,
                    "remote",
                    &notify.setting.to,
                    &message,
                    &notify.setting.to,
                    now,
                )
                .await?;
            }
        }
        NotifySource::Local => {
            let targets = notify_target::Entity::find()
                .filter(notify_target::Column::ProjectId.eq(project.id))
                .filter(notify_target::Column::Enabled.eq(true))
                .all(&state.db)
                .await?;
            for target in targets {
                let message = render_template(
                    target.message_template.as_deref(),
                    device_id,
                    device_name,
                    old_status,
                    new_status,
                    &now,
                );
                record_attempt(
                    state,
                    project.id,
                    device_id,
                    device_name,
                    old_status,
                    new_status,
                    "local",
                    &target.webhook_url,
                    &message,
                    &target.name,
                    now,
                )
                .await?;
            }
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn record_attempt(
    state: &AppState,
    project_id: i32,
    device_id: &str,
    device_name: &str,
    old_status: Option<&str>,
    new_status: &str,
    source: &str,
    webhook_url: &str,
    message: &str,
    target_label: &str,
    sent_at: DateTime<Utc>,
) -> Result<()> {
    let (success, error) = match send_discord_webhook(&state.client, webhook_url, message).await {
        Ok(()) => {
            tracing::info!("notification sent to {target_label} ({source}) for device {device_id}");
            (true, None)
        }
        Err(e) => {
            tracing::warn!("notification to {target_label} ({source}) for device {device_id} failed: {e:#}");
            (false, Some(e.to_string()))
        }
    };

    notify_history::ActiveModel {
        project_id: Set(project_id),
        device_id: Set(device_id.to_string()),
        device_name: Set(device_name.to_string()),
        old_status: Set(old_status.map(|s| s.to_string())),
        new_status: Set(new_status.to_string()),
        source: Set(source.to_string()),
        target_label: Set(target_label.to_string()),
        message: Set(message.to_string()),
        success: Set(success),
        error: Set(error),
        sent_at: Set(sent_at),
        ..Default::default()
    }
    .insert(&state.db)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_template_substitutes_all_variables() {
        let time = DateTime::parse_from_rfc3339("2026-08-24T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let out = render_template(
            Some("{device_name}/{device_id}: {old_status} -> {status} @ {time}"),
            "dev1",
            "Sensor A",
            Some("online"),
            "offline",
            &time,
        );
        assert_eq!(out, "Sensor A/dev1: online -> offline @ 2026-08-24T00:00:00+00:00");
    }

    #[test]
    fn render_template_falls_back_to_default_when_empty() {
        let time = Utc::now();
        let out = render_template(Some(""), "dev1", "Sensor A", None, "abnormal", &time);
        assert_eq!(out, "Alert: Device Sensor A (dev1) status changed to abnormal");
    }

    #[test]
    fn render_template_falls_back_to_default_when_none() {
        let time = Utc::now();
        let out = render_template(None, "dev1", "Sensor A", None, "abnormal", &time);
        assert_eq!(out, "Alert: Device Sensor A (dev1) status changed to abnormal");
    }
}
