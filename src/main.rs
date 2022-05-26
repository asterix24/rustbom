use axum::routing::post;
use axum::{response::Html, routing::get, Json, Router};
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod lib;
use lib::bom::{Bom, ItemView};

//use serde::{Deserialize, Serialize};

// use lib::item::Item;
//use lib::load::XlsxLoader;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "example_form=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let app: _ = Router::new()
        .route("/", get(show_form))
        .route("/view", post(merge_view_post))
        .route("/data", post(merge_post));

    // run it with hyper
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn show_form() -> Html<&'static str> {
    Html(std::include_str!("../templates/index.html"))
}

async fn merge_post() -> Json<Vec<Vec<String>>> {
    let bom = Bom::from_csv("./boms/test.csv").unwrap();
    Json(bom.merge().odered_vector())
}

async fn merge_view_post() -> Json<Vec<ItemView>> {
    let bom = Bom::from_csv("./boms/test.csv").unwrap();
    Json(bom.merge().odered_vector_view())
}
