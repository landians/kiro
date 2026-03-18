use axum::{Router, response::Html, routing::get};

pub fn build_routes() -> Router {
    let router = Router::new().route("/", get(handler));
    router
}

async fn handler() -> Html<&'static str> {
    Html("<h1>Hello, World!</h1>")
}
