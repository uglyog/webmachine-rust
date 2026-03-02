//! The `context` module encapsulates the context of the environment that the webmachine is
//! executing in. Basically wraps the request and response.

use std::any::Any;
use std::collections::{BTreeMap, HashMap};
use std::fmt::{Debug, Display};
use std::sync::Arc;
use std::time::SystemTime;
use bytes::Bytes;
use chrono::{DateTime, FixedOffset};
use maplit::hashmap;

use crate::headers::HeaderValue;

/// Request that the state machine is executing against
#[derive(Debug, Clone, PartialEq)]
pub struct WebmachineRequest {
  /// Path of the received request
  pub request_path: String,
  /// Resource base path (configured in the dispatcher)
  pub base_path: String,
  /// Any additional path after the base path
  pub sub_path: Option<String>,
  /// Path parts mapped to any variables (i.e. parts like /{id} will have id mapped)
  pub path_vars: HashMap<String, String>,
  /// Request method
  pub method: String,
  /// Request headers
  pub headers: HashMap<String, Vec<HeaderValue>>,
  /// Request body
  pub body: Option<Bytes>,
  /// Query parameters
  pub query: HashMap<String, Vec<String>>
}

impl Default for WebmachineRequest {
  /// Creates a default request (GET /)
  fn default() -> WebmachineRequest {
    WebmachineRequest {
      request_path: "/".to_string(),
      base_path: "/".to_string(),
      sub_path: None,
      path_vars: Default::default(),
      method: "GET".to_string(),
      headers: HashMap::new(),
      body: None,
      query: HashMap::new()
    }
  }
}

impl WebmachineRequest {
    /// returns the content type of the request, based on the content type header. Defaults to
    /// 'application/json' if there is no header.
    pub fn content_type(&self) -> HeaderValue {
      match self.headers.keys().find(|k| k.to_uppercase() == "CONTENT-TYPE") {
        Some(header) => match self.headers.get(header).unwrap().first() {
          Some(value) => value.clone(),
          None => HeaderValue::json()
        },
        None => HeaderValue::json()
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

    /// If the request is a get
    pub fn is_get(&self) -> bool {
        self.method.to_uppercase() == "GET"
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
        self.has_header("ACCEPT")
    }

    /// Returns the acceptable media types from the Accept header
    pub fn accept(&self) -> Vec<HeaderValue> {
        self.find_header("ACCEPT")
    }

    /// If an Accept-Language header exists
    pub fn has_accept_language_header(&self) -> bool {
        self.has_header("ACCEPT-LANGUAGE")
    }

    /// Returns the acceptable languages from the Accept-Language header
    pub fn accept_language(&self) -> Vec<HeaderValue> {
        self.find_header("ACCEPT-LANGUAGE")
    }

    /// If an Accept-Charset header exists
    pub fn has_accept_charset_header(&self) -> bool {
        self.has_header("ACCEPT-CHARSET")
    }

    /// Returns the acceptable charsets from the Accept-Charset header
    pub fn accept_charset(&self) -> Vec<HeaderValue> {
        self.find_header("ACCEPT-CHARSET")
    }

    /// If an Accept-Encoding header exists
    pub fn has_accept_encoding_header(&self) -> bool {
        self.has_header("ACCEPT-ENCODING")
    }

    /// Returns the acceptable encodings from the Accept-Encoding header
    pub fn accept_encoding(&self) -> Vec<HeaderValue> {
        self.find_header("ACCEPT-ENCODING")
    }

    /// If the request has the provided header
    pub fn has_header(&self, header: &str) -> bool {
      self.headers.keys().find(|k| k.to_uppercase() == header.to_uppercase()).is_some()
    }

    /// Returns the list of values for the provided request header. If the header is not present,
    /// or has no value, and empty vector is returned.
    pub fn find_header(&self, header: &str) -> Vec<HeaderValue> {
        match self.headers.keys().find(|k| k.to_uppercase() == header.to_uppercase()) {
            Some(header) => self.headers.get(header).unwrap().clone(),
            None => Vec::new()
        }
    }

    /// If the header has a matching value
    pub fn has_header_value(&self, header: &str, value: &str) -> bool {
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
    /// Response Body
    pub body: Option<Bytes>
}

impl WebmachineResponse {
    /// Creates a default response (200 OK)
    pub fn default() -> WebmachineResponse {
        WebmachineResponse {
            status: 200,
            headers: BTreeMap::new(),
            body: None
        }
    }

    /// If the response has the provided header
    pub fn has_header(&self, header: &str) -> bool {
      self.headers.keys().find(|k| k.to_uppercase() == header.to_uppercase()).is_some()
    }

    /// Adds the header values to the headers
    pub fn add_header(&mut self, header: &str, values: Vec<HeaderValue>) {
      self.headers.insert(header.to_string(), values);
    }

    /// Adds the headers from a HashMap to the headers
    pub fn add_headers(&mut self, headers: HashMap<String, Vec<String>>) {
      for (k, v) in headers {
        self.headers.insert(k, v.iter().map(HeaderValue::basic).collect());
      }
    }

    /// Adds standard CORS headers to the response
    pub fn add_cors_headers(&mut self, allowed_methods: &[&str]) {
      let cors_headers = WebmachineResponse::cors_headers(allowed_methods);
      for (k, v) in cors_headers {
        self.add_header(k.as_str(), v.iter().map(HeaderValue::basic).collect());
      }
    }

    /// Returns a HashMap of standard CORS headers
    pub fn cors_headers(allowed_methods: &[&str]) -> HashMap<String, Vec<String>> {
      hashmap!{
        "Access-Control-Allow-Origin".to_string() => vec!["*".to_string()],
        "Access-Control-Allow-Methods".to_string() => allowed_methods.iter().map(|v| v.to_string()).collect(),
        "Access-Control-Allow-Headers".to_string() => vec!["Content-Type".to_string()]
      }
    }

    /// If the response has a body
    pub fn has_body(&self) -> bool {
        match &self.body {
            &None => false,
            &Some(ref body) => !body.is_empty()
        }
    }
}

/// Trait to store arbitrary values
pub trait MetaDataThing: Any + Debug {}

/// Values that can be stored as metadata
#[derive(Debug, Clone)]
pub enum MetaDataValue {
  /// No Value,
  Empty,
  /// String Value
  String(String),
  /// Unsigned integer
  UInteger(u64),
  /// Signed integer
  Integer(i64),
  /// Boxed Any
  Anything(Arc<dyn MetaDataThing + Send + Sync>)
}

impl MetaDataValue {
  /// If the metadata value is empty
  pub fn is_empty(&self) -> bool {
    match self {
      MetaDataValue::Empty => true,
      _ => false
    }
  }

  /// If the metadata value is a String
  pub fn as_string(&self) -> Option<String> {
    match self {
      MetaDataValue::String(s) => Some(s.clone()),
      _ => None
    }
  }

  /// If the metadata value is an unsigned integer
  pub fn as_uint(&self) -> Option<u64> {
    match self {
      MetaDataValue::UInteger(u) => Some(*u),
      _ => None
    }
  }

  /// If the metadata value is a signed integer
  pub fn as_int(&self) -> Option<i64> {
    match self {
      MetaDataValue::Integer(i) => Some(*i),
      _ => None
    }
  }

  /// If the metadata value is an Anything
  pub fn as_anything(&self) -> Option<&(dyn Any + Send + Sync)> {
    match self {
      MetaDataValue::Anything(thing) => Some(thing.as_ref()),
      _ => None
    }
  }
}

impl Display for MetaDataValue {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      MetaDataValue::String(s) => write!(f, "{}", s.as_str()),
      MetaDataValue::UInteger(u) => write!(f, "{}", *u),
      MetaDataValue::Integer(i) => write!(f, "{}", *i),
      MetaDataValue::Empty => Ok(()),
      MetaDataValue::Anything(thing) => write!(f, "any({:?})", thing)
    }
  }
}

impl Default for MetaDataValue {
  fn default() -> Self {
    MetaDataValue::Empty
  }
}

impl Default for &MetaDataValue {
  fn default() -> Self {
    &MetaDataValue::Empty
  }
}

impl From<String> for MetaDataValue {
  fn from(value: String) -> Self {
    MetaDataValue::String(value)
  }
}

impl From<&String> for MetaDataValue {
  fn from(value: &String) -> Self {
    MetaDataValue::String(value.clone())
  }
}

impl From<&str> for MetaDataValue {
  fn from(value: &str) -> Self {
    MetaDataValue::String(value.to_string())
  }
}

impl From<u16> for MetaDataValue {
  fn from(value: u16) -> Self {
    MetaDataValue::UInteger(value as u64)
  }
}

impl From<i16> for MetaDataValue {
  fn from(value: i16) -> Self {
    MetaDataValue::Integer(value as i64)
  }
}

impl From<u64> for MetaDataValue {
  fn from(value: u64) -> Self {
    MetaDataValue::UInteger(value)
  }
}

impl From<i64> for MetaDataValue {
  fn from(value: i64) -> Self {
    MetaDataValue::Integer(value)
  }
}

/// Main context struct that holds the request and response.
#[derive(Debug, Clone)]
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
  pub redirect: bool,
  /// If a new resource was created
  pub new_resource: bool,
  /// General store of metadata. You can use this to store attributes as the webmachine executes.
  pub metadata: HashMap<String, MetaDataValue>,
  /// Start time instant when the context was created
  pub start_time: SystemTime
}

impl WebmachineContext {
  /// Convenience method to downcast a metadata anything value
  pub fn downcast_metadata_value<'a, T: 'static>(&'a self, key: &'a str) -> Option<&'a T> {
    self.metadata.get(key)
      .and_then(|value| value.as_anything())
      .and_then(|value| value.downcast_ref())
  }
}

impl Default for WebmachineContext {
  /// Creates a default context
  fn default() -> WebmachineContext {
    WebmachineContext {
      request: WebmachineRequest::default(),
      response: WebmachineResponse::default(),
      selected_media_type: None,
      selected_language: None,
      selected_charset: None,
      selected_encoding: None,
      if_unmodified_since: None,
      if_modified_since: None,
      redirect: false,
      new_resource: false,
      metadata: HashMap::new(),
      start_time: SystemTime::now()
    }
  }
}

#[cfg(test)]
mod tests {
  use expectest::prelude::*;

  use crate::headers::*;

  use super::*;

  #[test]
  fn request_does_not_have_header_test() {
      let request = WebmachineRequest {
          .. WebmachineRequest::default()
      };
      expect!(request.has_header("Vary")).to(be_false());
      expect!(request.has_header_value("Vary", "*")).to(be_false());
  }

  #[test]
  fn request_with_empty_header_test() {
      let request = WebmachineRequest {
          headers: hashmap!{ "HeaderA".to_string() => Vec::new() },
          .. WebmachineRequest::default()
      };
      expect!(request.has_header("HeaderA")).to(be_true());
      expect!(request.has_header_value("HeaderA", "*")).to(be_false());
  }

  #[test]
  fn request_with_header_single_value_test() {
      let request = WebmachineRequest {
          headers: hashmap!{ "HeaderA".to_string() => vec![h!("*")] },
          .. WebmachineRequest::default()
      };
      expect!(request.has_header("HeaderA")).to(be_true());
      expect!(request.has_header_value("HeaderA", "*")).to(be_true());
      expect!(request.has_header_value("HeaderA", "other")).to(be_false());
  }

  #[test]
  fn request_with_header_multiple_value_test() {
      let request = WebmachineRequest {
          headers: hashmap!{ "HeaderA".to_string() => vec![h!("*"), h!("other")]},
          .. WebmachineRequest::default()
      };
      expect!(request.has_header("HeaderA")).to(be_true());
      expect!(request.has_header_value("HeaderA", "*")).to(be_true());
      expect!(request.has_header_value("HeaderA", "other")).to(be_true());
      expect!(request.has_header_value("HeaderA", "other2")).to(be_false());
  }
}
