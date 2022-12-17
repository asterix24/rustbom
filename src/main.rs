use askama::Template;
use axum::{
    body::Bytes,
    extract::Multipart,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
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

use mergebom_web::{
    bom::{merge_key_list, Bom, ItemsTable},
    outjob::OutJobXlsx,
};

const STATIC_DIRECTORY: &str = "static";
const UPLOADS_DIRECTORY: &str = "static/uploads";
const MERGED_DIRECTORY: &str = "static/merged";

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "MergeBom-Web=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let app: _ = Router::new()
        .route("/", get(render_index))
        .route("/view", post(merge_view_post))
        .route("/jobs", post(jobs_done))
        .route("/upload", post(accept_form))
        .merge(axum_extra::routing::SpaRouter::new(
            "/static",
            STATIC_DIRECTORY,
        ));

    // run it with hyper
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

fn files_on_server(path: &str) -> Vec<String> {
    let mut uploaded_files = vec![];
    for i in ["*.csv", "*.xlsx", "*.xls"] {
        let upf: Vec<String> = glob(Path::new(path).join(i).to_str().unwrap())
            .unwrap()
            .map(|path| {
                path.unwrap()
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string()
            })
            .collect();
        uploaded_files.extend(upf);
    }
    uploaded_files
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    static_dir: String,
    merge_dir: String,
    upload_dir: String,
    uploaded_bom_list: Vec<String>,
    merged_bom_list: Vec<String>,
    merge_key_list: Vec<String>,
}
struct HtmlTemplate<T>(T);

impl<T> IntoResponse for HtmlTemplate<T>
where
    T: Template,
{
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(html) => Html(html).into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to render template. Error: {}", err),
            )
                .into_response(),
        }
    }
}

async fn render_index() -> impl IntoResponse {
    HtmlTemplate(IndexTemplate {
        static_dir: STATIC_DIRECTORY.to_string(),
        merge_dir: MERGED_DIRECTORY.to_string(),
        upload_dir: UPLOADS_DIRECTORY.to_string(),
        uploaded_bom_list: files_on_server(UPLOADS_DIRECTORY),
        merged_bom_list: files_on_server(MERGED_DIRECTORY),
        merge_key_list: merge_key_list(),
    })
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
struct JobDone {
    merged_files: Vec<String>,
}
async fn jobs_done() -> Json<JobDone> {
    Json(JobDone {
        merged_files: files_on_server(MERGED_DIRECTORY),
    })
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
struct MergeCfg {
    merge_file_name: String,
    merge_files: Vec<String>,
    merge_keys: Vec<String>,
}

async fn merge_view_post(Json(payload): Json<MergeCfg>) -> Json<ItemsTable> {
    let files: Vec<_> = payload
        .merge_files
        .iter()
        .map(|f| Path::new(UPLOADS_DIRECTORY).join(f))
        .collect();

    let mut file_name = "merged_bom.xlsx".to_string();
    if !payload.merge_file_name.is_empty() {
        file_name = payload.merge_file_name;
    }

    let bom = Bom::loader(files.as_slice(), &payload.merge_keys);
    let data = bom.merge().odered_vector_table();
    OutJobXlsx::new(Path::new(MERGED_DIRECTORY).join(file_name)).write(&data);
    Json(data)
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

    Json(ReplyStatus {
        uploaded_files: files_on_server(UPLOADS_DIRECTORY),
        error: "".to_string(),
    })
}

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
    let path = std::path::Path::new(path);
    let mut components = path.components().peekable();

    if let Some(first) = components.peek() {
        if !matches!(first, std::path::Component::Normal(_)) {
            return false;
        }
    }
    components.count() == 1
}
