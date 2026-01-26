use async_web::web::{Resolution, resolution::json_resolution::JsonResolution};

#[derive(serde::Serialize)]
pub struct AltText {
    pub text: Option<String>,
    pub error: Option<String>,
}

impl AltText {
    
    /// Serve resolution with error
    pub fn with_error(error: String) -> Self {
        AltText {
            text: None,
            error: Some(error),
        }
    }

    /// serve with some value
    pub fn with_value(text: String) -> Self {
        AltText {
            text: Some(text),
            error: None,
        }
    }

    
    pub fn resolve(self) -> Box<dyn Resolution + Send + 'static> {
        match JsonResolution::serialize(self) {
            Ok(j_res) => j_res.resolve(),
            Err(e_res) => e_res.resolve(),
        }
    }
}
