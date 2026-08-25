use std::collections::HashMap;

use axum::extract::State;
use maud::{Markup, PreEscaped, html};
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
            script {
                (PreEscaped(r#"
                    document.body.addEventListener("htmx:beforeSwap", function (event) {
                        const target = event.detail && event.detail.target;
                        if (!target || target.id !== "dashboard-table") return;
                        target.__openDetails = Array.from(target.querySelectorAll("details[id][open]"))
                            .map(function (details) { return details.id; });
                    });
                    document.body.addEventListener("htmx:afterSwap", function (event) {
                        const target = event.detail && event.detail.target;
                        if (!target || target.id !== "dashboard-table") return;
                        (target.__openDetails || []).forEach(function (id) {
                            const details = document.getElementById(id);
                            if (details) details.open = true;
                        });
                        delete target.__openDetails;
                    });
                "#))
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
            @let devices = devices_by_project.get(&p.id);
            @let total = devices.map(|devices| devices.len()).unwrap_or(0);
            @let unset = devices.map(|devices| count_status(devices, "unset")).unwrap_or(0);
            @let start = devices.map(|devices| count_status(devices, "start")).unwrap_or(0);
            @let online = devices.map(|devices| count_status(devices, "online")).unwrap_or(0);
            @let offline = devices.map(|devices| count_status(devices, "offline")).unwrap_or(0);
            @let stop = devices.map(|devices| count_status(devices, "stop")).unwrap_or(0);
            @let abnormal = devices.map(|devices| count_status(devices, "abnormal")).unwrap_or(0);
            details id=(format!("project-{}", p.id)) class="mb-6 bg-white rounded shadow" {
                summary class="cursor-pointer p-4" {
                    span class="inline-flex min-w-0 flex-nowrap items-center gap-3 align-middle" {
                        span class="shrink-0 whitespace-nowrap text-lg font-semibold" { (ep_name) " / " (p.name) }
                        span class="inline-flex min-w-0 flex-nowrap gap-2 overflow-x-auto whitespace-nowrap text-xs align-middle" {
                            span class="rounded-full bg-slate-50 text-slate-800 px-2 py-0.5 font-semibold" {
                                "Total " (total)
                            }
                            span class="rounded-full bg-green-100 text-green-800 px-2 py-0.5 font-semibold" {
                                "Online " (online)
                            }
                            span class="rounded-full bg-red-100 text-red-800 px-2 py-0.5 font-semibold" {
                                "Offline " (offline)
                            }
                            span class="rounded-full bg-red-100 text-red-800 px-2 py-0.5 font-semibold" {
                                "Abnormal " (abnormal)
                            }
                            span class="rounded-full bg-yellow-100 text-yellow-800 px-2 py-0.5 font-semibold" {
                                "Start " (start)
                            }
                            span class="rounded-full bg-yellow-100 text-yellow-800 px-2 py-0.5 font-semibold" {
                                "Stop " (stop)
                            }
                            span class="rounded-full bg-gray-100 text-gray-800 px-2 py-0.5 font-semibold" {
                                "Unset " (unset)
                            }
                        }
                    }
                }
                div class="pl-8 pr-4 pb-4" {
                    @match devices {
                        None => p class="text-slate-500 text-sm" { "No devices seen yet." },
                        Some(devices) => {
                            div class="overflow-x-auto" {
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
            }
        }
    })
}

fn count_status(devices: &[device_status::Model], status: &str) -> usize {
    devices
        .iter()
        .filter(|device| device.status.eq_ignore_ascii_case(status))
        .count()
}
