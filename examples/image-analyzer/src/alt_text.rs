use async_web::web::{Resolution, resolution::json_resolution::JsonResolution};


#[derive(serde::Serialize)]
pub struct AltText {
    pub text: Option<String>,
    pub error: Option<String>
}

impl AltText {
    pub fn with_error(error: String) -> Self {
        AltText { text: None, error: Some(error) }
    }

    pub fn with_value(text: String) -> Self {
        AltText { text: Some(text), error: None }
    }

    pub fn as_resolution(&self) -> Box<dyn Resolution + Send + 'static> {

        let mut json_res = JsonResolution::new(self).unwrap();

        let code = match self.error {
            None => 200,
            Some(_) => 500
        };

        json_res.set_status(code);

        json_res.into_resolution()
    }
}