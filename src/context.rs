//! The `context` module encapsulates the context of the environment that the webmachine is
//! executing in. Basically wraps the request and response.

/// Request that the state machine is executing against
pub struct WebmachineRequest {
    /// Path of the request relative to the resource
    pub request_path: String,
    /// Resource base path
    pub base_path: String
}

impl WebmachineRequest {
    /// Creates a default request (GET /)
    pub fn default() -> WebmachineRequest {
        WebmachineRequest {
            request_path: s!("/"),
            base_path: s!("/")
        }
    }
}

/// Response that is generated as a result of the webmachine execution
pub struct WebmachineResponse {
    /// status code to return
    pub status: u16
}

impl WebmachineResponse {
    /// Creates a default response (200 OK)
    pub fn default() -> WebmachineResponse {
        WebmachineResponse {
            status: 200
        }
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
