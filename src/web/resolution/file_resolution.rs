use futures::Stream;

use crate::web::{Resolution, resolution::get_status_header, streams::stream_file};

/// # File Resolution
///
/// Resolution that gives you the ability to serve files as an array of bytes.
///
/// This is useful for a content folder where you need to serve non-text based files.
///
/// ## Example
///
/// ```
///  let file_resolution = FileResolution::new("/images/profile_image.png".to_string());
/// ```
///
/// The content type of the file is determined based on the extension, this header is passed via the Resolution::get_headers function.
///
/// The status of the file is determined based on if the file exist.
///
/// If the file `exist` than the status is `200`
///
/// If the file `does not exist` than the status is `404`
///
pub struct FileResolution {
    pub file_path: String,
}

impl FileResolution {
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

impl Resolution for FileResolution {
    fn get_headers(&self) -> std::pin::Pin<Box<dyn Future<Output = Vec<String>> + Send + '_>> {
        Box::pin(async move {
            vec![
                get_status_header(self.get_status()),
                self.get_file_type_header(),
            ]
        })
    }

    fn get_content(&self) -> std::pin::Pin<Box<dyn Stream<Item = Vec<u8>> + Send + 'static>> {
        let file_path = self.file_path.clone();

        Box::pin(stream_file(file_path))
    }
}
