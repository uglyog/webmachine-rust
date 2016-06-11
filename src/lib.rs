//! The `webmachine-rust` crate provides a port of webmachine to rust.

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
    B13Available
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
        Decision::B13Available => Transition::Branch(Decision::End(200), Decision::End(503))
    };
}

fn execute_decision(decision: &Decision, selected_path: &String, resource: &WebmachineResource,
    mut req: &Request, mut res: &Response) -> bool {
    match decision {
        &Decision::B13Available => false,
        _ => false
    }
}

fn execute_state_machine(selected_path: &String, resource: &WebmachineResource, req: &Request, res: &mut Response) {
    let mut state = Decision::Start;
    let mut decisions: Vec<(Decision, bool, Decision)> = Vec::new();
    while !state.is_terminal() {
        p!(state);
        state = match TRANSITION_MAP.get(&state) {
            Some(transition) => match transition {
                &Transition::To(ref decision) => decision.clone(),
                &Transition::Branch(ref decision_true, ref decision_false) => {
                    if execute_decision(&state, selected_path, resource, req, res) {
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
    p!(state);
    p!(decisions);
    match state {
        Decision::End(status) => *res.status_mut() = StatusCode::from_u16(status),
        _ => ()
    }
}

/// Struct to represent a resource in webmachine
pub struct WebmachineResource {

}

impl WebmachineResource {
    /// Creates a default webmachine resource
    pub fn default() -> WebmachineResource {
        WebmachineResource {

        }
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
        let request_path = extract_path(&req.uri);
        let matching_paths: Vec<String> = self.match_paths(&request_path);
        if matching_paths.is_empty() {
            *res.status_mut() = StatusCode::NotFound;
        } else {
            let ordered_by_length = matching_paths.clone().iter()
                .cloned()
                .sorted_by(|a, b| Ord::cmp(&b.len(), &a.len()));
            let selected_path = ordered_by_length.first().unwrap();
            execute_state_machine(selected_path, self.routes.get(selected_path).unwrap(), &req, &mut res);
        }
    }

    fn match_paths(&self, request_path: &String) -> Vec<String> {
        let request_path = sanitise_path(request_path);
        self.routes
            .keys()
            .cloned()
            .filter(|k| request_path.starts_with(&sanitise_path(k)))
            .collect()
    }
}

#[cfg(test)]
#[macro_use(expect)]
extern crate expectest;

#[cfg(test)]
mod tests;
