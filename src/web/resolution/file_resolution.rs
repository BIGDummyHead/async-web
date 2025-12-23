use std::{
    path::{Path, absolute},
    pin::Pin,
};

use tokio::fs;

use crate::web::resolution::get_status_header;

use super::Resolution;


/// ## File Resolution
/// 
/// Gives the abilitiy to serve a file back to a client. 
/// 
/// Simply takes the path of the file to use and allows you to send it back.
/// 
/// If the file does not exist a 404 is given back to the client
/// 
/// ## Example
/// 
/// ```
/// // -- snip --
/// let file_resolution = FileResolution::new("/content/item.pdf"); 
/// ```
/// 
/// This could be used for a dynamic content folder if you give the ability of using wildcards in your router.
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
