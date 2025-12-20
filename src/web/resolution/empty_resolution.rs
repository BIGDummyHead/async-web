use crate::web::{Resolution, resolution::get_status_header};

/// A resolution with no conetnt and just a status code.
pub struct EmptyResolution {
    status_code: i32,
}

impl EmptyResolution {
    pub fn new(status_code: i32) -> Box<dyn super::Resolution + Send> {
        let res = Self { status_code };

        Box::new(res) as Box<dyn super::Resolution + Send>
    }
}

impl Resolution for EmptyResolution {
    fn get_headers(&self) -> std::pin::Pin<Box<dyn Future<Output = Vec<String>> + Send + '_>> {
        Box::pin(async move { vec![get_status_header(self.status_code)] })
    }

    fn get_content(&self) -> std::pin::Pin<Box<dyn Future<Output = String> + Send + '_>> {
        Box::pin(async move { "".to_string() })
    }
}
