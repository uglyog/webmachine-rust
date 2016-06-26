# webmachine-rust
Port of Webmachine-Ruby (https://github.com/webmachine/webmachine-ruby) to Rust.

[![Build Status](https://travis-ci.org/uglyog/webmachine-rust.svg?branch=master)](https://travis-ci.org/uglyog/webmachine-rust)

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

Currently the following features have not been implemented:

- Visual debugger (like Webmachine-Ruby)
- Streaming response bodies
-

## Implementation Deficiencies:

This implementation has the following deficiencies:

- Only supports Hyper
- WebmachineDispatcher and WebmachineResource are not shareable between threads.
- Automatically decoding request bodies and encoding response bodies.
- No easy mechanism to generate bodies with different content types (e.g. JSON vs. XML).
- No easy mechanism for handling sub-paths in a resource.
- Does not work with keep alive enabled (does not manage the Hyper thread pool).
- Dynamically determining the methods allowed on the resource.

## Getting started

Follow the getting started documentation from the Hyper crate to setup a Hyper Handler for your server. Then from the
handle function, you need to define a WebmachineDispatcher that maps resource paths to your webmachine resources (WebmachineResource). Each WebmachineResource defines all the callbacks (via Closures) and values required to implement a
resource.

Note: This example uses the maplit crate to provide the `btreemap` macro and the log crate for the logging macros.

```rust
use std::sync::Arc;
use hyper::server::{Handler, Server, Request, Response};
use webmachine_rust::*;
use webmachine_rust::context::*;
use webmachine_rust::headers::*;
use rustc_serialize::json::Json;

struct ServerHandler {
}

impl Handler for ServerHandler {

    fn handle(&self, req: Request, res: Response) {
        // setup the dispatcher, which maps paths to resources
        let dispatcher = WebmachineDispatcher::new(
            btreemap!{
                s!("/myresource") => Arc::new(WebmachineResource {
                    // Methods allowed on this resource
                    allowed_methods: vec![s!("OPTIONS"), s!("GET"), s!("HEAD"), s!("POST")],
                    // if the resource exists callback
                    resource_exists: Box::new(|context| true),
                    // callback to render the response for the resource
                    render_response: Box::new(|_| {
                        let mut data = vec![1, 2, 3, 4];
                        let json_response = Json::Object(btreemap!{ s!("data") => Json::Array(data) });
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
        match dispatcher.dispatch(req, res) {
            Ok(_) => (),
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
