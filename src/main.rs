use tide::Request;
use tide::prelude::*;

mod items;

#[derive(Debug, Deserialize)]
struct Animal {
    name: String,
    legs: u16,
}

#[async_std::main]
async fn main() -> tide::Result<()> {
    let mut app = tide::new();
    app.at("/orders/shoes").post(order_shoes);
    app.at("/").post(uno_post);
    app.listen("127.0.0.1:8080").await?;
    Ok(())
}

async fn order_shoes(mut req: Request<()>) -> tide::Result {
    let Animal { name, legs } = req.body_json().await?;
    Ok(format!("Hello, {}! I've put in an order for {} shoes", name, legs).into())
}

async fn uno_post(mut req: Request<()>) -> tide::Result {
    let Animal { name, legs } = req.body_json().await?;
    
    Ok(items::uno(name, legs).into())
}