use std::fs;

/// Represents a resolution for a request
pub trait Resolution {
    ///
    /// Get all headers for the HTTP response.
    ///
    fn get_headers(&self) -> Vec<String>;

    ///
    /// Get the content for the resolution. Gets pushed into the headers. Then a length is used.
    fn get_content(&self) -> String;
}


pub struct FileResolution {
    pub file: String
}

impl Resolution for FileResolution {
    fn get_headers(&self) -> Vec<String> {
        vec!["HTTP/1.1 200 OK".to_string()]
    }

    fn get_content(&self) -> String {

        let read_result = fs::read_to_string(&self.file);
        if let Ok(s) = read_result
        {
            return s;
        }

        panic!("Woah, failed to read file!");
    }
}