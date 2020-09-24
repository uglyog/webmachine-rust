/*!
# webmachine-rust

Port of Webmachine-Ruby (https://github.com/webmachine/webmachine-ruby) to Rust.

webmachine-rust is a port of the Ruby version of webmachine. It implements a finite state machine for the HTTP protocol
that provides semantic HTTP handling (based on the [diagram from the webmachine project](https://webmachine.github.io/images/http-headers-status-v3.png)).
It is basically a HTTP toolkit for building HTTP-friendly applications using the [Hyper](https://crates.io/crates/hyper) rust crate.

Webmachine-rust works with Hyper and sits between the Hyper Handler and your application code. It provides a resource struct
with callbacks to handle the decisions required as the state machine is executed against the request with the following sequence.

REQUEST -> Hyper Handler -> WebmachineDispatcher -> WebmachineResource -> Your application -> WebmachineResponse -> Hyper -> RESPONSE

## Features

- Handles the hard parts of content negotiation, conditional requests, and response codes for you.
- Provides a resource struct with points of extension to let you describe what is relevant about your particular resource.

## Missing Features

Currently, the following features from webmachine-ruby have not been implemented:

- Visual debugger
- Streaming response bodies

## Implementation Deficiencies:

This implementation has the following deficiencies:

- Only supports Hyper
- WebmachineDispatcher and WebmachineResource are not shareable between threads.
- Automatically decoding request bodies and encoding response bodies.
- No easy mechanism to generate bodies with different content types (e.g. JSON vs. XML).
- No easy mechanism for handling sub-paths in a resource.
- Does not work with keep alive enabled (does not manage the Hyper thread pool).
- Dynamically determining the methods allowed on the resource.
- Compiled against Hyper with all features turned off (no HTTPS).

## Getting started

Follow the getting started documentation from the Hyper crate to setup a Hyper Handler for your server. Then from the
handle function, you need to define a WebmachineDispatcher that maps resource paths to your webmachine resources (WebmachineResource). Each WebmachineResource defines all the callbacks (via Closures) and values required to implement a
resource.

Note: This example uses the maplit crate to provide the `btreemap` macro and the log crate for the logging macros.

```no_run
# #[macro_use] extern crate log;
# #[macro_use] extern crate maplit;
# extern crate hyper;
# extern crate webmachine_rust;
# extern crate serde_json;
 use std::sync::Arc;
 use hyper::server::{Handler, Server, Request, Response};
 use webmachine_rust::*;
 use webmachine_rust::context::*;
 use webmachine_rust::headers::*;
 use serde_json::{Value, json};
 use std::io::Read;

 # fn main() { }

 fn from_hyper(req: &mut Request) -> http::Request<Vec<u8>> {
  let mut request = http::Request::builder()
    .uri(req.uri.to_string())
    .method(req.method.as_ref());
  for header in req.headers.iter() {
    request = request.header(header.name(), header.value_string());
  }
  let mut buffer = Vec::new();
  req.read_to_end(&mut buffer);
  request.body(buffer.clone()).unwrap()
 }

 struct ServerHandler {
 }

 impl Handler for ServerHandler {

     fn handle(&self, mut req: Request, res: Response) {
         // setup the dispatcher, which maps paths to resources
         let dispatcher = WebmachineDispatcher::new(
             btreemap!{
                 "/myresource".to_string() => Arc::new(WebmachineResource {
                     // Methods allowed on this resource
                     allowed_methods: vec!["OPTIONS".to_string(), "GET".to_string(),
                        "HEAD".to_string(), "POST".to_string()],
                     // if the resource exists callback
                     resource_exists: Box::new(|context| true),
                     // callback to render the response for the resource
                     render_response: Box::new(|_| {
                         let json_response = json!({
                            "data": [1, 2, 3, 4]
                         });
                         Some(json_response.to_string())
                     }),
                     // callback to process the post for the resource
                     process_post: Box::new(|context|  /* Handle the post here */ Ok(true) ),
                     // default everything else
                     .. WebmachineResource::default()
                 })
             }
         );
         // then dispatch the request to the web machine.
         match dispatcher.dispatch(from_hyper(&mut req)) {
             Ok(res) => (),
             Err(err) => warn!("Error generating response - {}", err)
         };
     }
 }

 pub fn start_server() {
     match Server::http(format!("0.0.0.0:0").as_str()) {
         Ok(mut server) => {
             // It is important to turn keep alive off
             server.keep_alive(None);
             server.handle(ServerHandler {});
         },
         Err(err) => {
             error!("could not start server: {}", err);
         }
     }
 }
```

## Example implementations

For an example of a project using this crate, have a look at the [Pact Mock Server](https://github.com/pact-foundation/pact-reference/tree/master/rust/v1/pact_mock_server_cli) from the Pact reference implementation.
*/

#![warn(missing_docs)]

#[macro_use] extern crate log;
#[macro_use] extern crate maplit;
extern crate itertools;
#[macro_use] extern crate lazy_static;
extern crate chrono;
extern crate http;

use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;
use std::sync::Arc;
use itertools::Itertools;
use chrono::{DateTime, FixedOffset, Utc};
use context::{WebmachineContext, WebmachineResponse, WebmachineRequest};
use headers::HeaderValue;
use http::{Request, Response};

/// Simple macro to convert a string slice to a `String` struct.
macro_rules! s {
    ($e:expr) => ($e.to_string())
}

#[macro_use] pub mod headers;
pub mod context;
pub mod content_negotiation;

/// Struct to represent a resource in webmachine
pub struct WebmachineResource {
  /// This is called just before the final response is constructed and sent. It allows the resource
  /// an opportunity to modify the response after the webmachine has executed.
  pub finalise_response: Option<Box<dyn Fn(&mut WebmachineContext)>>,
  /// This is invoked to render the response for the resource
  pub render_response: Box<dyn Fn(&mut WebmachineContext) -> Option<String>>,
  /// Is the resource available? Returning false will result in a '503 Service Not Available'
  /// response. Defaults to true. If the resource is only temporarily not available,
  /// add a 'Retry-After' response header.
  pub available: Box<dyn Fn(&mut WebmachineContext) -> bool>,
  /// HTTP methods that are known to the resource. Default includes all standard HTTP methods.
  /// One could override this to allow additional methods
  pub known_methods: Vec<String>,
  /// If the URI is too long to be processed, this should return true, which will result in a
  // '414 Request URI Too Long' response. Defaults to false.
  pub uri_too_long: Box<dyn Fn(&mut WebmachineContext) -> bool>,
  /// HTTP methods that are allowed on this resource. Defaults to GET','HEAD and 'OPTIONS'.
  pub allowed_methods: Vec<String>,
  /// If the request is malformed, this should return true, which will result in a
  /// '400 Malformed Request' response. Defaults to false.
  pub malformed_request: Box<dyn Fn(&mut WebmachineContext) -> bool>,
  /// Is the client or request not authorized? Returning a Some<String>
  /// will result in a '401 Unauthorized' response.  Defaults to None. If a Some(String) is
  /// returned, the string will be used as the value in the WWW-Authenticate header.
  pub not_authorized: Box<dyn Fn(&mut WebmachineContext) -> Option<String>>,
  /// Is the request or client forbidden? Returning true will result in a '403 Forbidden' response.
  /// Defaults to false.
  pub forbidden: Box<dyn Fn(&mut WebmachineContext) -> bool>,
  /// If the request includes any invalid Content-* headers, this should return true, which will
  /// result in a '501 Not Implemented' response. Defaults to false.
  pub unsupported_content_headers: Box<dyn Fn(&mut WebmachineContext) -> bool>,
  /// The list of acceptable content types. Defaults to 'application/json'. If the content type
  /// of the request is not in this list, a '415 Unsupported Media Type' response is returned.
  pub acceptable_content_types: Vec<String>,
  /// If the entity length on PUT or POST is invalid, this should return false, which will result
  /// in a '413 Request Entity Too Large' response. Defaults to true.
  pub valid_entity_length: Box<dyn Fn(&mut WebmachineContext) -> bool>,
  /// This is called just before the final response is constructed and sent. This allows the
  /// response to be modified. The default implementation adds CORS headers to the response
  pub finish_request: Box<dyn Fn(&mut WebmachineContext, &WebmachineResource)>,
  /// If the OPTIONS method is supported and is used, this return a HashMap of headers that
  // should appear in the response. Defaults to CORS headers.
  pub options: Box<dyn Fn(&mut WebmachineContext, &WebmachineResource) -> Option<HashMap<String, Vec<String>>>>,
  /// The list of content types that this resource produces. Defaults to 'application/json'. If
  /// more than one is provided, and the client does not supply an Accept header, the first one
  /// will be selected.
  pub produces: Vec<String>,
  /// The list of content languages that this resource provides. Defaults to an empty list,
  /// which represents all languages. If more than one is provided, and the client does not
  /// supply an Accept-Language header, the first one will be selected.
  pub languages_provided: Vec<String>,
  /// The list of charsets that this resource provides. Defaults to an empty list,
  /// which represents all charsets with ISO-8859-1 as the default. If more than one is provided,
  /// and the client does not supply an Accept-Charset header, the first one will be selected.
  pub charsets_provided: Vec<String>,
  /// The list of encodings your resource wants to provide. The encoding will be applied to the
  /// response body automatically by Webmachine. Default includes only the 'identity' encoding.
  pub encodings_provided: Vec<String>,
  /// The list of header names that should be included in the response's Vary header. The standard
  /// content negotiation headers (Accept, Accept-Encoding, Accept-Charset, Accept-Language) do
  /// not need to be specified here as Webmachine will add the correct elements of those
  /// automatically depending on resource behavior. Default is an empty list.
  pub variances: Vec<String>,
  /// Does the resource exist? Returning a false value will result in a '404 Not Found' response
  /// unless it is a PUT or POST. Defaults to true.
  pub resource_exists: Box<dyn Fn(&mut WebmachineContext) -> bool>,
  /// If this resource is known to have existed previously, this should return true. Default is false.
  pub previously_existed: Box<dyn Fn(&mut WebmachineContext) -> bool>,
  /// If this resource has moved to a new location permanently, this should return the new
  /// location as a String. Default is to return None
  pub moved_permanently: Box<dyn Fn(&mut WebmachineContext) -> Option<String>>,
  /// If this resource has moved to a new location temporarily, this should return the new
  /// location as a String. Default is to return None
  pub moved_temporarily: Box<dyn Fn(&mut WebmachineContext) -> Option<String>>,
  /// If this returns true, the client will receive a '409 Conflict' response. This is only
  /// called for PUT requests. Default is false.
  pub is_conflict: Box<dyn Fn(&mut WebmachineContext) -> bool>,
  /// Return true if the resource accepts POST requests to nonexistent resources. Defaults to false.
  pub allow_missing_post: Box<dyn Fn(&mut WebmachineContext) -> bool>,
  /// If this returns a value, it will be used as the value of the ETag header and for
  /// comparison in conditional requests. Default is None.
  pub generate_etag: Box<dyn Fn(&mut WebmachineContext) -> Option<String>>,
  /// Returns the last modified date and time of the resource which will be added as the
  /// Last-Modified header in the response and used in negotiating conditional requests.
  /// Default is None
  pub last_modified: Box<dyn Fn(&mut WebmachineContext) -> Option<DateTime<FixedOffset>>>,
  /// Called when a DELETE request should be enacted. Return `Ok(true)` if the deletion succeeded,
  /// and `Ok(false)` if the deletion was accepted but cannot yet be guaranteed to have finished.
  /// If the delete fails for any reason, return an Err with the status code you wish returned
  /// (a 500 status makes sense).
  /// Defaults to `Ok(true)`.
  pub delete_resource: Box<dyn Fn(&mut WebmachineContext) -> Result<bool, u16>>,
  /// If POST requests should be treated as a request to put content into a (potentially new)
  /// resource as opposed to a generic submission for processing, then this should return true.
  /// If it does return true, then `create_path` will be called and the rest of the request will
  /// be treated much like a PUT to the path returned by that call. Default is false.
  pub post_is_create: Box<dyn Fn(&mut WebmachineContext) -> bool>,
  /// If `post_is_create` returns false, then this will be called to process any POST request.
  /// If it succeeds, return `Ok(true)`, `Ok(false)` otherwise. If it fails for any reason,
  /// return an Err with the status code you wish returned (e.g., a 500 status makes sense).
  /// Default is false. If you want the result of processing the POST to be a redirect, set
  /// `context.redirect` to true.
  pub process_post: Box<dyn Fn(&mut WebmachineContext) -> Result<bool, u16>>,
  /// This will be called on a POST request if `post_is_create` returns true. It should create
  /// the new resource and return the path as a valid URI part following the dispatcher prefix.
  /// That path will replace the previous one in the return value of `WebmachineRequest.request_path`
  /// for all subsequent resource function calls in the course of this request and will be set
  /// as the value of the Location header of the response. If it fails for any reason,
  /// return an Err with the status code you wish returned (e.g., a 500 status makes sense).
  /// Default will return an `Ok(WebmachineRequest.request_path)`. If you want the result of
  /// processing the POST to be a redirect, set `context.redirect` to true.
  pub create_path: Box<dyn Fn(&mut WebmachineContext) -> Result<String, u16>>,
  /// This will be called to process any PUT request. If it succeeds, return `Ok(true)`,
  /// `Ok(false)` otherwise. If it fails for any reason, return an Err with the status code
  /// you wish returned (e.g., a 500 status makes sense). Default is `Ok(true)`
  pub process_put: Box<dyn Fn(&mut WebmachineContext) -> Result<bool, u16>>,
  /// If this returns true, then it is assumed that multiple representations of the response are
  /// possible and a single one cannot be automatically chosen, so a 300 Multiple Choices will
  /// be sent instead of a 200. Default is false.
  pub multiple_choices: Box<dyn Fn(&mut WebmachineContext) -> bool>,
  /// If the resource expires, this should return the date/time it expires. Default is None.
  pub expires: Box<dyn Fn(&mut WebmachineContext) -> Option<DateTime<FixedOffset>>>
}

impl Default for WebmachineResource {
  /// Creates a default webmachine resource
  fn default() -> WebmachineResource {
    WebmachineResource {
      finalise_response: None,
      available: Box::new(|_| true),
      known_methods: vec![s!("OPTIONS"), s!("GET"), s!("POST"), s!("PUT"), s!("DELETE"),
                          s!("HEAD"), s!("TRACE"), s!("CONNECT"), s!("PATCH")],
      uri_too_long: Box::new(|_| false),
      allowed_methods: vec![s!("OPTIONS"), s!("GET"), s!("HEAD")],
      malformed_request: Box::new(|_| false),
      not_authorized: Box::new(|_| None),
      forbidden: Box::new(|_| false),
      unsupported_content_headers: Box::new(|_| false),
      acceptable_content_types: vec![s!("application/json")],
      valid_entity_length: Box::new(|_| true),
      finish_request: Box::new(|context, resource| context.response.add_cors_headers(&resource.allowed_methods)),
      options: Box::new(|_, resource| Some(WebmachineResponse::cors_headers(&resource.allowed_methods))),
      produces: vec![s!("application/json")],
      languages_provided: Vec::new(),
      charsets_provided: Vec::new(),
      encodings_provided: vec![s!("identity")],
      variances: Vec::new(),
      resource_exists: Box::new(|_| true),
      previously_existed: Box::new(|_| false),
      moved_permanently: Box::new(|_| None),
      moved_temporarily: Box::new(|_| None),
      is_conflict: Box::new(|_| false),
      allow_missing_post: Box::new(|_| false),
      generate_etag: Box::new(|_| None),
      last_modified: Box::new(|_| None),
      delete_resource: Box::new(|_| Ok(true)),
      post_is_create: Box::new(|_| false),
      process_post: Box::new(|_| Ok(false)),
      process_put: Box::new(|_| Ok(true)),
      multiple_choices: Box::new(|_| false),
      create_path: Box::new(|context| Ok(context.request.request_path.clone())),
      expires: Box::new(|_| None),
      render_response: Box::new(|_| None)
    }
  }
}

fn sanitise_path(path: &String) -> Vec<String> {
    path.split("/").filter(|p| !p.is_empty()).map(|p| p.to_string()).collect()
}

fn join_paths(base: &Vec<String>, path: &Vec<String>) -> String {
    let mut paths = base.clone();
    paths.extend_from_slice(path);
    let filtered: Vec<String> = paths.iter().cloned().filter(|p| !p.is_empty()).collect();
    if filtered.is_empty() {
        s!("/")
    } else {
        let new_path = filtered.iter().join("/");
        if new_path.starts_with("/") {
            new_path
        } else {
            s!("/") + &new_path
        }
    }
}

const MAX_STATE_MACHINE_TRANSITIONS: u8 = 100;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Decision {
    Start,
    End(u16),
    A3Options,
    B3Options,
    B4RequestEntityTooLarge,
    B5UnkownContentType,
    B6UnsupportedContentHeader,
    B7Forbidden,
    B8Authorized,
    B9MalformedRequest,
    B10MethodAllowed,
    B11UriTooLong,
    B12KnownMethod,
    B13Available,
    C3AcceptExists,
    C4AcceptableMediaTypeAvailable,
    D4AcceptLanguageExists,
    D5AcceptableLanguageAvailable,
    E5AcceptCharsetExists,
    E6AcceptableCharsetAvailable,
    F6AcceptEncodingExists,
    F7AcceptableEncodingAvailable,
    G7ResourceExists,
    G8IfMatchExists,
    G9IfMatchStarExists,
    G11EtagInIfMatch,
    H7IfMatchStarExists,
    H10IfUnmodifiedSinceExists,
    H11IfUnmodifiedSinceValid,
    H12LastModifiedGreaterThanUMS,
    I4HasMovedPermanently,
    I12IfNoneMatchExists,
    I13IfNoneMatchStarExists,
    I7Put,
    J18GetHead,
    K5HasMovedPermanently,
    K7ResourcePreviouslyExisted,
    K13ETagInIfNoneMatch,
    L5HasMovedTemporarily,
    L7Post,
    L13IfModifiedSinceExists,
    L14IfModifiedSinceValid,
    L15IfModifiedSinceGreaterThanNow,
    L17IfLastModifiedGreaterThanMS,
    M5Post,
    M7PostToMissingResource,
    M16Delete,
    M20DeleteEnacted,
    N5PostToMissingResource,
    N11Redirect,
    N16Post,
    O14Conflict,
    O16Put,
    O18MultipleRepresentations,
    O20ResponseHasBody,
    P3Conflict,
    P11NewResource
}

impl Decision {
    fn is_terminal(&self) -> bool {
        match self {
            &Decision::End(_) => true,
            &Decision::A3Options => true,
            _ => false
        }
    }
}

enum Transition {
    To(Decision),
    Branch(Decision, Decision)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum DecisionResult {
    True,
    False,
    StatusCode(u16)
}

impl DecisionResult {
    fn wrap(result: bool) -> DecisionResult {
        if result {
            DecisionResult::True
        } else {
            DecisionResult::False
        }
    }
}

lazy_static! {
    static ref TRANSITION_MAP: HashMap<Decision, Transition> = hashmap!{
        Decision::Start => Transition::To(Decision::B13Available),
        Decision::B3Options => Transition::Branch(Decision::A3Options, Decision::C3AcceptExists),
        Decision::B4RequestEntityTooLarge => Transition::Branch(Decision::End(413), Decision::B3Options),
        Decision::B5UnkownContentType => Transition::Branch(Decision::End(415), Decision::B4RequestEntityTooLarge),
        Decision::B6UnsupportedContentHeader => Transition::Branch(Decision::End(501), Decision::B5UnkownContentType),
        Decision::B7Forbidden => Transition::Branch(Decision::End(403), Decision::B6UnsupportedContentHeader),
        Decision::B8Authorized => Transition::Branch(Decision::B7Forbidden, Decision::End(401)),
        Decision::B9MalformedRequest => Transition::Branch(Decision::End(400), Decision::B8Authorized),
        Decision::B10MethodAllowed => Transition::Branch(Decision::B9MalformedRequest, Decision::End(405)),
        Decision::B11UriTooLong => Transition::Branch(Decision::End(414), Decision::B10MethodAllowed),
        Decision::B12KnownMethod => Transition::Branch(Decision::B11UriTooLong, Decision::End(501)),
        Decision::B13Available => Transition::Branch(Decision::B12KnownMethod, Decision::End(503)),
        Decision::C3AcceptExists => Transition::Branch(Decision::C4AcceptableMediaTypeAvailable, Decision::D4AcceptLanguageExists),
        Decision::C4AcceptableMediaTypeAvailable => Transition::Branch(Decision::D4AcceptLanguageExists, Decision::End(406)),
        Decision::D4AcceptLanguageExists => Transition::Branch(Decision::D5AcceptableLanguageAvailable, Decision::E5AcceptCharsetExists),
        Decision::D5AcceptableLanguageAvailable => Transition::Branch(Decision::E5AcceptCharsetExists, Decision::End(406)),
        Decision::E5AcceptCharsetExists => Transition::Branch(Decision::E6AcceptableCharsetAvailable, Decision::F6AcceptEncodingExists),
        Decision::E6AcceptableCharsetAvailable => Transition::Branch(Decision::F6AcceptEncodingExists, Decision::End(406)),
        Decision::F6AcceptEncodingExists => Transition::Branch(Decision::F7AcceptableEncodingAvailable, Decision::G7ResourceExists),
        Decision::F7AcceptableEncodingAvailable => Transition::Branch(Decision::G7ResourceExists, Decision::End(406)),
        Decision::G7ResourceExists => Transition::Branch(Decision::G8IfMatchExists, Decision::H7IfMatchStarExists),
        Decision::G8IfMatchExists => Transition::Branch(Decision::G9IfMatchStarExists, Decision::H10IfUnmodifiedSinceExists),
        Decision::G9IfMatchStarExists => Transition::Branch(Decision::H10IfUnmodifiedSinceExists, Decision::G11EtagInIfMatch),
        Decision::G11EtagInIfMatch => Transition::Branch(Decision::H10IfUnmodifiedSinceExists, Decision::End(412)),
        Decision::H7IfMatchStarExists => Transition::Branch(Decision::End(412), Decision::I7Put),
        Decision::H10IfUnmodifiedSinceExists => Transition::Branch(Decision::H11IfUnmodifiedSinceValid, Decision::I12IfNoneMatchExists),
        Decision::H11IfUnmodifiedSinceValid => Transition::Branch(Decision::H12LastModifiedGreaterThanUMS, Decision::I12IfNoneMatchExists),
        Decision::H12LastModifiedGreaterThanUMS => Transition::Branch(Decision::End(412), Decision::I12IfNoneMatchExists),
        Decision::I4HasMovedPermanently => Transition::Branch(Decision::End(301), Decision::P3Conflict),
        Decision::I7Put => Transition::Branch(Decision::I4HasMovedPermanently, Decision::K7ResourcePreviouslyExisted),
        Decision::I12IfNoneMatchExists => Transition::Branch(Decision::I13IfNoneMatchStarExists, Decision::L13IfModifiedSinceExists),
        Decision::I13IfNoneMatchStarExists => Transition::Branch(Decision::J18GetHead, Decision::K13ETagInIfNoneMatch),
        Decision::J18GetHead => Transition::Branch(Decision::End(304), Decision::End(412)),
        Decision::K13ETagInIfNoneMatch => Transition::Branch(Decision::J18GetHead, Decision::L13IfModifiedSinceExists),
        Decision::K5HasMovedPermanently => Transition::Branch(Decision::End(301), Decision::L5HasMovedTemporarily),
        Decision::K7ResourcePreviouslyExisted => Transition::Branch(Decision::K5HasMovedPermanently, Decision::L7Post),
        Decision::L5HasMovedTemporarily => Transition::Branch(Decision::End(307), Decision::M5Post),
        Decision::L7Post => Transition::Branch(Decision::M7PostToMissingResource, Decision::End(404)),
        Decision::L13IfModifiedSinceExists => Transition::Branch(Decision::L14IfModifiedSinceValid, Decision::M16Delete),
        Decision::L14IfModifiedSinceValid => Transition::Branch(Decision::L15IfModifiedSinceGreaterThanNow, Decision::M16Delete),
        Decision::L15IfModifiedSinceGreaterThanNow => Transition::Branch(Decision::M16Delete, Decision::L17IfLastModifiedGreaterThanMS),
        Decision::L17IfLastModifiedGreaterThanMS => Transition::Branch(Decision::M16Delete, Decision::End(304)),
        Decision::M5Post => Transition::Branch(Decision::N5PostToMissingResource, Decision::End(410)),
        Decision::M7PostToMissingResource => Transition::Branch(Decision::N11Redirect, Decision::End(404)),
        Decision::M16Delete => Transition::Branch(Decision::M20DeleteEnacted, Decision::N16Post),
        Decision::M20DeleteEnacted => Transition::Branch(Decision::O20ResponseHasBody, Decision::End(202)),
        Decision::N5PostToMissingResource => Transition::Branch(Decision::N11Redirect, Decision::End(410)),
        Decision::N11Redirect => Transition::Branch(Decision::End(303), Decision::P11NewResource),
        Decision::N16Post => Transition::Branch(Decision::N11Redirect, Decision::O16Put),
        Decision::O14Conflict => Transition::Branch(Decision::End(409), Decision::P11NewResource),
        Decision::O16Put => Transition::Branch(Decision::O14Conflict, Decision::O18MultipleRepresentations),
        Decision::P3Conflict => Transition::Branch(Decision::End(409), Decision::P11NewResource),
        Decision::P11NewResource => Transition::Branch(Decision::End(201), Decision::O20ResponseHasBody),
        Decision::O18MultipleRepresentations => Transition::Branch(Decision::End(300), Decision::End(200)),
        Decision::O20ResponseHasBody => Transition::Branch(Decision::O18MultipleRepresentations, Decision::End(204))
    };
}

fn resource_etag_matches_header_values(resource: &WebmachineResource, context: &mut WebmachineContext,
    header: &str) -> bool {
    let header_values = context.request.find_header(&s!(header));
    match resource.generate_etag.as_ref()(context) {
        Some(etag) => {
            header_values.iter().find(|val| {
                if val.value.starts_with("W/") {
                    val.weak_etag().unwrap() == etag
                } else {
                    val.value == etag
                }
            }).is_some()
        },
        None => false
    }
}

fn validate_header_date(request: &WebmachineRequest, header: &str,
    context_meta: &mut Option<DateTime<FixedOffset>>) -> bool {
    let header_values = request.find_header(&s!(header));
    let date_value = header_values.first().unwrap().clone().value;
    match DateTime::parse_from_rfc2822(&date_value) {
        Ok(datetime) => {
            *context_meta = Some(datetime.clone());
            true
        },
        Err(err) => {
            debug!("Failed to parse '{}' header value '{}' - {}", header, date_value, err);
            false
        }
    }
}

fn execute_decision(decision: &Decision, context: &mut WebmachineContext, resource: &WebmachineResource) -> DecisionResult {
    match decision {
        &Decision::B10MethodAllowed => {
            match resource.allowed_methods
                .iter().find(|m| m.to_uppercase() == context.request.method.to_uppercase()) {
                Some(_) => DecisionResult::True,
                None => {
                    context.response.add_header(s!("Allow"), resource.allowed_methods
                        .iter()
                        .map(HeaderValue::basic)
                        .collect());
                    DecisionResult::False
                }
            }
        },
        &Decision::B11UriTooLong => DecisionResult::wrap(resource.uri_too_long.as_ref()(context)),
        &Decision::B12KnownMethod => DecisionResult::wrap(resource.known_methods
            .iter().find(|m| m.to_uppercase() == context.request.method.to_uppercase()).is_some()),
        &Decision::B13Available => DecisionResult::wrap(resource.available.as_ref()(context)),
        &Decision::B9MalformedRequest => DecisionResult::wrap(resource.malformed_request.as_ref()(context)),
        &Decision::B8Authorized => match resource.not_authorized.as_ref()(context) {
            Some(realm) => {
                context.response.add_header(s!("WWW-Authenticate"), vec![HeaderValue::parse_string(realm.clone())]);
                DecisionResult::False
            },
            None => DecisionResult::True
        },
        &Decision::B7Forbidden => DecisionResult::wrap(resource.forbidden.as_ref()(context)),
        &Decision::B6UnsupportedContentHeader => DecisionResult::wrap(resource.unsupported_content_headers.as_ref()(context)),
        &Decision::B5UnkownContentType => DecisionResult::wrap(context.request.is_put_or_post() && resource.acceptable_content_types
                .iter().find(|ct| context.request.content_type().to_uppercase() == ct.to_uppercase() )
                .is_none()),
        &Decision::B4RequestEntityTooLarge => DecisionResult::wrap(context.request.is_put_or_post()
            && !resource.valid_entity_length.as_ref()(context)),
        &Decision::B3Options => DecisionResult::wrap(context.request.is_options()),
        &Decision::C3AcceptExists => DecisionResult::wrap(context.request.has_accept_header()),
        &Decision::C4AcceptableMediaTypeAvailable => match content_negotiation::matching_content_type(resource, &context.request) {
            Some(media_type) => {
                context.selected_media_type = Some(media_type);
                DecisionResult::True
            },
            None => DecisionResult::False
        },
        &Decision::D4AcceptLanguageExists => DecisionResult::wrap(context.request.has_accept_language_header()),
        &Decision::D5AcceptableLanguageAvailable => match content_negotiation::matching_language(resource, &context.request) {
            Some(language) => {
                if language != "*" {
                    context.selected_language = Some(language.clone());
                    context.response.add_header(s!("Content-Language"), vec![HeaderValue::parse_string(language)]);
                }
                DecisionResult::True
            },
            None => DecisionResult::False
        },
        &Decision::E5AcceptCharsetExists => DecisionResult::wrap(context.request.has_accept_charset_header()),
        &Decision::E6AcceptableCharsetAvailable => match content_negotiation::matching_charset(resource, &context.request) {
            Some(charset) => {
                if charset != "*" {
                    context.selected_charset = Some(charset.clone());
                }
                DecisionResult::True
            },
            None => DecisionResult::False
        },
        &Decision::F6AcceptEncodingExists => DecisionResult::wrap(context.request.has_accept_encoding_header()),
        &Decision::F7AcceptableEncodingAvailable => match content_negotiation::matching_encoding(resource, &context.request) {
            Some(encoding) => {
                context.selected_encoding = Some(encoding.clone());
                if encoding != "identity" {
                    context.response.add_header(s!("Content-Encoding"), vec![HeaderValue::parse_string(encoding)]);
                }
                DecisionResult::True
            },
            None => DecisionResult::False
        },
        &Decision::G7ResourceExists => DecisionResult::wrap(resource.resource_exists.as_ref()(context)),
        &Decision::G8IfMatchExists => DecisionResult::wrap(context.request.has_header(&s!("If-Match"))),
        &Decision::G9IfMatchStarExists | &Decision::H7IfMatchStarExists => DecisionResult::wrap(
            context.request.has_header_value(&s!("If-Match"), &s!("*"))),
        &Decision::G11EtagInIfMatch => DecisionResult::wrap(resource_etag_matches_header_values(resource, context, "If-Match")),
        &Decision::H10IfUnmodifiedSinceExists => DecisionResult::wrap(context.request.has_header(&s!("If-Unmodified-Since"))),
        &Decision::H11IfUnmodifiedSinceValid => DecisionResult::wrap(validate_header_date(&context.request,
            "If-Unmodified-Since", &mut context.if_unmodified_since)),
        &Decision::H12LastModifiedGreaterThanUMS => {
            match resource.last_modified.as_ref()(context) {
                Some(datetime) => DecisionResult::wrap(datetime > context.if_unmodified_since.unwrap()),
                None => DecisionResult::False
            }
        },
        &Decision::I7Put => if context.request.is_put() {
            context.new_resource = true;
            DecisionResult::True
        } else {
            DecisionResult::False
        },
        &Decision::I12IfNoneMatchExists => DecisionResult::wrap(context.request.has_header(&s!("If-None-Match"))),
        &Decision::I13IfNoneMatchStarExists => DecisionResult::wrap(context.request.has_header_value(&s!("If-None-Match"), &s!("*"))),
        &Decision::J18GetHead => DecisionResult::wrap(context.request.is_get_or_head()),
        &Decision::K7ResourcePreviouslyExisted => DecisionResult::wrap(resource.previously_existed.as_ref()(context)),
        &Decision::K13ETagInIfNoneMatch => DecisionResult::wrap(resource_etag_matches_header_values(resource, context, "If-None-Match")),
        &Decision::L5HasMovedTemporarily => match resource.moved_temporarily.as_ref()(context) {
            Some(location) => {
                context.response.add_header(s!("Location"), vec![HeaderValue::basic(&location)]);
                DecisionResult::True
            },
            None => DecisionResult::False
        },
        &Decision::L7Post | &Decision::M5Post | &Decision::N16Post => DecisionResult::wrap(context.request.is_post()),
        &Decision::L13IfModifiedSinceExists => DecisionResult::wrap(context.request.has_header(&s!("If-Modified-Since"))),
        &Decision::L14IfModifiedSinceValid => DecisionResult::wrap(validate_header_date(&context.request,
            "If-Modified-Since", &mut context.if_modified_since)),
        &Decision::L15IfModifiedSinceGreaterThanNow => {
            let datetime = context.if_modified_since.unwrap();
            let timezone = datetime.timezone();
            DecisionResult::wrap(datetime > Utc::now().with_timezone(&timezone))
        },
        &Decision::L17IfLastModifiedGreaterThanMS => {
            match resource.last_modified.as_ref()(context) {
                Some(datetime) => DecisionResult::wrap(datetime > context.if_modified_since.unwrap()),
                None => DecisionResult::False
            }
        },
        &Decision::I4HasMovedPermanently | &Decision::K5HasMovedPermanently => match resource.moved_permanently.as_ref()(context) {
            Some(location) => {
                context.response.add_header(s!("Location"), vec![HeaderValue::basic(&location)]);
                DecisionResult::True
            },
            None => DecisionResult::False
        },
        &Decision::M7PostToMissingResource | &Decision::N5PostToMissingResource =>
            if resource.allow_missing_post.as_ref()(context) {
                context.new_resource = true;
                DecisionResult::True
            } else {
                DecisionResult::False
            },
        &Decision::M16Delete => DecisionResult::wrap(context.request.is_delete()),
        &Decision::M20DeleteEnacted => match resource.delete_resource.as_ref()(context) {
            Ok(result) => DecisionResult::wrap(result),
            Err(status) => DecisionResult::StatusCode(status)
        },
        &Decision::N11Redirect => {
            if resource.post_is_create.as_ref()(context) {
                match resource.create_path.as_ref()(context) {
                    Ok(path) => {
                        let base_path = sanitise_path(&context.request.base_path);
                        let new_path = join_paths(&base_path, &sanitise_path(&path));
                        context.request.request_path = path.clone();
                        context.response.add_header(s!("Location"), vec![HeaderValue::basic(&new_path)]);
                        DecisionResult::wrap(context.redirect)
                    },
                    Err(status) => DecisionResult::StatusCode(status)
                }
            } else {
                match resource.process_post.as_ref()(context) {
                    Ok(_) => DecisionResult::wrap(context.redirect),
                    Err(status) => DecisionResult::StatusCode(status)
                }
            }
        },
        &Decision::P3Conflict | &Decision::O14Conflict => DecisionResult::wrap(resource.is_conflict.as_ref()(context)),
        &Decision::P11NewResource => {
            if context.request.is_put() {
                match resource.process_put.as_ref()(context) {
                    Ok(_) => DecisionResult::wrap(context.new_resource),
                    Err(status) => DecisionResult::StatusCode(status)
                }
            } else {
                DecisionResult::wrap(context.new_resource)
            }
        },
        &Decision::O16Put => DecisionResult::wrap(context.request.is_put()),
        &Decision::O18MultipleRepresentations => DecisionResult::wrap(resource.multiple_choices.as_ref()(context)),
        &Decision::O20ResponseHasBody => DecisionResult::wrap(context.response.has_body()),
        _ => DecisionResult::False
    }
}

fn execute_state_machine(context: &mut WebmachineContext, resource: &WebmachineResource) {
    let mut state = Decision::Start;
    let mut decisions: Vec<(Decision, bool, Decision)> = Vec::new();
    let mut loop_count = 0;
    while !state.is_terminal() {
        loop_count += 1;
        if loop_count >= MAX_STATE_MACHINE_TRANSITIONS {
            panic!("State machine has not terminated within {} transitions!", loop_count);
        }
        debug!("state is {:?}", state);
        state = match TRANSITION_MAP.get(&state) {
            Some(transition) => match transition {
                &Transition::To(ref decision) => {
                    debug!("Transitioning to {:?}", decision);
                    decision.clone()
                },
                &Transition::Branch(ref decision_true, ref decision_false) => {
                    match execute_decision(&state, context, resource) {
                        DecisionResult::True => {
                            debug!("Transitioning from {:?} to {:?} as decision is true", state, decision_true);
                            decisions.push((state, true, decision_true.clone()));
                            decision_true.clone()
                        },
                        DecisionResult::False => {
                            debug!("Transitioning from {:?} to {:?} as decision is false", state, decision_false);
                            decisions.push((state, false, decision_false.clone()));
                            decision_false.clone()
                        },
                        DecisionResult::StatusCode(code) => {
                            let decision = Decision::End(code);
                            debug!("Transitioning from {:?} to {:?} as decision is a status code", state, decision);
                            decisions.push((state, false, decision.clone()));
                            decision.clone()
                        }
                    }
                }
            },
            None => {
                error!("Error transitioning from {:?}, the TRANSITION_MAP is misconfigured", state);
                decisions.push((state, false, Decision::End(500)));
                Decision::End(500)
            }
        }
    }
    debug!("Final state is {:?}", state);
    match state {
        Decision::End(status) => context.response.status = status,
        Decision::A3Options => {
            context.response.status = 200;
            match resource.options.as_ref()(context, resource) {
                Some(headers) => context.response.add_headers(headers),
                None => ()
            }
        },
        _ => ()
    }
}

fn update_paths_for_resource(request: &mut WebmachineRequest, base_path: &String) {
    request.base_path = base_path.clone();
    if request.request_path.len() > base_path.len() {
        let request_path = request.request_path.clone();
        let subpath = request_path.split_at(base_path.len()).1;
        if subpath.starts_with("/") {
            request.request_path = s!(subpath);
        } else {
            request.request_path = s!("/") + subpath;
        }
    } else {
        request.request_path = s!("/");
    }
}

fn parse_header_values(value: &str) -> Vec<HeaderValue> {
  if value.is_empty() {
    Vec::new()
  } else {
    value.split(',').map(|s| HeaderValue::parse_string(s!(s.trim()))).collect()
  }
}

fn headers_from_http_request(req: &Request<Vec<u8>>) -> HashMap<String, Vec<HeaderValue>> {
  req.headers().iter()
    .map(|header| (s!(header.0.as_str()), parse_header_values(header.1.to_str().unwrap_or_default())))
    .collect()
}

fn decode_query(query: &str) -> String {
  let mut chars = query.chars();
  let mut ch = chars.next();
  let mut result = String::new();

  while ch.is_some() {
    let c = ch.unwrap();
    if c == '%' {
      let c1 = chars.next();
      let c2 = chars.next();
      match (c1, c2) {
        (Some(v1), Some(v2)) => {
          let mut s = String::new();
          s.push(v1);
          s.push(v2);
          let decoded: Result<Vec<u8>, _> = hex::decode(s);
          match decoded {
            Ok(n) => result.push(n[0] as char),
            Err(_) => {
              result.push('%');
              result.push(v1);
              result.push(v2);
            }
          }
        },
        (Some(v1), None) => {
          result.push('%');
          result.push(v1);
        },
        _ => result.push('%')
      }
    } else if c == '+' {
      result.push(' ');
    } else {
      result.push(c);
    }

    ch = chars.next();
  }

  result
}

fn parse_query(query: &str) -> HashMap<String, Vec<String>> {
  if !query.is_empty() {
    query.split("&").map(|kv| {
      if kv.is_empty() {
        vec![]
      } else if kv.contains("=") {
        kv.splitn(2, "=").collect::<Vec<&str>>()
      } else {
        vec![kv]
      }
    }).fold(HashMap::new(), |mut map, name_value| {
      if !name_value.is_empty() {
        let name = decode_query(name_value[0]);
        let value = if name_value.len() > 1 {
          decode_query(name_value[1])
        } else {
          String::new()
        };
        map.entry(name).or_insert(vec![]).push(value);
      }
      map
    })
  } else {
    HashMap::new()
  }
}

fn request_from_http_request(req: &Request<Vec<u8>>) -> WebmachineRequest {
  let request_path = req.uri().path().to_string();
  let body = if req.body().is_empty() {
    None
  } else {
    Some(req.body().clone())
  };
  let query = match req.uri().query() {
    Some(query) => parse_query(query),
    None => HashMap::new()
  };
  WebmachineRequest {
    request_path: request_path.clone(),
    base_path: s!("/"),
    method: req.method().as_str().into(),
    headers: headers_from_http_request(req),
    body,
    query
  }
}

fn finalise_response(context: &mut WebmachineContext, resource: &WebmachineResource) {
    if !context.response.has_header(&s!("Content-Type")) {
        let media_type = match &context.selected_media_type {
            &Some(ref media_type) => media_type.clone(),
            &None => s!("application/json")
        };
        let charset = match &context.selected_charset {
            &Some(ref charset) => charset.clone(),
            &None => s!("ISO-8859-1")
        };
        let header = HeaderValue {
            value: media_type,
            params: hashmap!{ s!("charset") => charset },
            quote: false
        };
        context.response.add_header(s!("Content-Type"), vec![header]);
    }

    let mut vary_header = if !context.response.has_header(&s!("Vary")) {
        resource.variances
            .iter()
            .map(|h| HeaderValue::parse_string(h.clone()))
            .collect()
    } else {
        Vec::new()
    };

    if resource.languages_provided.len() > 1 {
        vary_header.push(h!("Accept-Language"));
    }
    if resource.charsets_provided.len() > 1 {
        vary_header.push(h!("Accept-Charset"));
    }
    if resource.encodings_provided.len() > 1 {
        vary_header.push(h!("Accept-Encoding"));
    }
    if resource.produces.len() > 1 {
        vary_header.push(h!("Accept"));
    }

    if vary_header.len() > 1 {
        context.response.add_header(s!("Vary"), vary_header.iter().cloned().unique().collect());
    }

    if context.request.is_get_or_head() {
        match resource.generate_etag.as_ref()(context) {
            Some(etag) => context.response.add_header(s!("ETag"), vec![HeaderValue::basic(&etag).quote()]),
            None => ()
        }
        match resource.expires.as_ref()(context) {
            Some(datetime) => context.response.add_header(s!("Expires"),
                vec![HeaderValue::basic(&datetime.to_rfc2822()).quote()]),
            None => ()
        }
        match resource.last_modified.as_ref()(context) {
            Some(datetime) => context.response.add_header(s!("Last-Modified"),
                vec![HeaderValue::basic(&datetime.to_rfc2822()).quote()]),
            None => ()
        }
    }

    if context.response.body.is_none() && context.response.status == 200 && context.request.is_get() {
        match resource.render_response.as_ref()(context) {
            Some(body) => context.response.body = Some(body.into_bytes()),
            None => ()
        }
    }

    match &resource.finalise_response {
        &Some(ref callback) => callback.as_ref()(context),
        &None => ()
    }

    debug!("Final response: {:?}", context.response);
}

fn generate_http_response(context: &WebmachineContext) -> http::Result<Response<Vec<u8>>> {
  let mut response = Response::builder().status(context.response.status);

  for (header, values) in context.response.headers.clone() {
    let header_values = values.iter().map(|h| h.to_string()).join(", ");
    response = response.header(&header, &header_values);
  }
  match context.response.body.clone() {
    Some(body) => response.body(body),
    None => response.body(vec![])
  }
}

/// The main hyper dispatcher
pub struct WebmachineDispatcher {
    /// Map of routes to webmachine resources
    pub routes: Mutex<BTreeMap<String, Arc<WebmachineResource>>>
}

impl WebmachineDispatcher {
    /// Create a new Webmachine dispatcher given a map of routes to webmachine resources
    pub fn new(routes: BTreeMap<String, Arc<WebmachineResource>>) -> WebmachineDispatcher {
        WebmachineDispatcher {
            routes: Mutex::new(routes.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        }
    }

    /// Main dispatch function for the Webmachine. This will look for a matching resource
    /// based on the request path. If one is not found, a 404 Not Found response is returned
    pub fn dispatch(&self, req: Request<Vec<u8>>) -> http::Result<Response<Vec<u8>>> {
      let mut context = self.context_from_http_request(&req);
      self.dispatch_to_resource(&mut context);
      generate_http_response(&context)
    }

    fn context_from_http_request(&self, req: &Request<Vec<u8>>) -> WebmachineContext {
      let request = request_from_http_request(req);
      WebmachineContext {
        request,
        response: WebmachineResponse::default(),
        .. WebmachineContext::default()
      }
    }

    fn match_paths(&self, request: &WebmachineRequest) -> Vec<String> {
        let request_path = sanitise_path(&request.request_path);
        let routes = self.routes.lock().unwrap();
        routes
            .keys()
            .cloned()
            .filter(|k| request_path.starts_with(&sanitise_path(k)))
            .collect()
    }

    fn lookup_resource(&self, path: &String) -> Arc<WebmachineResource> {
        let routes = self.routes.lock().unwrap();
        let resource = routes.get(path).unwrap();
        resource.clone()
    }

    /// Dispatches to the matching webmachine resource. If there is no matching resource, returns
    /// 404 Not Found response
    pub fn dispatch_to_resource(&self, context: &mut WebmachineContext) {
        let matching_paths = self.match_paths(&context.request);
        let ordered_by_length: Vec<String> = matching_paths.iter()
            .cloned()
            .sorted_by(|a, b| Ord::cmp(&b.len(), &a.len())).collect();
        match ordered_by_length.first() {
            Some(path) => {
                update_paths_for_resource(&mut context.request, path);
                let resource = self.lookup_resource(path);
                execute_state_machine(context, &resource);
                finalise_response(context, &resource);
            },
            None => context.response.status = 404
        };
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod content_negotiation_tests;
