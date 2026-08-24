use axum::extract::{Path, State};
use axum::Form;
use maud::{html, Markup};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, Set};
use serde::Deserialize;

use crate::entities::{notify_target, project};
use crate::error::AppError;
use crate::state::AppState;
use crate::views::layout::page;

#[derive(Deserialize)]
pub struct NotifyTargetForm {
    pub name: String,
    pub webhook_url: String,
    #[serde(default)]
    pub message_template: String,
    #[serde(default)]
    pub enabled: Option<String>,
}

impl NotifyTargetForm {
    fn enabled(&self) -> bool {
        self.enabled.as_deref() == Some("on")
    }

    fn message_template(&self) -> Option<String> {
        let t = self.message_template.trim();
        if t.is_empty() {
            None
        } else {
            Some(t.to_string())
        }
    }
}

pub async fn list_page(
    State(state): State<AppState>,
    Path(project_id): Path<i32>,
) -> Result<Markup, AppError> {
    let Some(proj) = project::Entity::find_by_id(project_id).one(&state.db).await? else {
        return Ok(page(
            "Notify Targets",
            html! { p class="text-red-600" { "Project not found" } },
        ));
    };
    let list = render_list(&state, project_id).await?;
    Ok(page(
        "Notify Targets",
        html! {
            h1 class="text-2xl font-bold mb-1" { (proj.name) }
            p class="text-slate-500 mb-4" {
                "Only used while notify source is set to " strong { "Local" }
                " (currently: " (proj.notify_source) ")"
            }
            (render_form(project_id))
            div id="notify-target-list" class="mt-6" { (list) }
        },
    ))
}

pub async fn create(
    State(state): State<AppState>,
    Path(project_id): Path<i32>,
    Form(form): Form<NotifyTargetForm>,
) -> Result<Markup, AppError> {
    let message_template = form.message_template();
    let enabled = form.enabled();
    notify_target::ActiveModel {
        project_id: Set(project_id),
        name: Set(form.name),
        webhook_url: Set(form.webhook_url),
        message_template: Set(message_template),
        enabled: Set(enabled),
        ..Default::default()
    }
    .insert(&state.db)
    .await?;

    render_list(&state, project_id).await
}

pub async fn edit_form(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Markup, AppError> {
    let Some(row) = notify_target::Entity::find_by_id(id).one(&state.db).await? else {
        return Ok(html! { p class="text-red-600" { "Notify target not found" } });
    };
    Ok(render_row_edit(&row))
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Form(form): Form<NotifyTargetForm>,
) -> Result<Markup, AppError> {
    let Some(row) = notify_target::Entity::find_by_id(id).one(&state.db).await? else {
        return Ok(html! { p class="text-red-600" { "Notify target not found" } });
    };
    let project_id = row.project_id;
    let message_template = form.message_template();
    let enabled = form.enabled();
    let mut am = row.into_active_model();
    am.name = Set(form.name);
    am.webhook_url = Set(form.webhook_url);
    am.message_template = Set(message_template);
    am.enabled = Set(enabled);
    am.update(&state.db).await?;

    render_list(&state, project_id).await
}

pub async fn delete_target(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Markup, AppError> {
    let Some(row) = notify_target::Entity::find_by_id(id).one(&state.db).await? else {
        return Ok(html! { p { "Notify target not found" } });
    };
    let project_id = row.project_id;
    notify_target::Entity::delete_by_id(id).exec(&state.db).await?;
    render_list(&state, project_id).await
}

async fn render_list(state: &AppState, project_id: i32) -> Result<Markup, AppError> {
    let rows = notify_target::Entity::find()
        .filter(notify_target::Column::ProjectId.eq(project_id))
        .all(&state.db)
        .await?;

    Ok(html! {
        div id="notify-target-list" {
            @if rows.is_empty() {
                p class="text-slate-500" { "No notify targets yet." }
            } @else {
                table class="w-full text-sm bg-white rounded shadow" {
                    thead {
                        tr class="text-left text-slate-500 border-b" {
                            th class="py-2 px-3" { "Name" }
                            th class="py-2 px-3" { "Webhook URL" }
                            th class="py-2 px-3" { "Message Template" }
                            th class="py-2 px-3" { "Enabled" }
                            th class="py-2 px-3" { "" }
                        }
                    }
                    tbody {
                        @for row in &rows {
                            @let edit_url = format!("/settings/notify-targets/{}/edit", row.id);
                            @let del_url = format!("/settings/notify-targets/{}", row.id);
                            @let template_display = row.message_template.clone().unwrap_or_else(|| "(default)".to_string());
                            tr class="border-b last:border-0" {
                                td class="py-2 px-3" { (row.name) }
                                td class="py-2 px-3 text-slate-500 break-all" { (row.webhook_url) }
                                td class="py-2 px-3 text-slate-500" { (template_display) }
                                td class="py-2 px-3" { @if row.enabled { "yes" } @else { "no" } }
                                td class="py-2 px-3 flex gap-3" {
                                    button class="text-blue-600 underline"
                                        hx-get=(edit_url) hx-target="closest tr" hx-swap="outerHTML" { "Edit" }
                                    button class="text-red-600 underline"
                                        hx-delete=(del_url) hx-target="#notify-target-list" hx-swap="outerHTML"
                                        hx-confirm="Delete this notify target?" { "Delete" }
                                }
                            }
                        }
                    }
                }
            }
        }
    })
}

fn render_row_edit(row: &notify_target::Model) -> Markup {
    let update_url = format!("/settings/notify-targets/{}", row.id);
    let template = row.message_template.clone().unwrap_or_default();
    html! {
        tr {
            td class="py-2 px-3" colspan="5" {
                form class="flex gap-2 items-center flex-wrap"
                    hx-put=(update_url) hx-target="#notify-target-list" hx-swap="outerHTML" {
                    input class="border rounded px-2 py-1" type="text" name="name" value=(row.name) required;
                    input class="border rounded px-2 py-1 flex-1" type="text" name="webhook_url" value=(row.webhook_url) required;
                    input class="border rounded px-2 py-1 flex-1" type="text" name="message_template" value=(template)
                        placeholder="{device_name} {status} {time}";
                    label class="flex items-center gap-1 text-sm" {
                        input type="checkbox" name="enabled" checked[row.enabled];
                        "Enabled"
                    }
                    button class="text-blue-600 underline" type="submit" { "Save" }
                }
            }
        }
    }
}

fn render_form(project_id: i32) -> Markup {
    let create_url = format!("/settings/projects/{project_id}/notify-targets");
    html! {
        form class="flex gap-2 items-center flex-wrap bg-white rounded shadow p-4"
            hx-post=(create_url) hx-target="#notify-target-list" hx-swap="outerHTML" {
            input class="border rounded px-2 py-1" type="text" name="name" placeholder="Label" required;
            input class="border rounded px-2 py-1 flex-1" type="text" name="webhook_url"
                placeholder="https://discord.com/api/webhooks/..." required;
            input class="border rounded px-2 py-1 flex-1" type="text" name="message_template"
                placeholder="{device_name} {status} {time} (optional)";
            label class="flex items-center gap-1 text-sm" {
                input type="checkbox" name="enabled" checked;
                "Enabled"
            }
            button class="bg-blue-600 text-white rounded px-3 py-1" type="submit" { "Add" }
        }
    }
}
