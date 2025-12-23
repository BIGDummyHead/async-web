use crate::web::{Resolution, resolution::{empty_content, get_status_header}};

/// ## Empty Resolution
/// 
/// Implementation of the Resolution trait. 
/// 
/// Simply creates an empty respond to send to the client with a status code you can set.
pub struct EmptyResolution {
    status_code: i32,
}

impl EmptyResolution {

    /// Create a new boxed Empty Resolution
    pub fn new(status_code: i32) -> Box<dyn super::Resolution + Send> {
        let res = Self { status_code };

        Box::new(res) as Box<dyn super::Resolution + Send>
    }
}

impl Resolution for EmptyResolution {
    fn get_headers(&self) -> std::pin::Pin<Box<dyn Future<Output = Vec<String>> + Send + '_>> {
        Box::pin(async move { vec![get_status_header(self.status_code)] })
    }

    fn get_content(&self) -> std::pin::Pin<Box<dyn Future<Output = Vec<u8>> + Send + '_>> {
        Box::pin(async move { empty_content() })
    }
}
