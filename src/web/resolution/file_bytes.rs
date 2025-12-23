use std::f64::consts::E;

use tokio::{fs, io::AsyncReadExt};

use crate::web::{Resolution, resolution::{empty_content, get_status_header}};

/// # File Bytes
///
/// Gives you the ability to serve
pub struct FileBytes {
    pub file_path: String,
}

impl FileBytes {

    pub fn new(file_path: String) -> Box<dyn super::Resolution + Send> {
        let res = Self { file_path };

        Box::new(res) as Box<dyn super::Resolution + Send>
    }

    /// Retrieves the file type for a header.
    fn get_file_type_header(&self) -> String {
        // extract extension (lowercased)
        let ext = match std::path::Path::new(&self.file_path)
            .extension()
            .and_then(|e| e.to_str())
        {
            Some(e) => e.to_lowercase(),
            None => return "application/octet-stream".to_string(),
        };

        match ext.as_str() {
            // text types
            "html" | "htm" => "text/html",
            "css" => "text/css",
            "js" => "application/javascript",
            "json" => "application/json",
            "txt" => "text/plain",
            "csv" => "text/csv",
            "xml" => "application/xml",

            // images
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            "gif" => "image/gif",
            "bmp" => "image/bmp",
            "webp" => "image/webp",
            "svg" => "image/svg+xml",
            "ico" => "image/x-icon",

            // audio / video
            "mp3" => "audio/mpeg",
            "wav" => "audio/wav",
            "ogg" => "audio/ogg",
            "mp4" => "video/mp4",
            "webm" => "video/webm",

            // fonts
            "woff" => "font/woff",
            "woff2" => "font/woff2",
            "ttf" => "font/ttf",
            "otf" => "font/otf",

            // documents / archives
            "pdf" => "application/pdf",
            "zip" => "application/zip",
            "tar" => "application/x-tar",
            "gz" => "application/gzip",

            // fallback
            _ => "application/octet-stream",
        }
        .to_string()
    }

    fn get_status(&self) -> i32 {
        let path = std::path::Path::new(&self.file_path);

        if path.exists() {
            return 200;
        }

        404
    }
}

impl Resolution for FileBytes {
    fn get_headers(&self) -> std::pin::Pin<Box<dyn Future<Output = Vec<String>> + Send + '_>> {
        Box::pin(async move { vec![get_status_header(self.get_status()), self.get_file_type_header()] })
    }

    fn get_content(&self) -> std::pin::Pin<Box<dyn Future<Output = Vec<u8>> + Send + '_>> {
        Box::pin(async move {
            if self.get_status() != 200 {
                return empty_content();
            }

            let file_open = fs::File::open(&self.file_path).await;

            if file_open.is_err() {
                return empty_content();
            }

            let mut file = file_open.unwrap();

            let mut buffer = Vec::new();

            if let Err(e) = file.read_to_end(&mut buffer).await {
                todo!("Failed to read to end: {e}");
            }

            buffer
        })
    }
}
