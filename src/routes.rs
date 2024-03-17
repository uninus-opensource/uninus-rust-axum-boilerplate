use axum::{
    middleware::{self},
    routing::{get, post},
    Json, Router,
};
use serde_json::json;
use sqlx::{Pool, Postgres};
use utoipa::{
    openapi::security::{ApiKey, ApiKeyValue, SecurityScheme},
    Modify, OpenApi,
};
use utoipa_rapidoc::RapiDoc;
use utoipa_swagger_ui::SwaggerUi;

use rust_todo_api::{
    auth::{self, models::JWT_SECRET},
    common,
    common::middlewares::{auth_middleware, AuthState},
    todo, user,
};

pub fn build_routes(pool: Pool<Postgres>) -> Router {
    #[derive(OpenApi)]
    #[openapi(
        paths(
            auth::handlers::get_tokens,
            user::handlers::register_user,
            user::handlers::find_user_by_email,
            todo::handlers::create_todo,
            todo::handlers::find_todos,
            todo::handlers::find_todo_by_id,
            todo::handlers::edit_todo_by_id,
            todo::handlers::delete_todo_by_id,
        ),
        components(
            schemas(
                common::errors::ApiError,
                common::pagination::PaginatedTodoView,
                auth::views::LoginRequest,
                auth::views::TokenView,
                user::views::NewUserRequest,
                user::views::UserView,
                todo::views::TodoView,
                todo::views::NewTodoRequest,
                todo::views::EditTodoRequest,
            )
        ),
        info(
            title = "Todo API",
            description = "A simple todo API",
            version = "0.1.0",
            contact(
                name = "",
                url = ""
            ),
            license(
                name = "MIT",
                url = "https://opensource.org/licenses/MIT",
            )
        ),
        modifiers(&SecurityAddon),
        tags(
            (name = "auth", description = "Authentication API"),
            (name = "user", description = "User API"),
            (name = "todo", description = "Todo API")
        )
    )]
    struct ApiDoc;

    struct SecurityAddon;
    impl Modify for SecurityAddon {
        fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
            if let Some(components) = openapi.components.as_mut() {
                components.add_security_scheme(
                    "api_key",
                    SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("Authorization"))),
                )
            }
        }
    }

    let auth_state = AuthState {
        pool,
        jwt_secret: JWT_SECRET.to_string(),
    };

    let api_routes = Router::new()
        .nest(
            "/auth",
            Router::new().route("/tokens", post(auth::handlers::get_tokens)),
        )
        .nest(
            "/todos",
            Router::new()
                .route(
                    "/",
                    get(todo::handlers::find_todos).post(todo::handlers::create_todo),
                )
                .route(
                    "/:id",
                    get(todo::handlers::find_todo_by_id)
                        .put(todo::handlers::edit_todo_by_id)
                        .delete(todo::handlers::delete_todo_by_id),
                )
                .route_layer(middleware::from_fn_with_state(
                    auth_state.clone(),
                    auth_middleware,
                )),
        )
        .nest(
            "/users",
            Router::new()
                .route(
                    "/",
                    get(user::handlers::find_user_by_email).route_layer(
                        middleware::from_fn_with_state(auth_state.clone(), auth_middleware),
                    ),
                )
                .route("/", post(user::handlers::register_user)),
        );

    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .merge(RapiDoc::new("/api-docs/openapi.json").path("/rapidoc"))
        .route("/health", get(|| async { Json(json!({ "status": "ok" })) }))
        .nest("/api", api_routes.with_state(auth_state))
}
