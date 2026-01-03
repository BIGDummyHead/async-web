use std::pin::Pin;

use futures::{Stream, stream};
use tokio::fs;

use crate::web::resolution::{empty_content, get_status_header};

use super::Resolution;

/// ## File Text Resolution
///
/// This type of resolution provides the contents of a file as a Utf8 string
///
/// It reads the entire file line by line until it is completely read.
///
/// ### Note: This should only be used for serving pure text files and none content related files like (images, pdfs, and other data related files)
///
/// ## Example
///
/// ```
/// // -- snip --
/// let file_resolution = FileResolution::new("/content/item.pdf");
/// ```
///
/// This could be used for a dynamic content folder if you give the ability of using wildcards in your router.
pub struct FileTextResolution {
    file_path: String,
}

impl FileTextResolution {
    /// Create a new file resolution with status codes based on if the provided file exist.
    ///
    /// You can pass none into file_path which results in a 404 error.
    pub fn new(file_path: &str) -> Box<dyn super::Resolution + Send> {
        Box::new(Self {
            file_path: file_path.to_string(),
        }) as Box<dyn Resolution + Send>
    }
}

impl Resolution for FileTextResolution {
    fn get_headers(&self) -> Pin<Box<dyn Future<Output = Vec<String>> + Send + '_>> {
        Box::pin(async move {
            let exist = fs::try_exists(&self.file_path).await;

            let exist = exist.unwrap_or(false);

            let status_code = if exist { 200 } else { 404 };

            vec![get_status_header(status_code)]
        })
    }

    fn get_content(&self) -> Pin<Box<dyn Stream<Item = Vec<u8>> + Send + 'static>> {
        let file_path = self.file_path.clone();

        Box::pin(stream::once(async move {
            let read_result = fs::read_to_string(file_path).await;
            if let Ok(s) = read_result {
                return s.into_bytes();
            }

            empty_content()
        }))
    }
}
