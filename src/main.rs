use tide::{http::mime, Body, Redirect, Request, Response, Server, StatusCode};
use tide::prelude::*;

mod lib;
use lib::load::{Load, XlsxLoader};
use lib::item::Item;

#[derive(Debug, Deserialize)]
struct Due {
    name: String,
    reference: String,
}

#[async_std::main]
async fn main() -> tide::Result<()> {
    tide::log::start();
    let mut app = tide::new();
    app.at("/").get(uno_post);
    app.listen("127.0.0.1:8080").await?;
    Ok(())
}

async fn uno_post(mut req: Request<()>) -> tide::Result {
    //let uno = req.body_form().await?;
    let mut ld: XlsxLoader = Load::new("/Users/asterix/src/github/mergebom-web/boms/test0.xlsx");

    ld.read();
    let d = ld.raw_data();
    let mut items: Vec<Item> = Vec::new();
    for row in  d {
        let mut item = Item::new();
        item.push(row, ld.map_data());
        items.push(item);
    }

    let ret = serde_json::to_string(&items).unwrap();
    tide::log::info!("{}", ret);

    Ok(Response::builder(StatusCode::Ok).body(Body::from_json(&ret)?).build())
}