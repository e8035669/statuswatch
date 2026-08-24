use axum::extract::{Query, State};
use maud::{html, Markup};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use serde::Deserialize;

use crate::components::badge::status_badge;
use crate::entities::{notify_history, project};
use crate::error::AppError;
use crate::state::AppState;
use crate::views::layout::page;

#[derive(Deserialize)]
pub struct HistoryQuery {
    #[serde(default)]
    pub project_id: Option<String>,
}

pub async fn history_page(
    State(state): State<AppState>,
    Query(query): Query<HistoryQuery>,
) -> Result<Markup, AppError> {
    let selected_project_id: Option<i32> = query
        .project_id
        .as_deref()
        .filter(|s| !s.is_empty())
        .and_then(|s| s.parse().ok());

    let projects = project::Entity::find().all(&state.db).await?;

    let mut q = notify_history::Entity::find().order_by_desc(notify_history::Column::SentAt);
    if let Some(pid) = selected_project_id {
        q = q.filter(notify_history::Column::ProjectId.eq(pid));
    }
    let rows = q.limit(200).all(&state.db).await?;

    Ok(page(
        "History",
        html! {
            h1 class="text-2xl font-bold mb-4" { "Notification History" }
            form class="mb-4" method="get" {
                select class="border rounded px-2 py-1" name="project_id" onchange="this.form.submit()" {
                    option value="" selected[selected_project_id.is_none()] { "All projects" }
                    @for p in &projects {
                        option value=(p.id) selected[selected_project_id == Some(p.id)] { (p.name) }
                    }
                }
            }
            @if rows.is_empty() {
                p class="text-slate-500" { "No notifications sent yet." }
            } @else {
                table class="w-full text-sm bg-white rounded shadow" {
                    thead {
                        tr class="text-left text-slate-500 border-b" {
                            th class="py-2 px-3" { "Sent" }
                            th class="py-2 px-3" { "Device" }
                            th class="py-2 px-3" { "Change" }
                            th class="py-2 px-3" { "Source" }
                            th class="py-2 px-3" { "Target" }
                            th class="py-2 px-3" { "Result" }
                        }
                    }
                    tbody {
                        @for row in &rows {
                            @let old_status_display = row.old_status.clone().unwrap_or_else(|| "-".to_string());
                            tr class="border-b last:border-0" {
                                td class="py-2 px-3 text-slate-500" { (row.sent_at.to_rfc3339()) }
                                td class="py-2 px-3" { (row.device_name) " (" (row.device_id) ")" }
                                td class="py-2 px-3" {
                                    (old_status_display) " → " (status_badge(&row.new_status))
                                }
                                td class="py-2 px-3" { (row.source) }
                                td class="py-2 px-3 break-all text-slate-500" { (row.target_label) }
                                td class="py-2 px-3" {
                                    @if row.success {
                                        span class="text-green-700" { "sent" }
                                    } @else {
                                        @let err = row.error.clone().unwrap_or_default();
                                        span class="text-red-700" title=(err) { "failed" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        },
    ))
}
