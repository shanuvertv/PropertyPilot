use axum::extract::DefaultBodyLimit;
use axum::routing::{delete, get, post, put};
use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::state::AppState;

mod auth;
mod automation;
mod contracts;
mod dashboard;
mod documents;
mod email;
mod expenses;
mod import;
pub mod master;
mod renewals;
mod reports;
mod system;
mod users;
mod web;

pub fn router(state: AppState, web_dir: Option<std::path::PathBuf>) -> Router {
    // Bearer tokens, no cookies: a permissive CORS policy is safe and lets the
    // desktop webview (http://tauri.localhost), Android and `vite dev` all call in.
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
        .expose_headers(Any);

    let uploads = Router::new()
        .route(
            "/api/documents",
            get(documents::list).post(documents::upload),
        )
        .route("/api/import/preview", post(import::preview))
        .route("/api/import/commit", post(import::commit))
        .layer(DefaultBodyLimit::max(
            renewal_services::documents::MAX_BYTES + 64 * 1024,
        ));

    Router::new()
        // phase 0
        .route("/api/health", get(system::health))
        .route("/api/system/status", get(system::status))
        .route("/api/auth/bootstrap", post(auth::bootstrap))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/logout", post(auth::logout))
        .route("/api/auth/me", get(auth::me))
        .route("/api/auth/password", put(auth::change_password))
        .route("/api/users/{id}/password", put(auth::reset_password))
        .route("/api/users", get(users::list).post(users::create))
        .route("/api/users/{id}", delete(users::delete))
        .route("/api/users/{id}/active", put(users::set_active))
        .route("/api/employees", get(dashboard::employees))
        // phase 1
        .route(
            "/api/buildings",
            get(master::list_buildings).post(master::create_building),
        )
        .route("/api/buildings/options", get(master::building_options))
        .route(
            "/api/buildings/{id}",
            get(master::get_building)
                .put(master::update_building)
                .delete(master::archive_building),
        )
        .route(
            "/api/units",
            get(master::list_units).post(master::create_unit),
        )
        .route(
            "/api/units/{id}",
            get(master::get_unit)
                .put(master::update_unit)
                .delete(master::archive_unit),
        )
        .route("/api/units/{id}/status", put(master::set_unit_status))
        .route(
            "/api/tenants",
            get(master::list_tenants).post(master::create_tenant),
        )
        .route("/api/tenants/options", get(master::tenant_options))
        .route(
            "/api/tenants/{id}",
            get(master::get_tenant)
                .put(master::update_tenant)
                .delete(master::archive_tenant),
        )
        .merge(uploads)
        .route("/api/documents/{id}", delete(documents::delete))
        .route("/api/documents/{id}/download", get(documents::download))
        // phase 2
        .route(
            "/api/contracts",
            get(contracts::list).post(contracts::create),
        )
        .route(
            "/api/contracts/suggest-number",
            get(contracts::suggest_number),
        )
        .route(
            "/api/contracts/expire-overdue",
            post(contracts::expire_overdue),
        )
        .route(
            "/api/contracts/{id}",
            get(contracts::get).put(contracts::update),
        )
        .route("/api/contracts/{id}/activate", post(contracts::activate))
        .route("/api/contracts/{id}/terminate", post(contracts::terminate))
        .route("/api/contracts/{id}/assign", put(contracts::assign))
        .route(
            "/api/contracts/{id}/renewal",
            post(contracts::start_renewal),
        )
        // phase 3
        .route("/api/dashboard", get(dashboard::get))
        .route("/api/search", get(dashboard::search))
        // phase 4
        .route("/api/renewals", get(renewals::list_cases))
        .route(
            "/api/renewals/checklist-template",
            get(renewals::get_template).put(renewals::save_template),
        )
        .route("/api/renewals/{id}", get(renewals::get_case))
        .route("/api/renewals/{id}/status", put(renewals::set_status))
        .route("/api/renewals/{id}/assign", put(renewals::assign))
        .route("/api/renewals/{id}/notes", put(renewals::set_notes))
        .route(
            "/api/renewals/{id}/responses",
            post(renewals::record_response),
        )
        .route(
            "/api/renewals/{id}/checklist/{item}",
            put(renewals::set_checklist_item),
        )
        .route("/api/renewals/{id}/complete", post(renewals::complete))
        .route(
            "/api/renewals/{id}/follow-ups",
            post(renewals::create_follow_up),
        )
        .route("/api/follow-ups", get(renewals::list_follow_ups))
        .route("/api/follow-ups/counts", get(renewals::follow_up_counts))
        .route("/api/follow-ups/{id}", put(renewals::update_follow_up))
        .route(
            "/api/follow-ups/{id}/status",
            put(renewals::set_follow_up_status),
        )
        // phase 5
        .route("/api/email-templates", get(email::list_templates))
        .route(
            "/api/email-templates/placeholders",
            get(email::placeholders),
        )
        .route("/api/email-templates/{key}", put(email::update_template))
        .route(
            "/api/email-templates/{key}/preview",
            post(email::preview_template),
        )
        .route(
            "/api/emails",
            get(email::list_messages).post(email::compose),
        )
        .route("/api/emails/{id}", get(email::get_message))
        .route("/api/emails/{id}/retry", post(email::retry))
        .route("/api/system/mail", get(email::mail_status))
        .route(
            "/api/renewals/{id}/notice",
            get(email::notice_workspace).put(email::save_notice_draft),
        )
        .route("/api/renewals/{id}/notice/pdf", post(email::notice_pdf))
        .route("/api/renewals/{id}/notice/send", post(email::send_notice))
        // phase 6
        .route("/api/notifications", get(automation::list))
        .route("/api/notifications/count", get(automation::unread_count))
        .route(
            "/api/notifications/read-all",
            post(automation::mark_all_read),
        )
        .route("/api/notifications/{id}/read", put(automation::mark_read))
        .route("/api/events", get(automation::events))
        .route(
            "/api/settings/org",
            get(automation::get_org).put(automation::save_org),
        )
        .route(
            "/api/settings/mail",
            get(automation::mail_settings_get).put(automation::mail_settings_put),
        )
        .route(
            "/api/settings/mail/test",
            post(automation::mail_settings_test),
        )
        .route(
            "/api/settings/reminder-rules",
            get(automation::get_rules).put(automation::save_rules),
        )
        .route("/api/system/sweep", post(automation::run_sweep))
        // number of tenants per unit & expenses
        .route(
            "/api/units/{id}/occupant-count",
            put(master::set_unit_occupant_count),
        )
        .route(
            "/api/expenses",
            get(expenses::list_expenses).post(expenses::create_expense),
        )
        .route("/api/expenses/summary", get(expenses::summary))
        .route(
            "/api/expenses/{id}",
            get(expenses::get_expense)
                .put(expenses::update_expense)
                .delete(expenses::delete_expense),
        )
        .route("/api/expenses/{id}/split", post(expenses::split_equal))
        .route("/api/expenses/{id}/settled", put(expenses::settle))
        // phase 7
        .route("/api/reports/{kind}", get(reports::report))
        .route("/api/audit", get(reports::audit_list))
        .route(
            "/api/audit/{entity_type}/{id}",
            get(reports::audit_for_entity),
        )
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .fallback_service(web::fallback(web_dir))
        .with_state(state)
}
