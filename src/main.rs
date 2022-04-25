use axum::routing::post;
use axum::{response::Html, routing::get, Json, Router};
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod lib;
use lib::bom::Bom;

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
        .route("/data", post(merge_view_post));

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

async fn merge_view_post() -> Json<Vec<Vec<String>>> {
    //let bom = XlsxLoader::open("/Users/asterix/src/github/mergebom-web/boms/test0.xlsx").read();
    //let bom = Bom::from_csv("/Users/asterix/src/github/mergebom-web/boms/test.csv").unwrap();
    let bom = Bom::from_xlsx("/Users/asterix/src/github/mergebom-web/boms/bomx.xlsx").unwrap();

    let ret: Vec<Vec<String>> = bom.merge().collect();
    println!("{:#?}", ret);
    Json(ret)
}
