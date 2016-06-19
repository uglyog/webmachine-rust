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
macro_rules! s {
    ($e:expr) => ($e.to_string())
}

pub mod headers;

/// Simple macro to convert a string to a `HeaderValue` struct.
macro_rules! h {
    ($e:expr) => (HeaderValue::parse_string($e.to_string()))
}

pub mod context;
pub mod content_negotiation;

use context::*;
use headers::*;

/// Struct to represent a resource in webmachine
pub struct WebmachineResource {
    /// This is called just before the final response is constructed and sent. It allows the resource
    /// an opportunity to modify the response after the webmachine has executed.
    pub finalise_response: Option<Box<Fn(&mut WebmachineContext)>>,
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
    pub options: Box<Fn(&mut WebmachineContext, &WebmachineResource) -> Option<HashMap<String, Vec<String>>>>,
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
    pub resource_exists: Box<Fn(&mut WebmachineContext) -> bool>,
    /// If this resource is known to have existed previously, this should return true. Default is false.
    pub previously_existed: Box<Fn(&mut WebmachineContext) -> bool>,
    /// If this resource has moved to a new location permanently, this should return the new
    /// location as a String. Default is to return None
    pub moved_permanently: Box<Fn(&mut WebmachineContext) -> Option<String>>,
    /// If this returns true, the client will receive a '409 Conflict' response. This is only
    /// called for PUT requests. Default is false.
    pub is_conflict: Box<Fn(&mut WebmachineContext) -> bool>,
    /// Return true if the resource accepts POST requests to nonexistent resources. Defaults to false.
    pub allow_missing_post: Box<Fn(&mut WebmachineContext) -> bool>,
}

impl WebmachineResource {
    /// Creates a default webmachine resource
    pub fn default() -> WebmachineResource {
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
            is_conflict: Box::new(|_| false),
            allow_missing_post: Box::new(|_| false)
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

const MAX_STATE_MACHINE_TRANSISIONS: u8 = 100;

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
    C7NotAcceptable,
    D4AcceptLanguageExists,
    D5AcceptableLanguageAvailable,
    E5AcceptCharsetExists,
    E6AcceptableCharsetAvailable,
    F6AcceptEncodingExists,
    F7AcceptableEncodingAvailable,
    G7ResourceExists,
    H7IfMatchStarExists,
    I4HasMovedPermanently,
    I7Put,
    K5HasMovedPermanently,
    K7ResourcePreviouslyExisted,
    L7Post,
    M7PostToMissingResource,
    P3Conflict
}

impl Decision {
    fn is_terminal(&self) -> bool {
        match self {
            &Decision::End(_) => true,
            &Decision::A3Options => true,
            &Decision::C7NotAcceptable => true,
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
        Decision::C4AcceptableMediaTypeAvailable => Transition::Branch(Decision::D4AcceptLanguageExists, Decision::C7NotAcceptable),
        Decision::D4AcceptLanguageExists => Transition::Branch(Decision::D5AcceptableLanguageAvailable, Decision::E5AcceptCharsetExists),
        Decision::D5AcceptableLanguageAvailable => Transition::Branch(Decision::E5AcceptCharsetExists, Decision::C7NotAcceptable),
        Decision::E5AcceptCharsetExists => Transition::Branch(Decision::E6AcceptableCharsetAvailable, Decision::F6AcceptEncodingExists),
        Decision::E6AcceptableCharsetAvailable => Transition::Branch(Decision::F6AcceptEncodingExists, Decision::C7NotAcceptable),
        Decision::F6AcceptEncodingExists => Transition::Branch(Decision::F7AcceptableEncodingAvailable, Decision::G7ResourceExists),
        Decision::F7AcceptableEncodingAvailable => Transition::Branch(Decision::G7ResourceExists, Decision::C7NotAcceptable),
        Decision::G7ResourceExists => Transition::Branch(/* --> */Decision::End(200), Decision::H7IfMatchStarExists),
        Decision::H7IfMatchStarExists => Transition::Branch(Decision::End(412), Decision::I7Put),
        Decision::I4HasMovedPermanently => Transition::Branch(Decision::End(301), Decision::P3Conflict),
        Decision::I7Put => Transition::Branch(Decision::I4HasMovedPermanently, Decision::K7ResourcePreviouslyExisted),
        Decision::K5HasMovedPermanently => Transition::Branch(Decision::End(301), /* --> */Decision::End(200)),
        Decision::K7ResourcePreviouslyExisted => Transition::Branch(Decision::K5HasMovedPermanently, Decision::L7Post),
        Decision::L7Post => Transition::Branch(Decision::M7PostToMissingResource, Decision::End(404)),
        Decision::M7PostToMissingResource => Transition::Branch(/* --> */Decision::End(200), Decision::End(404)),
        Decision::P3Conflict => Transition::Branch(Decision::End(409), /* --> */Decision::End(200)),
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
        &Decision::C3AcceptExists => context.request.has_accept_header(),
        &Decision::C4AcceptableMediaTypeAvailable => match content_negotiation::matching_content_type(resource, &context.request) {
            Some(media_type) => {
                context.selected_media_type = Some(media_type);
                true
            },
            None => false
        },
        &Decision::D4AcceptLanguageExists => context.request.has_accept_language_header(),
        &Decision::D5AcceptableLanguageAvailable => match content_negotiation::matching_language(resource, &context.request) {
            Some(language) => {
                if language != "*" {
                    context.selected_language = Some(language.clone());
                    context.response.add_header(s!("Content-Language"), vec![HeaderValue::parse_string(language)]);
                }
                true
            },
            None => false
        },
        &Decision::E5AcceptCharsetExists => context.request.has_accept_charset_header(),
        &Decision::E6AcceptableCharsetAvailable => match content_negotiation::matching_charset(resource, &context.request) {
            Some(charset) => {
                if charset != "*" {
                    context.selected_charset = Some(charset.clone());
                }
                true
            },
            None => false
        },
        &Decision::F6AcceptEncodingExists => context.request.has_accept_encoding_header(),
        &Decision::F7AcceptableEncodingAvailable => match content_negotiation::matching_encoding(resource, &context.request) {
            Some(encoding) => {
                context.selected_encoding = Some(encoding.clone());
                if encoding != "identity" {
                    context.response.add_header(s!("Content-Encoding"), vec![HeaderValue::parse_string(encoding)]);
                }
                true
            },
            None => false
        },
        &Decision::G7ResourceExists => resource.resource_exists.as_ref()(context),
        &Decision::H7IfMatchStarExists => context.request.has_header_value(&s!("If-Match"), &s!("*")),
        &Decision::I7Put => context.request.is_put(),
        &Decision::K7ResourcePreviouslyExisted => resource.previously_existed.as_ref()(context),
        &Decision::L7Post => context.request.is_post(),
        &Decision::I4HasMovedPermanently | &Decision::K5HasMovedPermanently => match resource.moved_permanently.as_ref()(context) {
            Some(location) => {
                context.response.add_header(s!("Location"), vec![HeaderValue::basic(&location)]);
                true
            },
            None => false
        },
        &Decision::P3Conflict => resource.is_conflict.as_ref()(context),
        &Decision::M7PostToMissingResource => resource.allow_missing_post.as_ref()(context),
        _ => false
    }
}

fn execute_state_machine(context: &mut WebmachineContext, resource: &WebmachineResource) {
    let mut state = Decision::Start;
    let mut decisions: Vec<(Decision, bool, Decision)> = Vec::new();
    let mut loop_count = 0;
    while !state.is_terminal() {
        loop_count += 1;
        if loop_count >= MAX_STATE_MACHINE_TRANSISIONS {
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
    match state {
        Decision::End(status) => context.response.status = status,
        Decision::A3Options => {
            context.response.status = 200;
            match resource.options.as_ref()(context, resource) {
                Some(headers) => context.response.add_headers(headers),
                None => ()
            }
        },
        Decision::C7NotAcceptable => context.response.status = 406,
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
            params: hashmap!{ s!("charset") => charset }
        };
        context.response.add_header(s!("Content-Type"), vec![header]);
    }

    if !resource.variances.is_empty() && !context.response.has_header(&s!("Vary")) {
        context.response.add_header(s!("Vary"), resource.variances
            .iter()
            .map(|h| HeaderValue::parse_string(h.clone()))
            .collect()
        );
    }

    match &resource.finalise_response {
        &Some(ref callback) => callback.as_ref()(context),
        &None => ()
    }

    debug!("Final response: {:?}", context.response);
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
            response: WebmachineResponse::default(),
            selected_media_type: None,
            selected_language: None,
            selected_charset: None,
            selected_encoding: None
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
                finalise_response(context, resource);
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

#[cfg(test)]
mod content_negotiation_tests;
