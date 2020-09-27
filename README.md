# webmachine-rust

Port of Webmachine-Ruby (https://github.com/webmachine/webmachine-ruby) to Rust.

[![Build Status](https://travis-ci.org/uglyog/webmachine-rust.svg?branch=master)](https://travis-ci.org/uglyog/webmachine-rust)

webmachine-rust is a port of the Ruby version of webmachine. It implements a finite state machine for the HTTP protocol
that provides semantic HTTP handling (based on the [diagram from the webmachine project](https://webmachine.github.io/images/http-headers-status-v3.png)).
It is basically a HTTP toolkit for building HTTP-friendly applications using the [Hyper](https://crates.io/crates/hyper) rust crate.

Webmachine-rust works with Hyper and sits between the Hyper Handler and your application code. It provides a resource struct
with callbacks to handle the decisions required as the state machine is executed against the request with the following sequence.

REQUEST -> Hyper Handler -> WebmachineDispatcher -> WebmachineResource -> Your application code -> WebmachineResponse -> Hyper -> RESPONSE

## Features

- Handles the hard parts of content negotiation, conditional requests, and response codes for you.
- Provides a resource struct with points of extension to let you describe what is relevant about your particular resource.

## Missing Features

Currently, the following features from webmachine-ruby have not been implemented:

- Visual debugger
- Streaming response bodies

## Implementation Deficiencies:

This implementation has the following deficiencies:

- Automatically decoding request bodies and encoding response bodies.
- No easy mechanism to generate bodies with different content types (e.g. JSON vs. XML).
- No easy mechanism for handling sub-paths in a resource.
- Dynamically determining the methods allowed on the resource.

## Getting started with Hyper

Follow the getting started documentation from the Hyper crate to setup a Hyper service for your server.
You need to define a WebmachineDispatcher that maps resource paths to your webmachine resources (WebmachineResource).
Each WebmachineResource defines all the callbacks (via Closures) and values required to implement a resource.
The WebmachineDispatcher implementes the Hyper Service trait, so you can pass it to the `make_service_fn`.

Note: This example uses the maplit crate to provide the `btreemap` macro and the log crate for the logging macros.

 ```rust
 use hyper::server::Server;
 use webmachine_rust::*;
 use webmachine_rust::context::*;
 use webmachine_rust::headers::*;
 use serde_json::{Value, json};
 use std::io::Read;
 use std::net::SocketAddr;
 use hyper::service::make_service_fn;
 use std::convert::Infallible;

 // setup the dispatcher, which maps paths to resources. The requirement of make_service_fn is
 // that it has a static lifetime
 fn dispatcher() -> WebmachineDispatcher<'static> {
   WebmachineDispatcher {
       routes: btreemap!{
          "/myresource" => WebmachineResource {
            // Methods allowed on this resource
            allowed_methods: vec!["OPTIONS", "GET", "HEAD", "POST"],
            // if the resource exists callback
            resource_exists: callback(&|_, _| true),
            // callback to render the response for the resource
            render_response: callback(&|_, _| {
                let json_response = json!({
                   "data": [1, 2, 3, 4]
                });
                Some(json_response.to_string())
            }),
            // callback to process the post for the resource
            process_post: callback(&|_, _|  /* Handle the post here */ Ok(true) ),
            // default everything else
            .. WebmachineResource::default()
          }
      }
   }
 }

 async fn start_server() -> Result<(), String> {
   // Create a Hyper server that delegates to the dispatcher
   let addr = "0.0.0.0:8080".parse().unwrap();
   let make_svc = make_service_fn(|_| async { Ok::<_, Infallible>(dispatcher()) });
   match Server::try_bind(&addr) {
     Ok(server) => {
       // start the actual server
       server.serve(make_svc).await;
       Ok(())
     },
     Err(err) => {
       error!("could not start server: {}", err);
       Err(format!("could not start server: {}", err))
     }
   }
 }
 ```

## Example implementations

For an example of a project using this crate, have a look at the [Pact Mock Server](https://github.com/pact-foundation/pact-reference/tree/master/rust/v1/pact_mock_server_cli) from the Pact reference implementation.
