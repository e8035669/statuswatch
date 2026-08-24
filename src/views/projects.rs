use axum::extract::{Path, State};
use axum::Form;
use maud::{html, Markup};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, Set, TransactionTrait,
};
use serde::Deserialize;

use crate::entities::{device_status, endpoint, notify_history, notify_target, project};
use crate::error::AppError;
use crate::state::AppState;
use crate::views::layout::page;

#[derive(Deserialize)]
pub struct ProjectForm {
    pub name: String,
    pub project_key: String,
    #[serde(default)]
    pub notify_source: String,
    #[serde(default)]
    pub poll_enabled: Option<String>,
}

impl ProjectForm {
    fn poll_enabled(&self) -> bool {
        self.poll_enabled.as_deref() == Some("on")
    }

    fn notify_source(&self) -> String {
        if self.notify_source == "local" {
            "local".to_string()
        } else {
            "remote".to_string()
        }
    }
}

pub async fn list_page(
    State(state): State<AppState>,
    Path(endpoint_id): Path<i32>,
) -> Result<Markup, AppError> {
    let Some(ep) = endpoint::Entity::find_by_id(endpoint_id).one(&state.db).await? else {
        return Ok(page(
            "Projects",
            html! { p class="text-red-600" { "Endpoint not found" } },
        ));
    };
    let list = render_list(&state, endpoint_id).await?;
    Ok(page(
        "Projects",
        html! {
            h1 class="text-2xl font-bold mb-1" { (ep.name) }
            p class="text-slate-500 mb-4" { (ep.base_url) }
            (render_form(endpoint_id))
            div id="project-list" class="mt-6" { (list) }
        },
    ))
}

pub async fn create(
    State(state): State<AppState>,
    Path(endpoint_id): Path<i32>,
    Form(form): Form<ProjectForm>,
) -> Result<Markup, AppError> {
    let notify_source = form.notify_source();
    let poll_enabled = form.poll_enabled();
    project::ActiveModel {
        endpoint_id: Set(endpoint_id),
        project_key: Set(form.project_key),
        name: Set(form.name),
        notify_source: Set(notify_source),
        poll_enabled: Set(poll_enabled),
        created_at: Set(chrono::Utc::now()),
        ..Default::default()
    }
    .insert(&state.db)
    .await?;

    render_list(&state, endpoint_id).await
}

pub async fn edit_form(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Markup, AppError> {
    let Some(row) = project::Entity::find_by_id(id).one(&state.db).await? else {
        return Ok(html! { p class="text-red-600" { "Project not found" } });
    };
    Ok(render_row_edit(&row))
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Form(form): Form<ProjectForm>,
) -> Result<Markup, AppError> {
    let Some(row) = project::Entity::find_by_id(id).one(&state.db).await? else {
        return Ok(html! { p class="text-red-600" { "Project not found" } });
    };
    let endpoint_id = row.endpoint_id;
    let notify_source = form.notify_source();
    let poll_enabled = form.poll_enabled();
    let mut am = row.into_active_model();
    am.name = Set(form.name);
    am.project_key = Set(form.project_key);
    am.notify_source = Set(notify_source);
    am.poll_enabled = Set(poll_enabled);
    am.update(&state.db).await?;

    render_list(&state, endpoint_id).await
}

/// Deletes the project together with its device status, notify targets and history.
pub async fn delete_project(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Markup, AppError> {
    let Some(row) = project::Entity::find_by_id(id).one(&state.db).await? else {
        return Ok(html! { p { "Project not found" } });
    };
    let endpoint_id = row.endpoint_id;

    let txn = state.db.begin().await?;
    device_status::Entity::delete_many()
        .filter(device_status::Column::ProjectId.eq(id))
        .exec(&txn)
        .await?;
    notify_target::Entity::delete_many()
        .filter(notify_target::Column::ProjectId.eq(id))
        .exec(&txn)
        .await?;
    notify_history::Entity::delete_many()
        .filter(notify_history::Column::ProjectId.eq(id))
        .exec(&txn)
        .await?;
    project::Entity::delete_by_id(id).exec(&txn).await?;
    txn.commit().await?;

    render_list(&state, endpoint_id).await
}

async fn render_list(state: &AppState, endpoint_id: i32) -> Result<Markup, AppError> {
    let rows = project::Entity::find()
        .filter(project::Column::EndpointId.eq(endpoint_id))
        .all(&state.db)
        .await?;

    Ok(html! {
        div id="project-list" {
            @if rows.is_empty() {
                p class="text-slate-500" { "No projects yet." }
            } @else {
                table class="w-full text-sm bg-white rounded shadow" {
                    thead {
                        tr class="text-left text-slate-500 border-b" {
                            th class="py-2 px-3" { "Name" }
                            th class="py-2 px-3" { "Project Key" }
                            th class="py-2 px-3" { "Notify Source" }
                            th class="py-2 px-3" { "Polling" }
                            th class="py-2 px-3" { "" }
                        }
                    }
                    tbody {
                        @for row in &rows {
                            @let edit_url = format!("/settings/projects/{}/edit", row.id);
                            @let del_url = format!("/settings/projects/{}", row.id);
                            @let targets_url = format!("/settings/projects/{}/notify-targets", row.id);
                            tr class="border-b last:border-0" {
                                td class="py-2 px-3" { (row.name) }
                                td class="py-2 px-3 text-slate-500" { (row.project_key) }
                                td class="py-2 px-3" { (row.notify_source) }
                                td class="py-2 px-3" { @if row.poll_enabled { "on" } @else { "off" } }
                                td class="py-2 px-3 flex gap-3" {
                                    a class="text-blue-600 underline" href=(targets_url) { "Notify Targets" }
                                    button class="text-blue-600 underline"
                                        hx-get=(edit_url) hx-target="closest tr" hx-swap="outerHTML" { "Edit" }
                                    button class="text-red-600 underline"
                                        hx-delete=(del_url) hx-target="#project-list" hx-swap="outerHTML"
                                        hx-confirm="Delete this project and its history?" { "Delete" }
                                }
                            }
                        }
                    }
                }
            }
        }
    })
}

fn render_row_edit(row: &project::Model) -> Markup {
    let update_url = format!("/settings/projects/{}", row.id);
    html! {
        tr {
            td class="py-2 px-3" colspan="5" {
                form class="flex gap-2 items-center flex-wrap"
                    hx-put=(update_url) hx-target="#project-list" hx-swap="outerHTML" {
                    input class="border rounded px-2 py-1" type="text" name="name" value=(row.name) required;
                    input class="border rounded px-2 py-1" type="text" name="project_key" value=(row.project_key) required;
                    select class="border rounded px-2 py-1" name="notify_source" {
                        option value="remote" selected[row.notify_source == "remote"] { "Remote" }
                        option value="local" selected[row.notify_source == "local"] { "Local" }
                    }
                    label class="flex items-center gap-1 text-sm" {
                        input type="checkbox" name="poll_enabled" checked[row.poll_enabled];
                        "Polling enabled"
                    }
                    button class="text-blue-600 underline" type="submit" { "Save" }
                }
            }
        }
    }
}

fn render_form(endpoint_id: i32) -> Markup {
    let create_url = format!("/settings/endpoints/{endpoint_id}/projects");
    html! {
        form class="flex gap-2 items-center flex-wrap bg-white rounded shadow p-4"
            hx-post=(create_url) hx-target="#project-list" hx-swap="outerHTML" {
            input class="border rounded px-2 py-1" type="text" name="name" placeholder="Label" required;
            input class="border rounded px-2 py-1" type="text" name="project_key" placeholder="Project key" required;
            select class="border rounded px-2 py-1" name="notify_source" {
                option value="remote" { "Remote" }
                option value="local" { "Local" }
            }
            label class="flex items-center gap-1 text-sm" {
                input type="checkbox" name="poll_enabled" checked;
                "Polling enabled"
            }
            button class="bg-blue-600 text-white rounded px-3 py-1" type="submit" { "Add" }
        }
    }
}
