use linked_hash_map::LinkedHashMap;

use crate::web::{
    Resolution,
    resolution::{empty_content, get_status_header},
};

pub type Location = &'static str;

/// Redirect Types
///
/// Redirect types that you can use to set the header of your redirect.\
#[repr(i32)] //tells the enum to align with i32
pub enum RedirectType {
    /// The requested URL has more than one possible responses available.
    ///
    /// See: https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Status/300
    ///
    /// # NOT YET IMPLEMENTED
    MultipleChoices, //TODO: This needs a specific struct to specify other locations. This may not even be implemented since it is 'rare' according to mozilla.

    /// The resource has been moved permanently.
    ///
    /// See: https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Status/301
    MovedPermanently(Location),

    /// The requested URL has been moved temporarliy.
    ///
    /// See: https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Status/302
    Found(Location),

    /// Indicates that the browser should redirect to the url in the location header.
    ///
    /// See: https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Status/303
    SeeOther(Location),

    /// Indicates to the browser that the page has not been modified since a requested date.
    ///
    /// See: https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Status/304
    ///
    /// Useful for caching items.
    ///
    /// You should use the header `If-Modified-Since` and check before sending this header.
    NotModified,

    /// The requested URL has been temporarliy moved
    ///
    /// See: https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Status/307
    TemporaryRedirect(Location),

    /// The requested URL has been permanently moved.
    ///
    /// See: https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Status/308
    PermanentRedirect(Location),
}

impl RedirectType {
    /// the status of the redirection type 300, etc...
    fn status(&self) -> i32 {
        match self {
            RedirectType::MultipleChoices => 300,
            RedirectType::MovedPermanently(_) => 301,
            RedirectType::Found(_) => 302,
            RedirectType::SeeOther(_) => 303,
            RedirectType::NotModified => 304,
            RedirectType::TemporaryRedirect(_) => 307,
            RedirectType::PermanentRedirect(_) => 308,
        }
    }

    //returns the amount of headers that will be included
    //this is used for optimization to create a sized vector.
    fn size(&self) -> usize {
        match self {
            RedirectType::NotModified => 0, //this is more for caching, just letting the browser something has not modified since XYZ

            //TODO implement the multiple choices.
            RedirectType::MultipleChoices => todo!(),

            //the rest of the current implement the Location: header.
            _ => 1,
        }
    }
}

pub struct Redirect {
    redirect_header_type: RedirectType,
}

impl Redirect {
    /// Create a new redirect resolution with a redirect type.
    pub fn new(redirect_type: RedirectType) -> Self {
        Self {
            redirect_header_type: redirect_type,
        }
    }
}

//formats the url into a Location: Url header.
fn location_header(url: Location) -> (String, String) {
    ("Location".to_string(), url.to_string())
}

impl Resolution for Redirect {
    //sets the header for the redirection!
    fn get_headers(&self) -> LinkedHashMap<String, Option<String>> {
        let mut hmap = LinkedHashMap::<String, Option<String>>::with_capacity(
            1 + self.redirect_header_type.size(),
        );

        let (n, v) = get_status_header(self.redirect_header_type.status());
        hmap.insert(n, Some(v));

        //subject to change
        let redir_headers: Option<(String, String)> = match self.redirect_header_type {
            //just use the location header.
            RedirectType::MovedPermanently(url) => Some(location_header(url)),
            RedirectType::Found(url) => Some(location_header(url)),
            RedirectType::SeeOther(url) => Some(location_header(url).into()),
            RedirectType::PermanentRedirect(url) => Some(location_header(url)),
            RedirectType::TemporaryRedirect(url) => Some(location_header(url)),

            //TODO: Implement the multiple choices.
            RedirectType::MultipleChoices => todo!(),
            RedirectType::NotModified => None,
        };

        //push the redirection header.
        if let Some((n, v)) = redir_headers {
            hmap.insert(n, Some(v));
        }

        hmap
    }

    fn get_content(&self) -> std::pin::Pin<Box<dyn futures::Stream<Item = Vec<u8>> + Send>> {
        Box::pin(tokio_stream::once(empty_content()))
    }

    fn resolve(self) -> Box<dyn Resolution + Send + 'static> {
        Box::new(self)
    }
}
