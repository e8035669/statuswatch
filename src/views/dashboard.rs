use std::collections::HashMap;

use axum::extract::State;
use maud::{html, Markup};
use sea_orm::EntityTrait;

use crate::components::badge::status_badge;
use crate::entities::{device_status, endpoint, project};
use crate::error::AppError;
use crate::state::AppState;
use crate::views::layout::page;

pub async fn dashboard(State(state): State<AppState>) -> Result<Markup, AppError> {
    let table = render_status_table(&state).await?;
    Ok(page(
        "Dashboard",
        html! {
            h1 class="text-2xl font-bold mb-4" { "Device Status" }
            div id="dashboard-table"
                hx-get="/partials/dashboard"
                hx-trigger="every 10s"
                hx-swap="innerHTML" {
                (table)
            }
        },
    ))
}

pub async fn dashboard_partial(State(state): State<AppState>) -> Result<Markup, AppError> {
    render_status_table(&state).await
}

async fn render_status_table(state: &AppState) -> Result<Markup, AppError> {
    let endpoints: HashMap<i32, endpoint::Model> = endpoint::Entity::find()
        .all(&state.db)
        .await?
        .into_iter()
        .map(|e| (e.id, e))
        .collect();

    let projects = project::Entity::find().all(&state.db).await?;

    let mut devices_by_project: HashMap<i32, Vec<device_status::Model>> = HashMap::new();
    for d in device_status::Entity::find().all(&state.db).await? {
        devices_by_project.entry(d.project_id).or_default().push(d);
    }
    for devices in devices_by_project.values_mut() {
        devices.sort_by(|a, b| a.device_name.cmp(&b.device_name));
    }

    if projects.is_empty() {
        return Ok(html! {
            p class="text-slate-500" {
                "No projects configured yet. Go to "
                a class="text-blue-600 underline" href="/settings/endpoints" { "Endpoints" }
                " to add one."
            }
        });
    }

    Ok(html! {
        @for p in &projects {
            @let ep_name = endpoints.get(&p.endpoint_id).map(|e| e.name.as_str()).unwrap_or("(unknown endpoint)");
            div class="mb-6 bg-white rounded shadow p-4" {
                h2 class="text-lg font-semibold mb-2" { (ep_name) " / " (p.name) }
                @match devices_by_project.get(&p.id) {
                    None => p class="text-slate-500 text-sm" { "No devices seen yet." },
                    Some(devices) => {
                        table class="w-full text-sm" {
                            thead {
                                tr class="text-left text-slate-500 border-b" {
                                    th class="py-1 pr-4" { "Device" }
                                    th class="py-1 pr-4" { "Status" }
                                    th class="py-1 pr-4" { "Last Data" }
                                    th class="py-1 pr-4" { "Checked" }
                                    th class="py-1" { "Changed" }
                                }
                            }
                            tbody {
                                @for d in devices {
                                    tr class="border-b last:border-0" {
                                        td class="py-1 pr-4" { (d.device_name) " (" (d.device_id) ")" }
                                        td class="py-1 pr-4" { (status_badge(&d.status)) }
                                        td class="py-1 pr-4 text-slate-500" {
                                            (d.last_data_time.clone().unwrap_or_default())
                                        }
                                        td class="py-1 pr-4 text-slate-500" { (d.last_checked_at.to_rfc3339()) }
                                        td class="text-slate-500" { (d.last_changed_at.to_rfc3339()) }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    })
}
