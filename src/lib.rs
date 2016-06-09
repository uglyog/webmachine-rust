//! The `webmachine-rust` crate provides a port of webmachine to rust.

#![warn(missing_docs)]

extern crate hyper;
#[macro_use] extern crate log;
#[macro_use] extern crate p_macro;
#[macro_use] extern crate maplit;

use std::collections::BTreeMap;
use hyper::server::{Request, Response};

/// Struct to represent a resource in webmachine
pub struct WebmachineResource {

}

/// The main hyper dispatcher
pub struct WebmachineDispatcher {
    routes: BTreeMap<String, WebmachineResource>
}

impl WebmachineDispatcher {
    /// Main hyper dispatch function for the Webmachine
    pub fn dispatch(mut req: Request, mut res: Response) {

    }
}

#[cfg(test)]
mod tests;
