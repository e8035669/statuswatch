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
pub struct EndpointForm {
    pub name: String,
    pub base_url: String,
    pub kind: String,
}

pub async fn list_page(State(state): State<AppState>) -> Result<Markup, AppError> {
    let list = render_list(&state).await?;
    Ok(page(
        "Endpoints",
        html! {
            h1 class="text-2xl font-bold mb-4" { "Endpoints" }
            (render_form())
            div id="endpoint-list" class="mt-6" { (list) }
        },
    ))
}

pub async fn create(
    State(state): State<AppState>,
    Form(form): Form<EndpointForm>,
) -> Result<Markup, AppError> {
    endpoint::ActiveModel {
        name: Set(form.name),
        base_url: Set(form.base_url),
        kind: Set(form.kind),
        created_at: Set(chrono::Utc::now()),
        ..Default::default()
    }
    .insert(&state.db)
    .await?;

    render_list(&state).await
}

pub async fn edit_form(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Markup, AppError> {
    let Some(row) = endpoint::Entity::find_by_id(id).one(&state.db).await? else {
        return Ok(html! { p class="text-red-600" { "Endpoint not found" } });
    };
    Ok(render_row_edit(&row))
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Form(form): Form<EndpointForm>,
) -> Result<Markup, AppError> {
    if let Some(row) = endpoint::Entity::find_by_id(id).one(&state.db).await? {
        let mut am = row.into_active_model();
        am.name = Set(form.name);
        am.base_url = Set(form.base_url);
        am.kind = Set(form.kind);
        am.update(&state.db).await?;
    }
    render_list(&state).await
}

/// Deletes the endpoint together with all its projects and their dependent rows.
pub async fn delete_endpoint(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Markup, AppError> {
    let txn = state.db.begin().await?;

    let project_ids: Vec<i32> = project::Entity::find()
        .filter(project::Column::EndpointId.eq(id))
        .all(&txn)
        .await?
        .into_iter()
        .map(|p| p.id)
        .collect();

    for pid in &project_ids {
        device_status::Entity::delete_many()
            .filter(device_status::Column::ProjectId.eq(*pid))
            .exec(&txn)
            .await?;
        notify_target::Entity::delete_many()
            .filter(notify_target::Column::ProjectId.eq(*pid))
            .exec(&txn)
            .await?;
        notify_history::Entity::delete_many()
            .filter(notify_history::Column::ProjectId.eq(*pid))
            .exec(&txn)
            .await?;
    }
    project::Entity::delete_many()
        .filter(project::Column::EndpointId.eq(id))
        .exec(&txn)
        .await?;
    endpoint::Entity::delete_by_id(id).exec(&txn).await?;
    txn.commit().await?;

    render_list(&state).await
}

async fn render_list(state: &AppState) -> Result<Markup, AppError> {
    let rows = endpoint::Entity::find().all(&state.db).await?;
    Ok(html! {
        div id="endpoint-list" {
            @if rows.is_empty() {
                p class="text-slate-500" { "No endpoints yet." }
            } @else {
                table class="w-full text-sm bg-white rounded shadow" {
                    thead {
                        tr class="text-left text-slate-500 border-b" {
                            th class="py-2 px-3" { "Name" }
                            th class="py-2 px-3" { "Base URL" }
                            th class="py-2 px-3" { "Kind" }
                            th class="py-2 px-3" { "" }
                        }
                    }
                    tbody {
                        @for row in &rows {
                            @let edit_url = format!("/settings/endpoints/{}/edit", row.id);
                            @let del_url = format!("/settings/endpoints/{}", row.id);
                            @let projects_url = format!("/settings/endpoints/{}/projects", row.id);
                            tr class="border-b last:border-0" {
                                td class="py-2 px-3" { (row.name) }
                                td class="py-2 px-3 text-slate-500 break-all" { (row.base_url) }
                                td class="py-2 px-3" { (row.kind) }
                                td class="py-2 px-3 flex gap-3" {
                                    a class="text-blue-600 underline" href=(projects_url) { "Projects" }
                                    button class="text-blue-600 underline"
                                        hx-get=(edit_url) hx-target="closest tr" hx-swap="outerHTML" { "Edit" }
                                    button class="text-red-600 underline"
                                        hx-delete=(del_url) hx-target="#endpoint-list" hx-swap="outerHTML"
                                        hx-confirm="Delete this endpoint and all its projects?" { "Delete" }
                                }
                            }
                        }
                    }
                }
            }
        }
    })
}

fn render_row_edit(row: &endpoint::Model) -> Markup {
    let update_url = format!("/settings/endpoints/{}", row.id);
    html! {
        tr {
            td class="py-2 px-3" colspan="4" {
                form class="flex gap-2 items-center flex-wrap"
                    hx-put=(update_url) hx-target="#endpoint-list" hx-swap="outerHTML" {
                    input class="border rounded px-2 py-1" type="text" name="name" value=(row.name) required;
                    input class="border rounded px-2 py-1 flex-1" type="text" name="base_url" value=(row.base_url) required;
                    select class="border rounded px-2 py-1" name="kind" {
                        option value="general" selected[row.kind == "general"] { "General" }
                        option value="edge" selected[row.kind == "edge"] { "Edge" }
                    }
                    button class="text-blue-600 underline" type="submit" { "Save" }
                }
            }
        }
    }
}

fn render_form() -> Markup {
    html! {
        form class="flex gap-2 items-center flex-wrap bg-white rounded shadow p-4"
            hx-post="/settings/endpoints" hx-target="#endpoint-list" hx-swap="outerHTML" {
            input class="border rounded px-2 py-1" type="text" name="name" placeholder="Name" required;
            input class="border rounded px-2 py-1 flex-1" type="text" name="base_url"
                placeholder="https://iot.example.com/iot/v1" required;
            select class="border rounded px-2 py-1" name="kind" {
                option value="general" { "General" }
                option value="edge" { "Edge" }
            }
            button class="bg-blue-600 text-white rounded px-3 py-1" type="submit" { "Add" }
        }
    }
}
