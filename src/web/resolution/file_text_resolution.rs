use std::{
    path::{Path, absolute},
    pin::Pin,
};

use tokio::fs;

use crate::web::resolution::get_status_header;

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
pub struct FileTextResolution<'a> {
    pub file: Option<Box<&'a Path>>,
    status_code: i32,
}

impl<'a> FileTextResolution<'a> {
    /// Create a new file resolution with status codes based on if the provided file exist.
    ///
    /// You can pass none into file_path which results in a 404 error.
    pub fn new(file_path: Option<&'a str>) -> Box<dyn super::Resolution + Send + 'a> {
        let mut path: Option<Box<&'a Path>> = None;

        let status_code = match file_path {
            None => 404,
            Some(f_path) => {
                let f_path: &'a Path = Path::new(f_path);

                let code = if f_path.exists() && f_path.is_file() {
                    200
                } else {
                    404
                };

                path = Some(Box::new(f_path));
                code
            }
        };

        Box::new(Self {
            status_code,
            file: path,
        }) as Box<dyn Resolution + Send + 'a>
    }
}

impl<'a> Resolution for FileTextResolution<'a> {
    fn get_headers(&self) -> Pin<Box<dyn Future<Output = Vec<String>> + Send + '_>> {
        Box::pin(async move { vec![get_status_header(self.status_code)] })
    }

    fn get_content(&self) -> Pin<Box<dyn Future<Output = Vec<u8>> + Send + '_>> {
        Box::pin(async move {
            //No content to serve.
            if self.file.is_none() {
                return Vec::new();
            }

            let path = self.file.as_ref().unwrap();

            let absolute_path = absolute(**path);

            //
            if let Err(_) = absolute_path {
                todo!()
            }

            let read_result = fs::read_to_string(&absolute_path.unwrap()).await;
            if let Ok(s) = read_result {
                return s.into_bytes();
            }

            todo!();
        })
    }
}
