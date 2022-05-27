use axum::{
    body::Bytes,
    extract::Multipart,
    http::StatusCode,
    response::Html,
    routing::{get, post},
    BoxError, Json, Router,
};
use futures::{Stream, TryStreamExt};
use glob::glob;
use serde::{Deserialize, Serialize};
use std::{io, net::SocketAddr, path::Path, vec};
use tokio::{fs::File, io::BufWriter};
use tokio_util::io::StreamReader;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod lib;
use lib::bom::{Bom, ItemView};

const UPLOADS_DIRECTORY: &str = "uploads";

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "MergeBom-Web=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();
    // tokio::fs::create_dir(UPLOADS_DIRECTORY)
    //     .await
    //     .expect("failed to create `uploads` directory");

    let app: _ = Router::new()
        .route("/", get(show_form))
        .route("/view", post(merge_view_post))
        .route("/data", post(merge_post))
        .route("/upload", post(accept_form));

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

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
struct MergeCfg {
    merge_files: Vec<String>,
}

async fn merge_view_post(Json(payload): Json<MergeCfg>) -> Json<Vec<ItemView>> {
    tracing::debug!("{:?}", payload);

    // for i in payload.merge_files.iter() {
    let bom =
        Bom::from_csv(Path::new(UPLOADS_DIRECTORY).join(payload.merge_files.first().unwrap()))
            .unwrap();
    // }
    Json(bom.merge().odered_vector_view())
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
struct ReplyStatus {
    uploaded_files: Vec<String>,
    error: String,
}

// Handler that accepts a multipart form upload and streams each field to a file.
async fn accept_form(mut multipart: Multipart) -> Json<ReplyStatus> {
    while let Some(field) = multipart.next_field().await.unwrap() {
        let file_name = if let Some(file_name) = field.file_name() {
            file_name.to_owned()
        } else {
            continue;
        };
        tracing::debug!("{}", file_name);

        if let Err(e) = stream_to_file(&file_name, field).await {
            let error = format!("failed to save file: {:?}", e);
            tracing::error!("{}", error);
            return Json(ReplyStatus {
                uploaded_files: vec!["".to_string()],
                error,
            });
        }
    }

    let mut uploaded_files = vec![];
    for i in ["*.csv", "*.xlsx", "*.xls"] {
        uploaded_files.push(
            glob(format!("{}/{}", UPLOADS_DIRECTORY, i).as_str())
                .unwrap()
                .map(|path| path.unwrap().to_str().unwrap().to_string())
                .collect(),
        );
    }

    Json(ReplyStatus {
        uploaded_files,
        error: "".to_string(),
    })
}

// Save a `Stream` to a file
async fn stream_to_file<S, E>(path: &str, stream: S) -> Result<(), (StatusCode, String)>
where
    S: Stream<Item = Result<Bytes, E>>,
    E: Into<BoxError>,
{
    if !path_is_valid(path) {
        return Err((StatusCode::BAD_REQUEST, "Invalid path".to_owned()));
    }

    async {
        // Convert the stream into an `AsyncRead`.
        let body_with_io_error = stream.map_err(|err| io::Error::new(io::ErrorKind::Other, err));
        let body_reader = StreamReader::new(body_with_io_error);
        futures::pin_mut!(body_reader);

        // Create the file. `File` implements `AsyncWrite`.
        let path = std::path::Path::new(UPLOADS_DIRECTORY).join(path);
        let mut file = BufWriter::new(File::create(path).await?);

        // Copy the body into the file.
        tokio::io::copy(&mut body_reader, &mut file).await?;

        Ok::<_, io::Error>(())
    }
    .await
    .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))
}

// to prevent directory traversal attacks we ensure the path conists of exactly one normal
// component
fn path_is_valid(path: &str) -> bool {
    let path = std::path::Path::new(&*path);
    let mut components = path.components().peekable();

    if let Some(first) = components.peek() {
        if !matches!(first, std::path::Component::Normal(_)) {
            return false;
        }
    }
    components.count() == 1
}
