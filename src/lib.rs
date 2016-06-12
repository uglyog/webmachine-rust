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

use context::*;

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
    pub malformed_request: Box<Fn(&mut WebmachineContext) -> bool>
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
            malformed_request: Box::new(|_| false)
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
    B9MalformedRequest
}

impl Decision {
    fn is_terminal(&self) -> bool {
        match self {
            &Decision::End(_) => true,
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
        Decision::B9MalformedRequest => Transition::Branch(Decision::End(400), Decision::End(200))
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
                    context.response.add_header(s!("Allow"), resource.allowed_methods.clone());
                    false
                }
            }
        },
        &Decision::B9MalformedRequest => resource.malformed_request.as_ref()(context),
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

fn request_from_hyper_request(req: &Request) -> WebmachineRequest {
    let request_path = extract_path(&req.uri);
    WebmachineRequest {
        request_path: request_path.clone(),
        base_path: s!("/"),
        method: s!(req.method.as_ref())
    }
}

fn generate_hyper_response(context: &WebmachineContext, res: &mut Response) {
    *res.status_mut() = StatusCode::from_u16(context.response.status);
    for (header, values) in context.response.headers.clone() {
        let header = header.clone();
        let header_values = values.iter().join(", ").into_bytes();
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
