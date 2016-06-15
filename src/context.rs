//! The `context` module encapsulates the context of the environment that the webmachine is
//! executing in. Basically wraps the request and response.

use std::collections::HashMap;
use headers::*;

/// Request that the state machine is executing against
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

    /// If the request is an options
    pub fn is_options(&self) -> bool {
        self.method.to_uppercase() == "OPTIONS"
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
}

/// Response that is generated as a result of the webmachine execution
pub struct WebmachineResponse {
    /// status code to return
    pub status: u16,
    /// headers to return
    pub headers: HashMap<String, Vec<HeaderValue>>
}

impl WebmachineResponse {
    /// Creates a default response (200 OK)
    pub fn default() -> WebmachineResponse {
        WebmachineResponse {
            status: 200,
            headers: HashMap::new()
        }
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
pub struct WebmachineContext {
    /// Request that the webmachine is executing against
    pub request: WebmachineRequest,
    /// Response that is the result of the execution
    pub response: WebmachineResponse,
    /// selected media type after content negotiation
    pub selected_media_type: Option<String>,
    /// selected language after content negotiation
    pub selected_language: Option<String>
}

impl WebmachineContext {
    /// Creates a default context
    pub fn default() -> WebmachineContext {
        WebmachineContext {
            request: WebmachineRequest::default(),
            response: WebmachineResponse::default(),
            selected_media_type: None,
            selected_language: None
        }
    }
}
