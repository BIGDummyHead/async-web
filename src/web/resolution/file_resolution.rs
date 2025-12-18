use std::{
    path::{Path, absolute},
    pin::Pin,
};

use tokio::fs;

use crate::web::resolution::get_status_header;

use super::Resolution;

/// Provides a resolution for serving files back to the client.
pub struct FileResolution<'a> {
    pub file: Option<Box<&'a Path>>,
    status_code: i32,
}

impl<'a> FileResolution<'a> {
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

impl<'a> Resolution for FileResolution<'a> {
    fn get_headers(&self) -> Pin<Box<dyn Future<Output = Vec<String>> + Send + '_>> {
        Box::pin(async move { vec![get_status_header(self.status_code)] })
    }

    fn get_content(&self) -> Pin<Box<dyn Future<Output = String> + Send + '_>> {
        Box::pin(async move {
            //No content to serve.
            if self.file.is_none() {
                return "".to_string();
            }

            let path = self.file.as_ref().unwrap();

            let absolute_path = absolute(**path);

            //
            if let Err(_) = absolute_path {
                todo!()
            }

            let read_result = fs::read_to_string(&absolute_path.unwrap()).await;
            if let Ok(s) = read_result {
                return s;
            }

            todo!();
        })
    }
}
