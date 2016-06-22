//! The `context` module encapsulates the context of the environment that the webmachine is
//! executing in. Basically wraps the request and response.

use std::collections::{HashMap, BTreeMap};
use headers::*;
use chrono::*;

/// Request that the state machine is executing against
#[derive(Debug, Clone, PartialEq)]
pub struct WebmachineRequest {
    /// Path of the request relative to the resource
    pub request_path: String,
    /// Resource base path
    pub base_path: String,
    /// Request method
    pub method: String,
    /// Request headers
    pub headers: HashMap<String, Vec<HeaderValue>>
}

impl WebmachineRequest {
    /// Creates a default request (GET /)
    pub fn default() -> WebmachineRequest {
        WebmachineRequest {
            request_path: s!("/"),
            base_path: s!("/"),
            method: s!("GET"),
            headers: HashMap::new()
        }
    }

    /// returns the content type of the request, based on the content type header. Defaults to
    /// 'application/json' if there is no header.
    pub fn content_type(&self) -> String {
        match self.headers.keys().find(|k| k.to_uppercase() == "CONTENT-TYPE") {
            Some(header) => match self.headers.get(header).unwrap().first() {
                Some(value) => value.clone().value,
                None => s!("application/json")
            },
            None => s!("application/json")
        }
    }

    /// If the request is a put or post
    pub fn is_put_or_post(&self) -> bool {
        ["PUT", "POST"].contains(&self.method.to_uppercase().as_str())
    }

    /// If the request is a get or head request
    pub fn is_get_or_head(&self) -> bool {
        ["GET", "HEAD"].contains(&self.method.to_uppercase().as_str())
    }

    /// If the request is an options
    pub fn is_options(&self) -> bool {
        self.method.to_uppercase() == "OPTIONS"
    }

    /// If the request is a put
    pub fn is_put(&self) -> bool {
        self.method.to_uppercase() == "PUT"
    }

    /// If the request is a post
    pub fn is_post(&self) -> bool {
        self.method.to_uppercase() == "POST"
    }

    /// If the request is a delete
    pub fn is_delete(&self) -> bool {
        self.method.to_uppercase() == "DELETE"
    }

    /// If an Accept header exists
    pub fn has_accept_header(&self) -> bool {
        self.has_header(&s!("ACCEPT"))
    }

    /// Returns the acceptable media types from the Accept header
    pub fn accept(&self) -> Vec<HeaderValue> {
        self.find_header(&s!("ACCEPT"))
    }

    /// If an Accept-Language header exists
    pub fn has_accept_language_header(&self) -> bool {
        self.has_header(&s!("ACCEPT-LANGUAGE"))
    }

    /// Returns the acceptable languages from the Accept-Language header
    pub fn accept_language(&self) -> Vec<HeaderValue> {
        self.find_header(&s!("ACCEPT-LANGUAGE"))
    }

    /// If an Accept-Charset header exists
    pub fn has_accept_charset_header(&self) -> bool {
        self.has_header(&s!("ACCEPT-CHARSET"))
    }

    /// Returns the acceptable charsets from the Accept-Charset header
    pub fn accept_charset(&self) -> Vec<HeaderValue> {
        self.find_header(&s!("ACCEPT-CHARSET"))
    }

    /// If an Accept-Encoding header exists
    pub fn has_accept_encoding_header(&self) -> bool {
        self.has_header(&s!("ACCEPT-ENCODING"))
    }

    /// Returns the acceptable encodings from the Accept-Encoding header
    pub fn accept_encoding(&self) -> Vec<HeaderValue> {
        self.find_header(&s!("ACCEPT-ENCODING"))
    }

    /// If the request has the provided header
    pub fn has_header(&self, header: &String) -> bool {
        self.headers.keys().find(|k| k.to_uppercase() == header.to_uppercase()).is_some()
    }

    /// Returns the list of values for the provided request header. If the header is not present,
    /// or has no value, and empty vector is returned.
    pub fn find_header(&self, header: &String) -> Vec<HeaderValue> {
        match self.headers.keys().find(|k| k.to_uppercase() == header.to_uppercase()) {
            Some(header) => self.headers.get(header).unwrap().clone(),
            None => Vec::new()
        }
    }

    /// If the header has a matching value
    pub fn has_header_value(&self, header: &String, value: &String) -> bool {
        match self.headers.keys().find(|k| k.to_uppercase() == header.to_uppercase()) {
            Some(header) => match self.headers.get(header).unwrap().iter().find(|val| *val == value) {
                Some(_) => true,
                None => false
            },
            None => false
        }
    }
}

/// Response that is generated as a result of the webmachine execution
#[derive(Debug, Clone, PartialEq)]
pub struct WebmachineResponse {
    /// status code to return
    pub status: u16,
    /// headers to return
    pub headers: BTreeMap<String, Vec<HeaderValue>>,
}

impl WebmachineResponse {
    /// Creates a default response (200 OK)
    pub fn default() -> WebmachineResponse {
        WebmachineResponse {
            status: 200,
            headers: BTreeMap::new()
        }
    }

    /// If the response has the provided header
    pub fn has_header(&self, header: &String) -> bool {
        self.headers.keys().find(|k| k.to_uppercase() == header.to_uppercase()).is_some()
    }

    /// Adds the header values to the headers
    pub fn add_header(&mut self, header: String, values: Vec<HeaderValue>) {
        self.headers.insert(header, values);
    }

    /// Adds the headers from a HashMap to the headers
    pub fn add_headers(&mut self, headers: HashMap<String, Vec<String>>) {
        for (k, v) in headers {
            self.headers.insert(k, v.iter().map(HeaderValue::basic).collect());
        }
    }

    /// Adds standard CORS headers to the response
    pub fn add_cors_headers(&mut self, allowed_methods: &Vec<String>) {
        let cors_headers = WebmachineResponse::cors_headers(allowed_methods);
        for (k, v) in cors_headers {
            self.add_header(k, v.iter().map(HeaderValue::basic).collect());
        }
    }

    /// Returns a HaspMap of standard CORS headers
    pub fn cors_headers(allowed_methods: &Vec<String>) -> HashMap<String, Vec<String>> {
        hashmap!{
            s!("Access-Control-Allow-Origin") => vec![s!("*")],
            s!("Access-Control-Allow-Methods") => allowed_methods.clone(),
            s!("Access-Control-Allow-Headers") => vec![s!("Content-Type")]
        }
    }
}

/// Main context struct that holds the request and response.
#[derive(Debug, Clone, PartialEq)]
pub struct WebmachineContext {
    /// Request that the webmachine is executing against
    pub request: WebmachineRequest,
    /// Response that is the result of the execution
    pub response: WebmachineResponse,
    /// selected media type after content negotiation
    pub selected_media_type: Option<String>,
    /// selected language after content negotiation
    pub selected_language: Option<String>,
    /// selected charset after content negotiation
    pub selected_charset: Option<String>,
    /// selected encoding after content negotiation
    pub selected_encoding: Option<String>,
    /// parsed date and time from the If-Unmodified-Since header
    pub if_unmodified_since: Option<DateTime<FixedOffset>>,
    /// parsed date and time from the If-Modified-Since header
    pub if_modified_since: Option<DateTime<FixedOffset>>,
    /// If the response should be a redirect
    pub redirect: bool
}

impl WebmachineContext {
    /// Creates a default context
    pub fn default() -> WebmachineContext {
        WebmachineContext {
            request: WebmachineRequest::default(),
            response: WebmachineResponse::default(),
            selected_media_type: None,
            selected_language: None,
            selected_charset: None,
            selected_encoding: None,
            if_unmodified_since: None,
            if_modified_since: None,
            redirect: false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use headers::*;
    use expectest::prelude::*;

    #[test]
    fn request_does_not_have_header_test() {
        let request = WebmachineRequest {
            .. WebmachineRequest::default()
        };
        expect!(request.has_header(&s!("Vary"))).to(be_false());
        expect!(request.has_header_value(&s!("Vary"), &s!("*"))).to(be_false());
    }

    #[test]
    fn request_with_empty_header_test() {
        let request = WebmachineRequest {
            headers: hashmap!{ s!("HeaderA") => Vec::new() },
            .. WebmachineRequest::default()
        };
        expect!(request.has_header(&s!("HeaderA"))).to(be_true());
        expect!(request.has_header_value(&s!("HeaderA"), &s!("*"))).to(be_false());
    }

    #[test]
    fn request_with_header_single_value_test() {
        let request = WebmachineRequest {
            headers: hashmap!{ s!("HeaderA") => vec![h!("*")] },
            .. WebmachineRequest::default()
        };
        expect!(request.has_header(&s!("HeaderA"))).to(be_true());
        expect!(request.has_header_value(&s!("HeaderA"), &s!("*"))).to(be_true());
        expect!(request.has_header_value(&s!("HeaderA"), &s!("other"))).to(be_false());
    }

    #[test]
    fn request_with_header_multiple_value_test() {
        let request = WebmachineRequest {
            headers: hashmap!{ s!("HeaderA") => vec![h!("*"), h!("other")]},
            .. WebmachineRequest::default()
        };
        expect!(request.has_header(&s!("HeaderA"))).to(be_true());
        expect!(request.has_header_value(&s!("HeaderA"), &s!("*"))).to(be_true());
        expect!(request.has_header_value(&s!("HeaderA"), &s!("other"))).to(be_true());
        expect!(request.has_header_value(&s!("HeaderA"), &s!("other2"))).to(be_false());
    }

}
