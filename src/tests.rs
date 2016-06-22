use super::*;
use super::sanitise_path;
use super::{
    execute_state_machine,
    update_paths_for_resource,
    parse_header_values,
    finalise_response,
    join_paths
};
use super::context::*;
use super::headers::*;
use expectest::prelude::*;
use std::collections::HashMap;
use chrono::*;

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

#[test]
fn execute_state_machine_sets_content_type_header_if_the_request_does_have_an_acceptable_content_type() {
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
        produces: vec![s!("application/xml")],
        .. WebmachineResource::default()
    };
    execute_state_machine(&mut context, &resource);
    finalise_response(&mut context, &resource);
    expect(context.response.status).to(be_equal_to(200));
    expect(context.response.headers).to(be_equal_to(btreemap!{ s!("Content-Type") => vec![h!("application/xml;charset=ISO-8859-1")] }));
}

#[test]
fn execute_state_machine_returns_406_if_the_request_does_not_have_an_acceptable_language() {
    let mut context = WebmachineContext {
        request: WebmachineRequest {
            headers: hashmap!{
                s!("Accept-Language") => vec![HeaderValue::basic(&s!("da"))]
            },
            .. WebmachineRequest::default()
        },
        .. WebmachineContext::default()
    };
    let resource = WebmachineResource {
        languages_provided: vec![s!("en")],
        .. WebmachineResource::default()
    };
    execute_state_machine(&mut context, &resource);
    expect(context.response.status).to(be_equal_to(406));
}

#[test]
fn execute_state_machine_sets_the_language_header_if_the_request_does_have_an_acceptable_language() {
    let mut context = WebmachineContext {
        request: WebmachineRequest {
            headers: hashmap!{
                s!("Accept-Language") => vec![HeaderValue::basic(&s!("en-gb"))]
            },
            .. WebmachineRequest::default()
        },
        .. WebmachineContext::default()
    };
    let resource = WebmachineResource {
        languages_provided: vec![s!("en")],
        .. WebmachineResource::default()
    };
    execute_state_machine(&mut context, &resource);
    expect(context.response.status).to(be_equal_to(200));
    expect(context.response.headers).to(be_equal_to(btreemap!{ s!("Content-Language") => vec![h!("en")] }));
}

#[test]
fn execute_state_machine_returns_406_if_the_request_does_not_have_an_acceptable_charset() {
    let mut context = WebmachineContext {
        request: WebmachineRequest {
            headers: hashmap!{
                s!("Accept-Charset") => vec![h!("iso-8859-5"), h!("iso-8859-1;q=0")]
            },
            .. WebmachineRequest::default()
        },
        .. WebmachineContext::default()
    };
    let resource = WebmachineResource {
        charsets_provided: vec![s!("UTF-8"), s!("US-ASCII")],
        .. WebmachineResource::default()
    };
    execute_state_machine(&mut context, &resource);
    expect(context.response.status).to(be_equal_to(406));
}

#[test]
fn execute_state_machine_sets_the_charset_if_the_request_does_have_an_acceptable_charset() {
    let mut context = WebmachineContext {
        request: WebmachineRequest {
            headers: hashmap!{
                s!("Accept-Charset") => vec![h!("UTF-8"), h!("iso-8859-1;q=0")]
            },
            .. WebmachineRequest::default()
        },
        .. WebmachineContext::default()
    };
    let resource = WebmachineResource {
        charsets_provided: vec![s!("UTF-8"), s!("US-ASCII")],
        .. WebmachineResource::default()
    };
    execute_state_machine(&mut context, &resource);
    finalise_response(&mut context, &resource);
    expect(context.response.status).to(be_equal_to(200));
    expect(context.response.headers).to(be_equal_to(btreemap!{ s!("Content-Type") => vec![h!("application/json;charset=UTF-8")] }));
}

#[test]
fn execute_state_machine_returns_406_if_the_request_does_not_have_an_acceptable_encoding() {
    let mut context = WebmachineContext {
        request: WebmachineRequest {
            headers: hashmap!{
                s!("Accept-Encoding") => vec![h!("compress"), h!("*;q=0")]
            },
            .. WebmachineRequest::default()
        },
        .. WebmachineContext::default()
    };
    let resource = WebmachineResource {
        encodings_provided: vec![s!("identity")],
        .. WebmachineResource::default()
    };
    execute_state_machine(&mut context, &resource);
    expect(context.response.status).to(be_equal_to(406));
}

#[test]
fn execute_state_machine_sets_the_vary_header_if_the_resource_has_variances() {
    let mut context = WebmachineContext {
        request: WebmachineRequest {
            .. WebmachineRequest::default()
        },
        .. WebmachineContext::default()
    };
    let resource = WebmachineResource {
        variances: vec![s!("HEADER-A"), s!("HEADER-B")],
        .. WebmachineResource::default()
    };
    execute_state_machine(&mut context, &resource);
    finalise_response(&mut context, &resource);
    expect(context.response.status).to(be_equal_to(200));
    expect(context.response.headers).to(be_equal_to(btreemap!{
        s!("Content-Type") => vec![h!("application/json;charset=ISO-8859-1")],
        s!("Vary") => vec![h!("HEADER-A"), h!("HEADER-B")]
    }));
}

#[test]
fn execute_state_machine_returns_404_if_the_resource_does_not_exist() {
    let mut context = WebmachineContext {
        request: WebmachineRequest {
            .. WebmachineRequest::default()
        },
        .. WebmachineContext::default()
    };
    let resource = WebmachineResource {
        resource_exists: Box::new(|_| false),
        .. WebmachineResource::default()
    };
    execute_state_machine(&mut context, &resource);
    expect(context.response.status).to(be_equal_to(404));
}

#[test]
fn execute_state_machine_returns_412_if_the_resource_does_not_exist_and_there_is_an_if_match_header() {
    let mut context = WebmachineContext {
        request: WebmachineRequest {
            headers: hashmap!{
                s!("If-Match") => vec![h!("*")]
            },
            .. WebmachineRequest::default()
        },
        .. WebmachineContext::default()
    };
    let resource = WebmachineResource {
        resource_exists: Box::new(|_| false),
        .. WebmachineResource::default()
    };
    execute_state_machine(&mut context, &resource);
    expect(context.response.status).to(be_equal_to(412));
}

#[test]
fn execute_state_machine_returns_301_and_sets_location_header_if_the_resource_has_moved_permanently() {
    let mut context = WebmachineContext {
        request: WebmachineRequest {
            method: s!("PUT"),
            .. WebmachineRequest::default()
        },
        .. WebmachineContext::default()
    };
    let resource = WebmachineResource {
        allowed_methods: vec![s!("PUT")],
        resource_exists: Box::new(|_| false),
        moved_permanently: Box::new(|_| Some(s!("http://go.away.com/to/here"))),
        .. WebmachineResource::default()
    };
    execute_state_machine(&mut context, &resource);
    expect(context.response.status).to(be_equal_to(301));
    expect(context.response.headers).to(be_equal_to(btreemap!{
        s!("Location") => vec![h!("http://go.away.com/to/here")]
    }));
}

#[test]
fn execute_state_machine_returns_409_if_the_put_request_is_a_conflict() {
    let mut context = WebmachineContext {
        request: WebmachineRequest {
            method: s!("PUT"),
            .. WebmachineRequest::default()
        },
        .. WebmachineContext::default()
    };
    let resource = WebmachineResource {
        allowed_methods: vec![s!("PUT")],
        resource_exists: Box::new(|_| false),
        is_conflict: Box::new(|_| true),
        .. WebmachineResource::default()
    };
    execute_state_machine(&mut context, &resource);
    expect(context.response.status).to(be_equal_to(409));
}

#[test]
fn execute_state_machine_returns_404_if_the_resource_does_not_exist_and_does_not_except_posts_to_nonexistant_resources() {
    let mut context = WebmachineContext {
        request: WebmachineRequest {
            method: s!("POST"),
            .. WebmachineRequest::default()
        },
        .. WebmachineContext::default()
    };
    let resource = WebmachineResource {
        allowed_methods: vec![s!("POST")],
        resource_exists: Box::new(|_| false),
        allow_missing_post: Box::new(|_| false),
        .. WebmachineResource::default()
    };
    execute_state_machine(&mut context, &resource);
    expect(context.response.status).to(be_equal_to(404));
}

#[test]
fn execute_state_machine_returns_301_and_sets_location_header_if_the_resource_has_moved_permanently_and_prev_existed_and_not_a_put() {
    let mut context = WebmachineContext {
        request: WebmachineRequest {
            method: s!("POST"),
            .. WebmachineRequest::default()
        },
        .. WebmachineContext::default()
    };
    let resource = WebmachineResource {
        allowed_methods: vec![s!("POST")],
        resource_exists: Box::new(|_| false),
        previously_existed: Box::new(|_| true),
        moved_permanently: Box::new(|_| Some(s!("http://go.away.com/to/here"))),
        .. WebmachineResource::default()
    };
    execute_state_machine(&mut context, &resource);
    expect(context.response.status).to(be_equal_to(301));
    expect(context.response.headers).to(be_equal_to(btreemap!{
        s!("Location") => vec![h!("http://go.away.com/to/here")]
    }));
}

#[test]
fn execute_state_machine_returns_307_and_sets_location_header_if_the_resource_has_moved_temporarily_and_not_a_put() {
    let mut context = WebmachineContext {
        request: WebmachineRequest {
            .. WebmachineRequest::default()
        },
        .. WebmachineContext::default()
    };
    let resource = WebmachineResource {
        resource_exists: Box::new(|_| false),
        previously_existed: Box::new(|_| true),
        moved_temporarily: Box::new(|_| Some(s!("http://go.away.com/to/here"))),
        .. WebmachineResource::default()
    };
    execute_state_machine(&mut context, &resource);
    expect(context.response.status).to(be_equal_to(307));
    expect(context.response.headers).to(be_equal_to(btreemap!{
        s!("Location") => vec![h!("http://go.away.com/to/here")]
    }));
}

#[test]
fn execute_state_machine_returns_410_if_the_resource_has_prev_existed_and_not_a_post() {
    let mut context = WebmachineContext {
        request: WebmachineRequest {
            .. WebmachineRequest::default()
        },
        .. WebmachineContext::default()
    };
    let resource = WebmachineResource {
        resource_exists: Box::new(|_| false),
        previously_existed: Box::new(|_| true),
        .. WebmachineResource::default()
    };
    execute_state_machine(&mut context, &resource);
    expect(context.response.status).to(be_equal_to(410));
}

#[test]
fn execute_state_machine_returns_410_if_the_resource_has_prev_existed_and_a_post_and_posts_to_missing_resource_not_allowed() {
    let mut context = WebmachineContext {
        request: WebmachineRequest {
            method: s!("POST"),
            .. WebmachineRequest::default()
        },
        .. WebmachineContext::default()
    };
    let resource = WebmachineResource {
        allowed_methods: vec![s!("POST")],
        resource_exists: Box::new(|_| false),
        previously_existed: Box::new(|_| true),
        allow_missing_post: Box::new(|_| false),
        .. WebmachineResource::default()
    };
    execute_state_machine(&mut context, &resource);
    expect(context.response.status).to(be_equal_to(410));
}

#[test]
fn execute_state_machine_returns_404_if_the_resource_has_not_prev_existed_and_a_post_and_posts_to_missing_resource_not_allowed() {
    let mut context = WebmachineContext {
        request: WebmachineRequest {
            method: s!("POST"),
            .. WebmachineRequest::default()
        },
        .. WebmachineContext::default()
    };
    let resource = WebmachineResource {
        allowed_methods: vec![s!("POST")],
        resource_exists: Box::new(|_| false),
        previously_existed: Box::new(|_| false),
        allow_missing_post: Box::new(|_| false),
        .. WebmachineResource::default()
    };
    execute_state_machine(&mut context, &resource);
    expect(context.response.status).to(be_equal_to(404));
}

#[test]
fn execute_state_machine_returns_412_if_the_resource_etag_does_not_match_if_match_header() {
    let mut context = WebmachineContext {
        request: WebmachineRequest {
            headers: hashmap!{
                s!("If-Match") => vec![h!("\"1234567891\"")]
            },
            .. WebmachineRequest::default()
        },
        .. WebmachineContext::default()
    };
    let resource = WebmachineResource {
        resource_exists: Box::new(|_| true),
        generate_etag: Box::new(|_| Some(s!("1234567890"))),
        .. WebmachineResource::default()
    };
    execute_state_machine(&mut context, &resource);
    expect(context.response.status).to(be_equal_to(412));
}

#[test]
fn execute_state_machine_returns_412_if_the_resource_etag_does_not_match_if_match_header_weak_etag() {
    let mut context = WebmachineContext {
        request: WebmachineRequest {
            headers: hashmap!{
                s!("If-Match") => vec![h!("W/\"1234567891\"")]
            },
            .. WebmachineRequest::default()
        },
        .. WebmachineContext::default()
    };
    let resource = WebmachineResource {
        resource_exists: Box::new(|_| true),
        generate_etag: Box::new(|_| Some(s!("1234567890"))),
        .. WebmachineResource::default()
    };
    execute_state_machine(&mut context, &resource);
    expect(context.response.status).to(be_equal_to(412));
}

#[test]
fn execute_state_machine_returns_412_if_the_resource_last_modified_gt_unmodified_since() {
    let datetime = Local::now().with_timezone(&FixedOffset::east(10 * 3600));
    let header_datetime = datetime - Duration::minutes(5);
    let mut context = WebmachineContext {
        request: WebmachineRequest {
            headers: hashmap!{
                s!("If-Unmodified-Since") => vec![h!(format!("\"{}\"", header_datetime.to_rfc2822()))]
            },
            .. WebmachineRequest::default()
        },
        .. WebmachineContext::default()
    };
    let resource = WebmachineResource {
        resource_exists: Box::new(|_| true),
        last_modified: Box::new(move |_| Some(datetime)),
        .. WebmachineResource::default()
    };
    execute_state_machine(&mut context, &resource);
    expect(context.response.status).to(be_equal_to(412));
}

#[test]
fn execute_state_machine_returns_304_if_non_match_star_exists_and_is_not_a_head_or_get() {
    let mut context = WebmachineContext {
        request: WebmachineRequest {
            method: s!("POST"),
            headers: hashmap!{
                s!("If-None-Match") => vec![h!("*")]
            },
            .. WebmachineRequest::default()
        },
        .. WebmachineContext::default()
    };
    let resource = WebmachineResource {
        resource_exists: Box::new(|_| true),
        allowed_methods: vec![s!("POST")],
        .. WebmachineResource::default()
    };
    execute_state_machine(&mut context, &resource);
    expect(context.response.status).to(be_equal_to(412));
}

#[test]
fn execute_state_machine_returns_304_if_non_match_star_exists_and_is_a_head_or_get() {
    let mut context = WebmachineContext {
        request: WebmachineRequest {
            method: s!("HEAD"),
            headers: hashmap!{
                s!("If-None-Match") => vec![h!("*")]
            },
            .. WebmachineRequest::default()
        },
        .. WebmachineContext::default()
    };
    let resource = WebmachineResource {
        resource_exists: Box::new(|_| true),
        allowed_methods: vec![s!("HEAD")],
        .. WebmachineResource::default()
    };
    execute_state_machine(&mut context, &resource);
    expect(context.response.status).to(be_equal_to(304));
}

#[test]
fn execute_state_machine_returns_412_if_resource_etag_in_if_non_match_and_is_not_a_head_or_get() {
    let mut context = WebmachineContext {
        request: WebmachineRequest {
            method: s!("POST"),
            headers: hashmap!{
                s!("If-None-Match") => vec![h!("W/\"1234567890\""), h!("W/\"1234567891\"")]
            },
            .. WebmachineRequest::default()
        },
        .. WebmachineContext::default()
    };
    let resource = WebmachineResource {
        resource_exists: Box::new(|_| true),
        allowed_methods: vec![s!("POST")],
        generate_etag: Box::new(|_| Some(s!("1234567890"))),
        .. WebmachineResource::default()
    };
    execute_state_machine(&mut context, &resource);
    expect(context.response.status).to(be_equal_to(412));
}

#[test]
fn execute_state_machine_returns_304_if_resource_etag_in_if_non_match_and_is_a_head_or_get() {
    let mut context = WebmachineContext {
        request: WebmachineRequest {
            headers: hashmap!{
                s!("If-None-Match") => vec![h!("\"1234567890\""), h!("\"1234567891\"")]
            },
            .. WebmachineRequest::default()
        },
        .. WebmachineContext::default()
    };
    let resource = WebmachineResource {
        resource_exists: Box::new(|_| true),
        generate_etag: Box::new(|_| Some(s!("1234567890"))),
        .. WebmachineResource::default()
    };
    execute_state_machine(&mut context, &resource);
    expect(context.response.status).to(be_equal_to(304));
}

#[test]
fn execute_state_machine_returns_304_if_the_resource_last_modified_gt_modified_since() {
    let datetime = Local::now().with_timezone(&FixedOffset::east(10 * 3600)) - Duration::minutes(15);
    let header_datetime = datetime + Duration::minutes(5);
    let mut context = WebmachineContext {
        request: WebmachineRequest {
            headers: hashmap!{
                s!("If-Modified-Since") => vec![h!(format!("\"{}\"", header_datetime.to_rfc2822()))]
            },
            .. WebmachineRequest::default()
        },
        .. WebmachineContext::default()
    };
    let resource = WebmachineResource {
        resource_exists: Box::new(|_| true),
        last_modified: Box::new(move |_| Some(datetime)),
        .. WebmachineResource::default()
    };
    execute_state_machine(&mut context, &resource);
    expect(context.response.status).to(be_equal_to(304));
}

#[test]
fn execute_state_machine_returns_202_if_delete_was_not_enacted() {
    let mut context = WebmachineContext {
        request: WebmachineRequest {
            method: s!("DELETE"),
            .. WebmachineRequest::default()
        },
        .. WebmachineContext::default()
    };
    let resource = WebmachineResource {
        resource_exists: Box::new(|_| true),
        delete_resource: Box::new(|_| Ok(false)),
        allowed_methods: vec![s!("DELETE")],
        .. WebmachineResource::default()
    };
    execute_state_machine(&mut context, &resource);
    expect(context.response.status).to(be_equal_to(202));
}

#[test]
fn execute_state_machine_returns_a_resource_status_code_if_delete_fails() {
    let mut context = WebmachineContext {
        request: WebmachineRequest {
            method: s!("DELETE"),
            .. WebmachineRequest::default()
        },
        .. WebmachineContext::default()
    };
    let resource = WebmachineResource {
        resource_exists: Box::new(|_| true),
        delete_resource: Box::new(|_| Err(500)),
        allowed_methods: vec![s!("DELETE")],
        .. WebmachineResource::default()
    };
    execute_state_machine(&mut context, &resource);
    expect(context.response.status).to(be_equal_to(500));
}

#[test]
fn join_paths_test() {
    expect!(join_paths(&Vec::new(), &Vec::new())).to(be_equal_to(s!("/")));
    expect!(join_paths(&vec![s!("")], &Vec::new())).to(be_equal_to(s!("/")));
    expect!(join_paths(&Vec::new(), &vec![s!("")])).to(be_equal_to(s!("/")));
    expect!(join_paths(&vec![s!("a"), s!("b"), s!("c")], &Vec::new())).to(be_equal_to(s!("/a/b/c")));
    expect!(join_paths(&vec![s!("a"), s!("b"), s!("")], &Vec::new())).to(be_equal_to(s!("/a/b")));
    expect!(join_paths(&Vec::new(), &vec![s!("a"), s!("b"), s!("c")])).to(be_equal_to(s!("/a/b/c")));
    expect!(join_paths(&vec![s!("a"), s!("b"), s!("c")], &vec![s!("d"), s!("e"), s!("f")])).to(be_equal_to(s!("/a/b/c/d/e/f")));
}

#[test]
fn execute_state_machine_returns_a_resource_status_code_if_post_fails_and_post_is_create() {
    let mut context = WebmachineContext {
        request: WebmachineRequest {
            method: s!("POST"),
            .. WebmachineRequest::default()
        },
        .. WebmachineContext::default()
    };
    let resource = WebmachineResource {
        resource_exists: Box::new(|_| true),
        post_is_create: Box::new(|_| true),
        create_path: Box::new(|_| Err(500)),
        allowed_methods: vec![s!("POST")],
        .. WebmachineResource::default()
    };
    execute_state_machine(&mut context, &resource);
    expect(context.response.status).to(be_equal_to(500));
}

#[test]
fn execute_state_machine_returns_a_resource_status_code_if_post_fails_and_post_is_not_create() {
    let mut context = WebmachineContext {
        request: WebmachineRequest {
            method: s!("POST"),
            .. WebmachineRequest::default()
        },
        .. WebmachineContext::default()
    };
    let resource = WebmachineResource {
        resource_exists: Box::new(|_| true),
        post_is_create: Box::new(|_| false),
        process_post: Box::new(|_| Err(500)),
        allowed_methods: vec![s!("POST")],
        .. WebmachineResource::default()
    };
    execute_state_machine(&mut context, &resource);
    expect(context.response.status).to(be_equal_to(500));
}

#[test]
fn execute_state_machine_returns_303_and_post_is_create_and_redirect_is_set() {
    let mut context = WebmachineContext {
        request: WebmachineRequest {
            method: s!("POST"),
            base_path: s!("/base/path"),
            .. WebmachineRequest::default()
        },
        .. WebmachineContext::default()
    };
    let resource = WebmachineResource {
        resource_exists: Box::new(|_| true),
        post_is_create: Box::new(|_| true),
        create_path: Box::new(|context| { context.redirect = true; Ok(s!("/new/path")) }),
        allowed_methods: vec![s!("POST")],
        .. WebmachineResource::default()
    };
    execute_state_machine(&mut context, &resource);
    expect(context.response.status).to(be_equal_to(303));
    expect(context.response.headers).to(be_equal_to(btreemap!{
        s!("Location") => vec![h!("/base/path/new/path")]
    }));
}

#[test]
fn execute_state_machine_returns_303_if_post_is_not_create_and_redirect_is_set() {
    let mut context = WebmachineContext {
        request: WebmachineRequest {
            method: s!("POST"),
            .. WebmachineRequest::default()
        },
        .. WebmachineContext::default()
    };
    let resource = WebmachineResource {
        resource_exists: Box::new(|_| true),
        post_is_create: Box::new(|_| false),
        process_post: Box::new(|context| { context.redirect = true; Ok(true) }),
        allowed_methods: vec![s!("POST")],
        .. WebmachineResource::default()
    };
    execute_state_machine(&mut context, &resource);
    expect(context.response.status).to(be_equal_to(303));
}

#[test]
fn execute_state_machine_returns_303_if_post_to_missing_resource_and_redirect_is_set() {
    let mut context = WebmachineContext {
        request: WebmachineRequest {
            method: s!("POST"),
            .. WebmachineRequest::default()
        },
        .. WebmachineContext::default()
    };
    let resource = WebmachineResource {
        resource_exists: Box::new(|_| false),
        previously_existed: Box::new(|_| false),
        allow_missing_post: Box::new(|_| true),
        post_is_create: Box::new(|_| false),
        process_post: Box::new(|context| { context.redirect = true; Ok(true) }),
        allowed_methods: vec![s!("POST")],
        .. WebmachineResource::default()
    };
    execute_state_machine(&mut context, &resource);
    expect(context.response.status).to(be_equal_to(303));
}
