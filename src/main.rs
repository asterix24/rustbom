use askama::Template;
use axum::routing::post;
use axum::{extract::Form, response::Html, routing::get, Json, Router};
use serde::Deserialize;
use serde_json::json;
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod lib;
use lib::item::Item;
use lib::load::{Load, XlsxLoader};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "example_form=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let app = Router::new()
        .route("/", get(show_form))
        .route("/data/", post(merge_view_post));

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

async fn merge_view_post() -> Json<Item> {
    let item = Item::new();
    Json(item)
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct Input {
    name: String,
    email: String,
}

async fn preview_post(Form(input): Form<Input>) {
    dbg!(&input);
}

// async fn preview_post(mut req: Request<()>) -> tide::Result {
//     //let uno = req.body_json().await?;
//     //ide::log::info!("{:?}", uno);
//     let mut ld: XlsxLoader = Load::new("/Users/asterix/src/github/mergebom-web/boms/test0.xlsx");

//     ld.read();
//     let d = ld.raw_data();
//     let mut items: Vec<Item> = Vec::new();

//     for row in d {
//         let mut item = Item::new();
//         item.push(row, ld.map_data());
//         items.push(item);
//     }

//     match Body::from_json(&items) {
//         Ok(body) => Ok(body.into()),
//         Err(_) => Ok(Response::new(StatusCode::NotFound)),
//     }
// }
