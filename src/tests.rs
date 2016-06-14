use super::*;
use super::sanitise_path;
use super::{execute_state_machine, update_paths_for_resource, parse_header_values};
use super::context::*;
use super::headers::*;
use expectest::prelude::*;
use std::collections::HashMap;

fn resource(path: &str) -> WebmachineRequest {
    WebmachineRequest {
        request_path: s!(path),
        base_path: s!("/"),
        method: s!("GET"),
        headers: HashMap::new()
    }
}

#[test]
fn path_matcher_test() {
    let dispatcher = WebmachineDispatcher {
        routes: btreemap!{
            s!("/") => WebmachineResource::default(),
            s!("/path1") => WebmachineResource::default(),
            s!("/path2") => WebmachineResource::default(),
            s!("/path1/path3") => WebmachineResource::default()
        }
    };
    expect!(dispatcher.match_paths(&resource("/path1"))).to(be_equal_to(vec!["/", "/path1"]));
    expect!(dispatcher.match_paths(&resource("/path1/"))).to(be_equal_to(vec!["/", "/path1"]));
    expect!(dispatcher.match_paths(&resource("/path1/path3"))).to(be_equal_to(vec!["/", "/path1", "/path1/path3"]));
    expect!(dispatcher.match_paths(&resource("/path1/path3/path4"))).to(be_equal_to(vec!["/", "/path1", "/path1/path3"]));
    expect!(dispatcher.match_paths(&resource("/path1/other"))).to(be_equal_to(vec!["/", "/path1"]));
    expect!(dispatcher.match_paths(&resource("/path12"))).to(be_equal_to(vec!["/"]));
    expect!(dispatcher.match_paths(&resource("/"))).to(be_equal_to(vec!["/"]));
}

#[test]
fn sanitise_path_test() {
    expect!(sanitise_path(&"/".to_string()).iter()).to(be_empty());
    expect!(sanitise_path(&"//".to_string()).iter()).to(be_empty());
    expect!(sanitise_path(&"/a/b/c".to_string())).to(be_equal_to(vec!["a", "b", "c"]));
    expect!(sanitise_path(&"/a/b/c/".to_string())).to(be_equal_to(vec!["a", "b", "c"]));
    expect!(sanitise_path(&"/a//b/c".to_string())).to(be_equal_to(vec!["a", "b", "c"]));
}

#[test]
fn dispatcher_returns_404_if_there_is_no_matching_resource() {
    let mut context = WebmachineContext::default();
    let displatcher = WebmachineDispatcher {
        routes: btreemap!{ s!("/some/path") => WebmachineResource::default() }
    };
    displatcher.dispatch_to_resource(&mut context);
    expect(context.response.status).to(be_equal_to(404));
}

#[test]
fn execute_state_machine_returns_503_if_resource_indicates_not_available() {
    let mut context = WebmachineContext::default();
    let resource = WebmachineResource {
        available: Box::new(|_| { false }),
        .. WebmachineResource::default()
    };
    execute_state_machine(&mut context, &resource);
    expect(context.response.status).to(be_equal_to(503));
}

#[test]
fn update_paths_for_resource_test_with_root() {
    let mut request = WebmachineRequest::default();
    update_paths_for_resource(&mut request, &s!("/"));
    expect(request.request_path).to(be_equal_to(s!("/")));
    expect(request.base_path).to(be_equal_to(s!("/")));
}

#[test]
fn update_paths_for_resource_test_with_subpath() {
    let mut request = WebmachineRequest {
        request_path: s!("/subpath"),
        .. WebmachineRequest::default()
    };
    update_paths_for_resource(&mut request, &s!("/"));
    expect(request.request_path).to(be_equal_to(s!("/subpath")));
    expect(request.base_path).to(be_equal_to(s!("/")));
}

#[test]
fn update_paths_for_resource_on_path() {
    let mut request = WebmachineRequest {
        request_path: s!("/path"),
        .. WebmachineRequest::default()
    };
    update_paths_for_resource(&mut request, &s!("/path"));
    expect(request.request_path).to(be_equal_to(s!("/")));
    expect(request.base_path).to(be_equal_to(s!("/path")));
}

#[test]
fn update_paths_for_resource_on_path_with_subpath() {
    let mut request = WebmachineRequest {
        request_path: s!("/path/path2"),
        .. WebmachineRequest::default()
    };
    update_paths_for_resource(&mut request, &s!("/path"));
    expect(request.request_path).to(be_equal_to(s!("/path2")));
    expect(request.base_path).to(be_equal_to(s!("/path")));
}

#[test]
fn execute_state_machine_returns_501_if_method_is_not_in_known_list() {
    let mut context = WebmachineContext {
        request: WebmachineRequest {
            method: s!("Blah"),
            .. WebmachineRequest::default()
        },
        .. WebmachineContext::default()
    };
    let resource = WebmachineResource::default();
    execute_state_machine(&mut context, &resource);
    expect(context.response.status).to(be_equal_to(501));
}

#[test]
fn execute_state_machine_returns_414_if_uri_is_too_long() {
    let mut context = WebmachineContext::default();
    let resource = WebmachineResource {
        uri_too_long: Box::new(|_| true),
        .. WebmachineResource::default()
    };
    execute_state_machine(&mut context, &resource);
    expect(context.response.status).to(be_equal_to(414));
}

#[test]
fn execute_state_machine_returns_405_if_method_is_not_allowed() {
    let mut context = WebmachineContext {
        request: WebmachineRequest {
            method: s!("TRACE"),
            .. WebmachineRequest::default()
        },
        .. WebmachineContext::default()
    };
    let resource = WebmachineResource::default();
    execute_state_machine(&mut context, &resource);
    expect(context.response.status).to(be_equal_to(405));
    expect(context.response.headers.get(&s!("Allow")).unwrap().clone()).to(be_equal_to(vec![
        HeaderValue::basic(&s!("OPTIONS")),
        HeaderValue::basic(&s!("GET")),
        HeaderValue::basic(&s!("HEAD"))
    ]));
}

#[test]
fn execute_state_machine_returns_400_if_malformed_request() {
    let mut context = WebmachineContext::default();
    let resource = WebmachineResource {
        malformed_request: Box::new(|_| true),
        .. WebmachineResource::default()
    };
    execute_state_machine(&mut context, &resource);
    expect(context.response.status).to(be_equal_to(400));
}

#[test]
fn execute_state_machine_returns_401_if_not_authorized() {
    let mut context = WebmachineContext::default();
    let resource = WebmachineResource {
        not_authorized: Box::new(|_| Some(s!("Basic realm=\"User Visible Realm\""))),
        .. WebmachineResource::default()
    };
    execute_state_machine(&mut context, &resource);
    expect(context.response.status).to(be_equal_to(401));
    expect(context.response.headers.get(&s!("WWW-Authenticate")).unwrap().clone()).to(be_equal_to(vec![
        HeaderValue::basic(&s!("Basic realm=\"User Visible Realm\""))
    ]));
}

#[test]
fn execute_state_machine_returns_403_if_forbidden() {
    let mut context = WebmachineContext::default();
    let resource = WebmachineResource {
        forbidden: Box::new(|_| true),
        .. WebmachineResource::default()
    };
    execute_state_machine(&mut context, &resource);
    expect(context.response.status).to(be_equal_to(403));
}

#[test]
fn execute_state_machine_returns_501_if_there_is_an_unsupported_content_header() {
    let mut context = WebmachineContext::default();
    let resource = WebmachineResource {
        unsupported_content_headers: Box::new(|_| true),
        .. WebmachineResource::default()
    };
    execute_state_machine(&mut context, &resource);
    expect(context.response.status).to(be_equal_to(501));
}

#[test]
fn execute_state_machine_returns_415_if_the_content_type_is_unknown() {
    let mut context = WebmachineContext {
        request: WebmachineRequest {
            method: s!("POST"),
            headers: hashmap!{
                s!("Content-type") => vec![HeaderValue::basic(&s!("application/xml"))]
            },
            .. WebmachineRequest::default()
        },
        .. WebmachineContext::default()
    };
    let resource = WebmachineResource {
        acceptable_content_types: vec![s!("application/json")],
        allowed_methods: vec![s!("POST")],
        .. WebmachineResource::default()
    };
    execute_state_machine(&mut context, &resource);
    expect(context.response.status).to(be_equal_to(415));
}

#[test]
fn execute_state_machine_returns_does_not_return_415_if_not_a_put_or_post() {
    let mut context = WebmachineContext {
        request: WebmachineRequest {
            headers: hashmap!{
                s!("Content-type") => vec![HeaderValue::basic(&s!("application/xml"))]
            },
            .. WebmachineRequest::default()
        },
        .. WebmachineContext::default()
    };
    let resource = WebmachineResource {
        .. WebmachineResource::default()
    };
    execute_state_machine(&mut context, &resource);
    expect(context.response.status).to_not(be_equal_to(415));
}

#[test]
fn parse_header_test() {
    expect(parse_header_values(s!("")).iter()).to(be_empty());
    expect(parse_header_values(s!("HEADER A"))).to(be_equal_to(vec![s!("HEADER A")]));
    expect(parse_header_values(s!("HEADER A, header B")))
        .to(be_equal_to(vec![s!("HEADER A"), s!("header B")]));
    expect(parse_header_values(s!("text/plain;  q=0.5,   text/html,text/x-dvi; q=0.8, text/x-c")))
        .to(be_equal_to(vec![
            HeaderValue { value: s!("text/plain"), params: hashmap!{s!("q") => s!("0.5")} },
            HeaderValue { value: s!("text/html"), params: hashmap!{} },
            HeaderValue { value: s!("text/x-dvi"), params: hashmap!{s!("q") => s!("0.8")} },
            HeaderValue { value: s!("text/x-c"), params: hashmap!{} }
        ]));
}

#[test]
fn execute_state_machine_returns_413_if_the_request_entity_is_too_large() {
    let mut context = WebmachineContext {
        request: WebmachineRequest {
            method: s!("POST"),
            .. WebmachineRequest::default()
        },
        .. WebmachineContext::default()
    };
    let resource = WebmachineResource {
        valid_entity_length: Box::new(|_| false),
        allowed_methods: vec![s!("POST")],
        .. WebmachineResource::default()
    };
    execute_state_machine(&mut context, &resource);
    expect(context.response.status).to(be_equal_to(413));
}

#[test]
fn execute_state_machine_returns_does_not_return_413_if_not_a_put_or_post() {
    let mut context = WebmachineContext {
        request: WebmachineRequest {
            .. WebmachineRequest::default()
        },
        .. WebmachineContext::default()
    };
    let resource = WebmachineResource {
        valid_entity_length: Box::new(|_| false),
        .. WebmachineResource::default()
    };
    execute_state_machine(&mut context, &resource);
    expect(context.response.status).to_not(be_equal_to(413));
}

#[test]
fn execute_state_machine_returns_headers_for_option_request() {
    let mut context = WebmachineContext {
        request: WebmachineRequest {
            method: s!("OPTIONS"),
            .. WebmachineRequest::default()
        },
        .. WebmachineContext::default()
    };
    let resource = WebmachineResource {
        allowed_methods: vec![s!("OPTIONS")],
        options: Box::new(|_, _| Some(hashmap!{
            s!("A") => vec![s!("B")],
            s!("C") => vec![s!("D;E=F")],
        })),
        .. WebmachineResource::default()
    };
    execute_state_machine(&mut context, &resource);
    expect(context.response.status).to(be_equal_to(200));
    expect(context.response.headers.get(&s!("A")).unwrap().clone()).to(be_equal_to(vec![s!("B")]));
    expect(context.response.headers.get(&s!("C")).unwrap().clone()).to(be_equal_to(vec![s!("D;E=F")]));
}

#[test]
fn execute_state_machine_returns_406_if_the_request_does_not_have_an_acceptable_content_type() {
    let mut context = WebmachineContext {
        request: WebmachineRequest {
            headers: hashmap!{
                s!("Accept") => vec![HeaderValue::basic(&s!("application/xml"))]
            },
            .. WebmachineRequest::default()
        },
        .. WebmachineContext::default()
    };
    let resource = WebmachineResource {
        produces: vec![s!("application/javascript")],
        .. WebmachineResource::default()
    };
    execute_state_machine(&mut context, &resource);
    expect(context.response.status).to(be_equal_to(406));
}
