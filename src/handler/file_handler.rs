use crate::dto::{ApiResponse, UploadFileResponse};
use axum::body::Body;
use axum::extract::{Multipart, Path};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use tokio::io::AsyncWriteExt;

pub async fn upload_file(mut multipart: Multipart) -> impl IntoResponse {
    let mut file_paths = Vec::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        let file_name = field
            .file_name()
            .unwrap_or("uploaded_file")
            .to_string();

        let upload_dir = "./uploads";
        if let Err(e) = tokio::fs::create_dir_all(upload_dir).await {
            return ApiResponse::<()>::error(&format!("Failed to create upload dir: {e}"))
                .into_response();
        }

        let dest = format!("{upload_dir}/{file_name}");
        match field.bytes().await {
            Ok(data) => {
                let mut file = match tokio::fs::File::create(&dest).await {
                    Ok(f) => f,
                    Err(e) => {
                        return ApiResponse::<()>::error(&format!("Failed to create file: {e}"))
                            .into_response();
                    }
                };
                if let Err(e) = file.write_all(&data).await {
                    return ApiResponse::<()>::error(&format!("Failed to write file: {e}"))
                        .into_response();
                }
                file_paths.push(format!("local:{dest}"));
            }
            Err(e) => {
                return ApiResponse::<()>::error(&format!("Failed to read field: {e}"))
                    .into_response();
            }
        }
    }

    ApiResponse::success(UploadFileResponse {
        file_path: file_paths,
    })
    .into_response()
}

pub async fn download_file(Path(filepath): Path<String>) -> Response {
    let path = std::path::Path::new(&filepath);

    // Security: prevent path traversal
    if filepath.contains("..") {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            "Invalid path",
        )
            .into_response();
    }

    match tokio::fs::read(path).await {
        Ok(data) => {
            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("download");

            Response::builder()
                .header(
                    header::CONTENT_DISPOSITION,
                    format!("attachment; filename=\"{filename}\""),
                )
                .header(header::CONTENT_TYPE, "application/octet-stream")
                .body(Body::from(data))
                .unwrap()
        }
        Err(_) => (axum::http::StatusCode::NOT_FOUND, "File not found").into_response(),
    }
}
