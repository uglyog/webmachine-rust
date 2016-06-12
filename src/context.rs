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

    /// Adds the headers values to the headers
    pub fn add_header(&mut self, header: String, values: Vec<HeaderValue>) {
        self.headers.insert(header, values);
    }
}

/// Main context struct that holds the request and response.
pub struct WebmachineContext {
    /// Request that the webmachine is executing against
    pub request: WebmachineRequest,
    /// Response that is the result of the execution
    pub response: WebmachineResponse
}

impl WebmachineContext {
    /// Creates a default context
    pub fn default() -> WebmachineContext {
        WebmachineContext {
            request: WebmachineRequest::default(),
            response: WebmachineResponse::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use expectest::prelude::*;


}
