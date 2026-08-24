pub mod dashboard;
pub mod endpoints;
pub mod history;
pub mod layout;
pub mod notify_targets;
pub mod projects;

use axum::routing::get;
use axum::Router;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(dashboard::dashboard))
        .route("/partials/dashboard", get(dashboard::dashboard_partial))
        .route(
            "/settings/endpoints",
            get(endpoints::list_page).post(endpoints::create),
        )
        .route("/settings/endpoints/{id}/edit", get(endpoints::edit_form))
        .route(
            "/settings/endpoints/{id}",
            axum::routing::put(endpoints::update).delete(endpoints::delete_endpoint),
        )
        .route(
            "/settings/endpoints/{endpoint_id}/projects",
            get(projects::list_page).post(projects::create),
        )
        .route("/settings/projects/{id}/edit", get(projects::edit_form))
        .route(
            "/settings/projects/{id}",
            axum::routing::put(projects::update).delete(projects::delete_project),
        )
        .route(
            "/settings/projects/{project_id}/notify-targets",
            get(notify_targets::list_page).post(notify_targets::create),
        )
        .route(
            "/settings/notify-targets/{id}/edit",
            get(notify_targets::edit_form),
        )
        .route(
            "/settings/notify-targets/{id}",
            axum::routing::put(notify_targets::update).delete(notify_targets::delete_target),
        )
        .route("/history", get(history::history_page))
}
