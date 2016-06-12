//! The `webmachine-rust` crate provides a port of webmachine to rust (port of webmachine-ruby).

#![warn(missing_docs)]

extern crate hyper;
#[macro_use] extern crate log;
#[macro_use] extern crate p_macro;
#[macro_use] extern crate maplit;
#[macro_use] extern crate itertools;
#[macro_use] extern crate lazy_static;

use std::collections::{BTreeMap, HashMap};
use hyper::server::{Request, Response};
use hyper::uri::RequestUri;
use hyper::status::StatusCode;
use itertools::Itertools;

/// Simple macro to convert a string slice to a `String` struct.
#[macro_export]
macro_rules! s {
    ($e:expr) => ($e.to_string())
}

pub mod context;
pub mod headers;

use context::*;
use headers::*;

/// Struct to represent a resource in webmachine
pub struct WebmachineResource {
    /// Is the resource available? Returning false will result in a '503 Service Not Available'
    /// response. Defaults to true. If the resource is only temporarily not available,
    /// add a 'Retry-After' response header.
    pub available: Box<Fn(&mut WebmachineContext) -> bool>,
    /// HTTP methods that are known to the resource. Default includes all standard HTTP methods.
    /// One could override this to allow additional methods
    pub known_methods: Vec<String>,
    /// If the URI is too long to be processed, this should return true, which will result in a
    // '414 Request URI Too Long' response. Defaults to false.
    pub uri_too_long: Box<Fn(&mut WebmachineContext) -> bool>,
    /// HTTP methods that are allowed on this resource. Defaults to GET','HEAD and 'OPTIONS'.
    pub allowed_methods: Vec<String>,
    /// If the request is malformed, this should return true, which will result in a
    /// '400 Malformed Request' response. Defaults to false.
    pub malformed_request: Box<Fn(&mut WebmachineContext) -> bool>,
    /// Is the client or request not authorized? Returning a Some<String>
    /// will result in a '401 Unauthorized' response.  Defaults to None. If a Some(String) is
    /// returned, the string will be used as the value in the WWW-Authenticate header.
    pub not_authorized: Box<Fn(&mut WebmachineContext) -> Option<String>>,
    /// Is the request or client forbidden? Returning true will result in a '403 Forbidden' response.
    /// Defaults to false.
    pub forbidden: Box<Fn(&mut WebmachineContext) -> bool>,
    /// If the request includes any invalid Content-* headers, this should return true, which will
    /// result in a '501 Not Implemented' response. Defaults to false.
    pub unsupported_content_headers: Box<Fn(&mut WebmachineContext) -> bool>,
    /// The list of acceptable content types. Defaults to 'application/json'. If the content type
    /// of the request is not in this list, a '415 Unsupported Media Type' response is returned.
    pub acceptable_content_types: Vec<String>,
    /// If the entity length on PUT or POST is invalid, this should return false, which will result
    /// in a '413 Request Entity Too Large' response. Defaults to true.
    pub valid_entity_length: Box<Fn(&mut WebmachineContext) -> bool>,
    /// This is called just before the final response is constructed and sent. This allows the
    /// response to be modified. The default implementation adds CORS headers to the response
    pub finish_request: Box<Fn(&mut WebmachineContext, &WebmachineResource)>,
    /// If the OPTIONS method is supported and is used, this return a HashMap of headers that
    // should appear in the response. Defaults to CORS headers.
    pub options: Box<Fn(&mut WebmachineContext, &WebmachineResource) -> Option<HashMap<String, Vec<String>>>>
}

impl WebmachineResource {
    /// Creates a default webmachine resource
    pub fn default() -> WebmachineResource {
        WebmachineResource {
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
            options: Box::new(|_, resource| Some(WebmachineResponse::cors_headers(&resource.allowed_methods)))
        }
    }
}

fn extract_path(uri: &RequestUri) -> String {
    match uri {
        &RequestUri::AbsolutePath(ref s) => s.splitn(2, "?").next().unwrap_or("/").to_string(),
        &RequestUri::AbsoluteUri(ref url) => url.path().to_string(),
        _ => uri.to_string()
    }
}

fn sanitise_path(path: &String) -> Vec<String> {
    path.split("/").filter(|p| !p.is_empty()).map(|p| p.to_string()).collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Decision {
    Start,
    End(u16),
    B13Available,
    B12KnownMethod,
    B11UriTooLong,
    B10MethodAllowed,
    B9MalformedRequest,
    B8Authorized,
    B7Forbidden,
    B6UnsupportedContentHeader,
    B5UnkownContentType,
    B4RequestEntityTooLarge,
    B3Options,
    A3Options
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

lazy_static! {
    static ref TRANSITION_MAP: HashMap<Decision, Transition> = hashmap!{
        Decision::Start => Transition::To(Decision::B13Available),
        Decision::B13Available => Transition::Branch(Decision::B12KnownMethod, Decision::End(503)),
        Decision::B12KnownMethod => Transition::Branch(Decision::B11UriTooLong, Decision::End(501)),
        Decision::B11UriTooLong => Transition::Branch(Decision::End(414), Decision::B10MethodAllowed),
        Decision::B10MethodAllowed => Transition::Branch(Decision::B9MalformedRequest, Decision::End(405)),
        Decision::B9MalformedRequest => Transition::Branch(Decision::End(400), Decision::B8Authorized),
        Decision::B8Authorized => Transition::Branch(Decision::B7Forbidden, Decision::End(401)),
        Decision::B7Forbidden => Transition::Branch(Decision::End(403), Decision::B6UnsupportedContentHeader),
        Decision::B6UnsupportedContentHeader => Transition::Branch(Decision::End(501), Decision::B5UnkownContentType),
        Decision::B5UnkownContentType => Transition::Branch(Decision::End(415), Decision::B4RequestEntityTooLarge),
        Decision::B4RequestEntityTooLarge => Transition::Branch(Decision::End(413), Decision::B3Options),
        Decision::B3Options => Transition::Branch(Decision::A3Options, Decision::End(200)),
    };
}

fn execute_decision(decision: &Decision, context: &mut WebmachineContext, resource: &WebmachineResource) -> bool {
    match decision {
        &Decision::B13Available => resource.available.as_ref()(context),
        &Decision::B12KnownMethod => resource.known_methods
            .iter().find(|m| m.to_uppercase() == context.request.method.to_uppercase()).is_some(),
        &Decision::B11UriTooLong => resource.uri_too_long.as_ref()(context),
        &Decision::B10MethodAllowed => {
            match resource.allowed_methods
                .iter().find(|m| m.to_uppercase() == context.request.method.to_uppercase()) {
                Some(_) => true,
                None => {
                    context.response.add_header(s!("Allow"), resource.allowed_methods
                        .iter()
                        .map(HeaderValue::basic)
                        .collect());
                    false
                }
            }
        },
        &Decision::B9MalformedRequest => resource.malformed_request.as_ref()(context),
        &Decision::B8Authorized => match resource.not_authorized.as_ref()(context) {
            Some(realm) => {
                context.response.add_header(s!("WWW-Authenticate"), vec![HeaderValue::parse_string(realm.clone())]);
                false
            },
            None => true
        },
        &Decision::B7Forbidden => resource.forbidden.as_ref()(context),
        &Decision::B6UnsupportedContentHeader => resource.unsupported_content_headers.as_ref()(context),
        &Decision::B5UnkownContentType => context.request.is_put_or_post() && resource.acceptable_content_types
                .iter().find(|ct| context.request.content_type().to_uppercase() == ct.to_uppercase() )
                .is_none(),
        &Decision::B4RequestEntityTooLarge => context.request.is_put_or_post() && !resource.valid_entity_length.as_ref()(context),
        &Decision::B3Options => context.request.is_options(),
        _ => false
    }
}

fn execute_state_machine(context: &mut WebmachineContext, resource: &WebmachineResource) {
    let mut state = Decision::Start;
    let mut decisions: Vec<(Decision, bool, Decision)> = Vec::new();
    while !state.is_terminal() {
        debug!("state is {:?}", state);
        state = match TRANSITION_MAP.get(&state) {
            Some(transition) => match transition {
                &Transition::To(ref decision) => {
                    debug!("Transitioning to {:?}", decision);
                    decision.clone()
                },
                &Transition::Branch(ref decision_true, ref decision_false) => {
                    if execute_decision(&state, context, resource) {
                        debug!("Transitioning from {:?} to {:?} as decision is true", state, decision_true);
                        decisions.push((state, true, decision_true.clone()));
                        decision_true.clone()
                    } else {
                        debug!("Transitioning from {:?} to {:?} as decision is false", state, decision_false);
                        decisions.push((state, false, decision_false.clone()));
                        decision_false.clone()
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
    debug!("Decisions: {:?}", decisions);
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
    if request.request_path.len() >  base_path.len() {
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

fn parse_header_values(value: String) -> Vec<HeaderValue> {
    if value.is_empty() {
        Vec::new()
    } else {
        value.split(',').map(|s| HeaderValue::parse_string(s!(s.trim()))).collect()
    }
}

fn headers_from_hyper_request(req: &Request) -> HashMap<String, Vec<HeaderValue>> {
    req.headers.iter()
        .map(|header| (s!(header.name()), parse_header_values(header.value_string())))
        .collect()
}

fn request_from_hyper_request(req: &Request) -> WebmachineRequest {
    let request_path = extract_path(&req.uri);
    WebmachineRequest {
        request_path: request_path.clone(),
        base_path: s!("/"),
        method: s!(req.method.as_ref()),
        headers: headers_from_hyper_request(req)
    }
}

fn generate_hyper_response(context: &WebmachineContext, res: &mut Response) {
    *res.status_mut() = StatusCode::from_u16(context.response.status);
    for (header, values) in context.response.headers.clone() {
        let header = header.clone();
        let header_values = values.iter().map(|h| h.to_string()).join(", ").into_bytes();
        res.headers_mut().set_raw(header, vec![header_values]);
    }
}

/// The main hyper dispatcher
pub struct WebmachineDispatcher {
    /// Map of routes to webmachine resources
    pub routes: BTreeMap<String, WebmachineResource>
}

impl WebmachineDispatcher {
    /// Main hyper dispatch function for the Webmachine. This will look for a matching resource
    /// based on the request path. If one is not found, a 404 Not Found response is returned
    pub fn dispatch(&self, req: Request, mut res: Response) {
        let mut context = self.context_from_hyper_request(&req);
        self.dispatch_to_resource(&mut context);
        generate_hyper_response(&context, &mut res);
    }

    fn context_from_hyper_request(&self, req: &Request) -> WebmachineContext {
        let request = request_from_hyper_request(req);
        WebmachineContext {
            request: request,
            response: WebmachineResponse::default()
        }
    }

    fn match_paths(&self, request: &WebmachineRequest) -> Vec<String> {
        let request_path = sanitise_path(&request.request_path);
        self.routes
            .keys()
            .cloned()
            .filter(|k| request_path.starts_with(&sanitise_path(k)))
            .collect()
    }

    /// Dispatches to the matching webmachine resource. If there is no matching resource, returns
    /// 404 Not Found response
    pub fn dispatch_to_resource(&self, context: &mut WebmachineContext) {
        let matching_paths = self.match_paths(&context.request);
        let ordered_by_length = matching_paths.clone().iter()
            .cloned()
            .sorted_by(|a, b| Ord::cmp(&b.len(), &a.len()));
        match ordered_by_length.first() {
            Some(path) => {
                let resource = self.routes.get(path).unwrap();
                update_paths_for_resource(&mut context.request, path);
                execute_state_machine(context, resource);
            },
            None => context.response.status = 404
        };
    }
}

#[cfg(test)]
#[macro_use(expect)]
extern crate expectest;

#[cfg(test)]
mod tests;
