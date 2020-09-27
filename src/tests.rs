use super::*;
use super::sanitise_path;
use super::{
  execute_state_machine,
  update_paths_for_resource,
  parse_header_values,
  finalise_response,
  join_paths,
};
use super::context::*;
use super::headers::*;
use expectest::prelude::*;
use std::collections::HashMap;
use chrono::*;

fn resource(path: &str) -> WebmachineRequest {
  WebmachineRequest {
    request_path: path.to_string(),
    base_path: "/".to_string(),
    method: "GET".to_string(),
    headers: HashMap::new(),
    body: None,
    query: HashMap::new(),
  }
}

#[test]
fn path_matcher_test() {
  let dispatcher = WebmachineDispatcher {
    routes: btreemap! {
      "/" => WebmachineResource::default(),
      "/path1" => WebmachineResource::default(),
      "/path2" => WebmachineResource::default(),
      "/path1/path3" => WebmachineResource::default()
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
    routes: btreemap! { "/some/path" => WebmachineResource::default() }
  };
  displatcher.dispatch_to_resource(&mut context);
  expect(context.response.status).to(be_equal_to(404));
}

#[test]
fn execute_state_machine_returns_503_if_resource_indicates_not_available() {
  let mut context = WebmachineContext::default();
  let resource = WebmachineResource {
    available: callback(&|_, _| { false }),
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(503));
}

#[test]
fn update_paths_for_resource_test_with_root() {
  let mut request = WebmachineRequest::default();
  update_paths_for_resource(&mut request, "/");
  expect(request.request_path).to(be_equal_to("/".to_string()));
  expect(request.base_path).to(be_equal_to("/".to_string()));
}

#[test]
fn update_paths_for_resource_test_with_subpath() {
  let mut request = WebmachineRequest {
    request_path: "/subpath".to_string(),
    ..WebmachineRequest::default()
  };
  update_paths_for_resource(&mut request, "/");
  expect(request.request_path).to(be_equal_to("/subpath".to_string()));
  expect(request.base_path).to(be_equal_to("/".to_string()));
}

#[test]
fn update_paths_for_resource_on_path() {
  let mut request = WebmachineRequest {
    request_path: "/path".to_string(),
    ..WebmachineRequest::default()
  };
  update_paths_for_resource(&mut request, "/path");
  expect(request.request_path).to(be_equal_to("/".to_string()));
  expect(request.base_path).to(be_equal_to("/path".to_string()));
}

#[test]
fn update_paths_for_resource_on_path_with_subpath() {
  let mut request = WebmachineRequest {
    request_path: "/path/path2".to_string(),
    ..WebmachineRequest::default()
  };
  update_paths_for_resource(&mut request, "/path");
  expect(request.request_path).to(be_equal_to("/path2".to_string()));
  expect(request.base_path).to(be_equal_to("/path".to_string()));
}

#[test]
fn execute_state_machine_returns_501_if_method_is_not_in_known_list() {
  let mut context = WebmachineContext {
    request: WebmachineRequest {
      method: "Blah".to_string(),
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource::default();
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(501));
}

#[test]
fn execute_state_machine_returns_414_if_uri_is_too_long() {
  let mut context = WebmachineContext::default();
  let resource = WebmachineResource {
    uri_too_long: callback(&|_, _| true),
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(414));
}

#[test]
fn execute_state_machine_returns_405_if_method_is_not_allowed() {
  let mut context = WebmachineContext {
    request: WebmachineRequest {
      method: "TRACE".to_string(),
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource::default();
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(405));
  expect(context.response.headers.get("Allow").unwrap().clone()).to(be_equal_to(vec![
    HeaderValue::basic("OPTIONS"),
    HeaderValue::basic("GET"),
    HeaderValue::basic("HEAD")
  ]));
}

#[test]
fn execute_state_machine_returns_400_if_malformed_request() {
  let mut context = WebmachineContext::default();
  let resource = WebmachineResource {
    malformed_request: callback(&|_, _| true),
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(400));
}

#[test]
fn execute_state_machine_returns_401_if_not_authorized() {
  let mut context = WebmachineContext::default();
  let resource = WebmachineResource {
    not_authorized: callback(&|_, _| Some("Basic realm=\"User Visible Realm\"".to_string())),
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(401));
  expect(context.response.headers.get("WWW-Authenticate").unwrap().clone()).to(be_equal_to(vec![
    HeaderValue::basic(&"Basic realm=\"User Visible Realm\"".to_string())
  ]));
}

#[test]
fn execute_state_machine_returns_403_if_forbidden() {
  let mut context = WebmachineContext::default();
  let resource = WebmachineResource {
    forbidden: callback(&|_, _| true),
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(403));
}

#[test]
fn execute_state_machine_returns_501_if_there_is_an_unsupported_content_header() {
  let mut context = WebmachineContext::default();
  let resource = WebmachineResource {
    unsupported_content_headers: callback(&|_, _| true),
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(501));
}

#[test]
fn execute_state_machine_returns_415_if_the_content_type_is_unknown() {
  let mut context = WebmachineContext {
    request: WebmachineRequest {
      method: "POST".to_string(),
      headers: hashmap! {
        "Content-type".to_string() => vec![HeaderValue::basic(&"application/xml".to_string())]
      },
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource {
    acceptable_content_types: vec!["application/json"],
    allowed_methods: vec!["POST"],
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(415));
}

#[test]
fn execute_state_machine_returns_does_not_return_415_if_not_a_put_or_post() {
  let mut context = WebmachineContext {
    request: WebmachineRequest {
      headers: hashmap! {
        "Content-type".to_string() => vec![HeaderValue::basic(&"application/xml".to_string())]
      },
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource {
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to_not(be_equal_to(415));
}

#[test]
fn parse_header_test() {
  expect(parse_header_values("").iter()).to(be_empty());
  expect(parse_header_values("HEADER A")).to(be_equal_to(vec!["HEADER A".to_string()]));
  expect(parse_header_values("HEADER A, header B"))
    .to(be_equal_to(vec!["HEADER A".to_string(), "header B".to_string()]));
  expect(parse_header_values("text/plain;  q=0.5,   text/html,text/x-dvi; q=0.8, text/x-c"))
    .to(be_equal_to(vec![
      HeaderValue { value: "text/plain".to_string(), params: hashmap! {"q".to_string() => "0.5".to_string()}, quote: false },
      HeaderValue { value: "text/html".to_string(), params: hashmap! {}, quote: false },
      HeaderValue { value: "text/x-dvi".to_string(), params: hashmap! {"q".to_string() => "0.8".to_string()}, quote: false },
      HeaderValue { value: "text/x-c".to_string(), params: hashmap! {}, quote: false }
    ]));
}

#[test]
fn execute_state_machine_returns_413_if_the_request_entity_is_too_large() {
  let mut context = WebmachineContext {
    request: WebmachineRequest {
      method: "POST".to_string(),
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource {
    valid_entity_length: callback(&|_, _| false),
    allowed_methods: vec!["POST"],
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(413));
}

#[test]
fn execute_state_machine_returns_does_not_return_413_if_not_a_put_or_post() {
  let mut context = WebmachineContext {
    request: WebmachineRequest {
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource {
    valid_entity_length: callback(&|_, _| false),
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to_not(be_equal_to(413));
}

#[test]
fn execute_state_machine_returns_headers_for_option_request() {
  let mut context = WebmachineContext {
    request: WebmachineRequest {
      method: "OPTIONS".to_string(),
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource {
    allowed_methods: vec!["OPTIONS"],
    options: callback(&|_, _| Some(hashmap! {
      "A".to_string() => vec!["B".to_string()],
      "C".to_string() => vec!["D;E=F".to_string()],
    })),
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(204));
  expect(context.response.headers.get("A").unwrap().clone()).to(be_equal_to(vec!["B".to_string()]));
  expect(context.response.headers.get("C").unwrap().clone()).to(be_equal_to(vec!["D;E=F".to_string()]));
}

#[test]
fn execute_state_machine_returns_406_if_the_request_does_not_have_an_acceptable_content_type() {
  let mut context = WebmachineContext {
    request: WebmachineRequest {
      headers: hashmap! {
        "Accept".to_string() => vec![HeaderValue::basic(&"application/xml".to_string())]
      },
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource {
    produces: vec!["application/javascript"],
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(406));
}

#[test]
fn execute_state_machine_sets_content_type_header_if_the_request_does_have_an_acceptable_content_type() {
  let mut context = WebmachineContext {
    request: WebmachineRequest {
      headers: hashmap! {
        "Accept".to_string() => vec![HeaderValue::basic(&"application/xml".to_string())]
      },
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource {
    produces: vec!["application/xml"],
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  finalise_response(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(200));
  expect(context.response.headers.get("Content-Type").unwrap()).to(be_equal_to(&vec![h!("application/xml;charset=ISO-8859-1")]));
}

#[test]
fn execute_state_machine_returns_406_if_the_request_does_not_have_an_acceptable_language() {
  let mut context = WebmachineContext {
    request: WebmachineRequest {
      headers: hashmap! {
        "Accept-Language".to_string() => vec![HeaderValue::basic(&"da".to_string())]
      },
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource {
    languages_provided: vec!["en"],
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(406));
}

#[test]
fn execute_state_machine_sets_the_language_header_if_the_request_does_have_an_acceptable_language() {
  let mut context = WebmachineContext {
    request: WebmachineRequest {
      headers: hashmap! {
        "Accept-Language".to_string() => vec![HeaderValue::basic(&"en-gb".to_string())]
      },
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource {
    languages_provided: vec!["en"],
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(200));
  expect(context.response.headers).to(be_equal_to(btreemap! { "Content-Language".to_string() => vec![h!("en")] }));
}

#[test]
fn execute_state_machine_returns_406_if_the_request_does_not_have_an_acceptable_charset() {
  let mut context = WebmachineContext {
    request: WebmachineRequest {
      headers: hashmap! {
        "Accept-Charset".to_string() => vec![h!("iso-8859-5"), h!("iso-8859-1;q=0")]
      },
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource {
    charsets_provided: vec!["UTF-8", "US-ASCII"],
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(406));
}

#[test]
fn execute_state_machine_sets_the_charset_if_the_request_does_have_an_acceptable_charset() {
  let mut context = WebmachineContext {
    request: WebmachineRequest {
      headers: hashmap! {
        "Accept-Charset".to_string() => vec![h!("UTF-8"), h!("iso-8859-1;q=0")]
      },
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource {
    charsets_provided: vec!["UTF-8", "US-ASCII"],
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  finalise_response(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(200));
  expect(context.response.headers.get("Content-Type").unwrap()).to(be_equal_to(&vec![h!("application/json;charset=UTF-8")]));
}

#[test]
fn execute_state_machine_returns_406_if_the_request_does_not_have_an_acceptable_encoding() {
  let mut context = WebmachineContext {
    request: WebmachineRequest {
      headers: hashmap! {
        "Accept-Encoding".to_string() => vec![h!("compress"), h!("*;q=0")]
      },
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource {
    encodings_provided: vec!["identity"],
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(406));
}

#[test]
fn execute_state_machine_sets_the_vary_header_if_the_resource_has_variances() {
  let mut context = WebmachineContext {
    request: WebmachineRequest {
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource {
    variances: vec!["HEADER-A", "HEADER-B"],
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  finalise_response(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(200));
  expect(context.response.headers).to(be_equal_to(btreemap! {
    "Content-Type".to_string() => vec![h!("application/json;charset=ISO-8859-1")],
    "Vary".to_string() => vec![h!("HEADER-A"), h!("HEADER-B")]
  }));
}

#[test]
fn execute_state_machine_returns_404_if_the_resource_does_not_exist() {
  let mut context = WebmachineContext {
    request: WebmachineRequest {
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource {
    resource_exists: callback(&|_, _| false),
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(404));
}

#[test]
fn execute_state_machine_returns_412_if_the_resource_does_not_exist_and_there_is_an_if_match_header() {
  let mut context = WebmachineContext {
    request: WebmachineRequest {
      headers: hashmap! {
        "If-Match".to_string() => vec![h!("*")]
      },
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource {
    resource_exists: callback(&|_, _| false),
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(412));
}

#[test]
fn execute_state_machine_returns_301_and_sets_location_header_if_the_resource_has_moved_permanently() {
  let mut context = WebmachineContext {
    request: WebmachineRequest {
      method: "PUT".to_string(),
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource {
    allowed_methods: vec!["PUT"],
    resource_exists: callback(&|_, _| false),
    moved_permanently: callback(&|_, _| Some("http://go.away.com/to/here".to_string())),
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(301));
  expect(context.response.headers).to(be_equal_to(btreemap! {
    "Location".to_string() => vec![h!("http://go.away.com/to/here")]
  }));
}

#[test]
fn execute_state_machine_returns_409_if_the_put_request_is_a_conflict() {
  let mut context = WebmachineContext {
    request: WebmachineRequest {
      method: "PUT".to_string(),
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource {
    allowed_methods: vec!["PUT"],
    resource_exists: callback(&|_, _| false),
    is_conflict: callback(&|_, _| true),
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(409));
}

#[test]
fn execute_state_machine_returns_404_if_the_resource_does_not_exist_and_does_not_except_posts_to_nonexistant_resources() {
  let mut context = WebmachineContext {
    request: WebmachineRequest {
      method: "POST".to_string(),
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource {
    allowed_methods: vec!["POST"],
    resource_exists: callback(&|_, _| false),
    allow_missing_post: callback(&|_, _| false),
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(404));
}

#[test]
fn execute_state_machine_returns_301_and_sets_location_header_if_the_resource_has_moved_permanently_and_prev_existed_and_not_a_put() {
  let mut context = WebmachineContext {
    request: WebmachineRequest {
      method: "POST".to_string(),
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource {
    allowed_methods: vec!["POST"],
    resource_exists: callback(&|_, _| false),
    previously_existed: callback(&|_, _| true),
    moved_permanently: callback(&|_, _| Some("http://go.away.com/to/here".to_string())),
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(301));
  expect(context.response.headers).to(be_equal_to(btreemap! {
    "Location".to_string() => vec![h!("http://go.away.com/to/here")]
  }));
}

#[test]
fn execute_state_machine_returns_307_and_sets_location_header_if_the_resource_has_moved_temporarily_and_not_a_put() {
  let mut context = WebmachineContext {
    request: WebmachineRequest {
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource {
    resource_exists: callback(&|_, _| false),
    previously_existed: callback(&|_, _| true),
    moved_temporarily: callback(&|_, _| Some("http://go.away.com/to/here".to_string())),
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(307));
  expect(context.response.headers).to(be_equal_to(btreemap! {
    "Location".to_string() => vec![h!("http://go.away.com/to/here")]
  }));
}

#[test]
fn execute_state_machine_returns_410_if_the_resource_has_prev_existed_and_not_a_post() {
  let mut context = WebmachineContext {
    request: WebmachineRequest {
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource {
    resource_exists: callback(&|_, _| false),
    previously_existed: callback(&|_, _| true),
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(410));
}

#[test]
fn execute_state_machine_returns_410_if_the_resource_has_prev_existed_and_a_post_and_posts_to_missing_resource_not_allowed() {
  let mut context = WebmachineContext {
    request: WebmachineRequest {
      method: "POST".to_string(),
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource {
    allowed_methods: vec!["POST"],
    resource_exists: callback(&|_, _| false),
    previously_existed: callback(&|_, _| true),
    allow_missing_post: callback(&|_, _| false),
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(410));
}

#[test]
fn execute_state_machine_returns_404_if_the_resource_has_not_prev_existed_and_a_post_and_posts_to_missing_resource_not_allowed() {
  let mut context = WebmachineContext {
    request: WebmachineRequest {
      method: "POST".to_string(),
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource {
    allowed_methods: vec!["POST"],
    resource_exists: callback(&|_, _| false),
    previously_existed: callback(&|_, _| false),
    allow_missing_post: callback(&|_, _| false),
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(404));
}

#[test]
fn execute_state_machine_returns_412_if_the_resource_etag_does_not_match_if_match_header() {
  let mut context = WebmachineContext {
    request: WebmachineRequest {
      headers: hashmap! {
        "If-Match".to_string() => vec![h!("\"1234567891\"")]
      },
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource {
    resource_exists: callback(&|_, _| true),
    generate_etag: callback(&|_, _| Some("1234567890".to_string())),
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(412));
}

#[test]
fn execute_state_machine_returns_412_if_the_resource_etag_does_not_match_if_match_header_weak_etag() {
  let mut context = WebmachineContext {
    request: WebmachineRequest {
      headers: hashmap! {
        "If-Match".to_string() => vec![h!("W/\"1234567891\"")]
      },
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource {
    resource_exists: callback(&|_, _| true),
    generate_etag: callback(&|_, _| Some("1234567890".to_string())),
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(412));
}

#[test]
fn execute_state_machine_returns_412_if_the_resource_last_modified_gt_unmodified_since() {
  let datetime = Local::now().with_timezone(&FixedOffset::east(10 * 3600));
  let header_datetime = datetime.clone() - Duration::minutes(5);
  let mut context = WebmachineContext {
    request: WebmachineRequest {
      headers: hashmap! {
        "If-Unmodified-Since".to_string() => vec![h!(&*format!("\"{}\"", header_datetime.to_rfc2822()))]
      },
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource {
    resource_exists: callback(&|_, _| true),
    last_modified: callback(&|_, _| Some(Local::now().with_timezone(&FixedOffset::east(10 * 3600)))),
    ..WebmachineResource::default()
  };

  execute_state_machine(&mut context, &resource);

  expect(context.response.status).to(be_equal_to(412));
}

#[test]
fn execute_state_machine_returns_304_if_non_match_star_exists_and_is_not_a_head_or_get() {
  let mut context = WebmachineContext {
    request: WebmachineRequest {
      method: "POST".to_string(),
      headers: hashmap! {
        "If-None-Match".to_string() => vec![h!("*")]
      },
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource {
    resource_exists: callback(&|_, _| true),
    allowed_methods: vec!["POST"],
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(412));
}

#[test]
fn execute_state_machine_returns_304_if_non_match_star_exists_and_is_a_head_or_get() {
  let mut context = WebmachineContext {
    request: WebmachineRequest {
      method: "HEAD".to_string(),
      headers: hashmap! {
        "If-None-Match".to_string() => vec![h!("*")]
      },
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource {
    resource_exists: callback(&|_, _| true),
    allowed_methods: vec!["HEAD"],
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(304));
}

#[test]
fn execute_state_machine_returns_412_if_resource_etag_in_if_non_match_and_is_not_a_head_or_get() {
  let mut context = WebmachineContext {
    request: WebmachineRequest {
      method: "POST".to_string(),
      headers: hashmap! {
        "If-None-Match".to_string() => vec![h!("W/\"1234567890\""), h!("W/\"1234567891\"")]
      },
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource {
    resource_exists: callback(&|_, _| true),
    allowed_methods: vec!["POST"],
    generate_etag: callback(&|_, _| Some("1234567890".to_string())),
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(412));
}

#[test]
fn execute_state_machine_returns_304_if_resource_etag_in_if_non_match_and_is_a_head_or_get() {
  let mut context = WebmachineContext {
    request: WebmachineRequest {
      headers: hashmap! {
        "If-None-Match".to_string() => vec![h!("\"1234567890\""), h!("\"1234567891\"")]
      },
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource {
    resource_exists: callback(&|_, _| true),
    generate_etag: callback(&|_, _| Some("1234567890".to_string())),
    ..WebmachineResource::default()
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
      headers: hashmap! {
        "If-Modified-Since".to_string() => vec![h!(&*format!("\"{}\"", header_datetime.to_rfc2822()))]
      },
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource {
    resource_exists: callback(&|_, _| true),
    last_modified: callback(&|_, _| Some(Local::now().with_timezone(&FixedOffset::east(10 * 3600)) - Duration::minutes(15))),
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(304));
}

#[test]
fn execute_state_machine_returns_202_if_delete_was_not_enacted() {
  let mut context = WebmachineContext {
    request: WebmachineRequest {
      method: "DELETE".to_string(),
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource {
    resource_exists: callback(&|_, _| true),
    delete_resource: callback(&|_, _| Ok(false)),
    allowed_methods: vec!["DELETE"],
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(202));
}

#[test]
fn execute_state_machine_returns_a_resource_status_code_if_delete_fails() {
  let mut context = WebmachineContext {
    request: WebmachineRequest {
      method: "DELETE".to_string(),
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource {
    resource_exists: callback(&|_, _| true),
    delete_resource: callback(&|_, _| Err(500)),
    allowed_methods: vec!["DELETE"],
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(500));
}

#[test]
fn join_paths_test() {
  expect!(join_paths(&Vec::new(), &Vec::new())).to(be_equal_to("/".to_string()));
  expect!(join_paths(&vec!["".to_string()], &Vec::new())).to(be_equal_to("/".to_string()));
  expect!(join_paths(&Vec::new(), &vec!["".to_string()])).to(be_equal_to("/".to_string()));
  expect!(join_paths(&vec!["a".to_string(), "b".to_string(), "c".to_string()], &Vec::new())).to(be_equal_to("/a/b/c".to_string()));
  expect!(join_paths(&vec!["a".to_string(), "b".to_string(), "".to_string()], &Vec::new())).to(be_equal_to("/a/b".to_string()));
  expect!(join_paths(&Vec::new(), &vec!["a".to_string(), "b".to_string(), "c".to_string()])).to(be_equal_to("/a/b/c".to_string()));
  expect!(join_paths(&vec!["a".to_string(), "b".to_string(), "c".to_string()], &vec!["d".to_string(), "e".to_string(), "f".to_string()])).to(be_equal_to("/a/b/c/d/e/f".to_string()));
}

#[test]
fn execute_state_machine_returns_a_resource_status_code_if_post_fails_and_post_is_create() {
  let mut context = WebmachineContext {
    request: WebmachineRequest {
      method: "POST".to_string(),
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource {
    resource_exists: callback(&|_, _| true),
    post_is_create: callback(&|_, _| true),
    create_path: callback(&|_, _| Err(500)),
    allowed_methods: vec!["POST"],
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(500));
}

#[test]
fn execute_state_machine_returns_a_resource_status_code_if_post_fails_and_post_is_not_create() {
  let mut context = WebmachineContext {
    request: WebmachineRequest {
      method: "POST".to_string(),
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource {
    resource_exists: callback(&|_, _| true),
    post_is_create: callback(&|_, _| false),
    process_post: callback(&|_, _| Err(500)),
    allowed_methods: vec!["POST"],
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(500));
}

#[test]
fn execute_state_machine_returns_303_and_post_is_create_and_redirect_is_set() {
  let mut context = WebmachineContext {
    request: WebmachineRequest {
      method: "POST".to_string(),
      base_path: "/base/path".to_string(),
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource {
    resource_exists: callback(&|_, _| true),
    post_is_create: callback(&|_, _| true),
    create_path: callback(&|context, _| {
      context.redirect = true;
      Ok("/new/path".to_string())
    }),
    allowed_methods: vec!["POST"],
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(303));
  expect(context.response.headers).to(be_equal_to(btreemap! {
    "Location".to_string() => vec![h!("/base/path/new/path")]
  }));
}

#[test]
fn execute_state_machine_returns_303_if_post_is_not_create_and_redirect_is_set() {
  let mut context = WebmachineContext {
    request: WebmachineRequest {
      method: "POST".to_string(),
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource {
    resource_exists: callback(&|_, _| true),
    post_is_create: callback(&|_, _| false),
    process_post: callback(&|context, _| {
      context.redirect = true;
      Ok(true)
    }),
    allowed_methods: vec!["POST"],
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(303));
}

#[test]
fn execute_state_machine_returns_303_if_post_to_missing_resource_and_redirect_is_set() {
  let mut context = WebmachineContext {
    request: WebmachineRequest {
      method: "POST".to_string(),
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource {
    resource_exists: callback(&|_, _| false),
    previously_existed: callback(&|_, _| false),
    allow_missing_post: callback(&|_, _| true),
    post_is_create: callback(&|_, _| false),
    process_post: callback(&|context, _| {
      context.redirect = true;
      Ok(true)
    }),
    allowed_methods: vec!["POST"],
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(303));
}

#[test]
fn execute_state_machine_returns_201_if_post_creates_new_resource() {
  let mut context = WebmachineContext {
    request: WebmachineRequest {
      method: "POST".to_string(),
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource {
    resource_exists: callback(&|_, _| false),
    previously_existed: callback(&|_, _| false),
    allow_missing_post: callback(&|_, _| true),
    post_is_create: callback(&|_, _| true),
    create_path: callback(&|_, _| { Ok("/new/path".to_string()) }),
    allowed_methods: vec!["POST"],
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(201));
  expect(context.response.headers).to(be_equal_to(btreemap! {
    "Location".to_string() => vec![h!("/new/path")]
  }));
}

#[test]
fn execute_state_machine_returns_201_if_put_to_new_resource() {
  let mut context = WebmachineContext {
    request: WebmachineRequest {
      method: "PUT".to_string(),
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource {
    resource_exists: callback(&|_, _| false),
    allowed_methods: vec!["PUT"],
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(201));
}

#[test]
fn execute_state_machine_returns_409_for_existing_resource_if_the_put_request_is_a_conflict() {
  let mut context = WebmachineContext {
    request: WebmachineRequest {
      method: "PUT".to_string(),
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource {
    allowed_methods: vec!["PUT"],
    resource_exists: callback(&|_, _| true),
    is_conflict: callback(&|_, _| true),
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(409));
}

#[test]
fn execute_state_machine_returns_200_if_put_request_to_existing_resource() {
  let mut context = WebmachineContext {
    request: WebmachineRequest {
      method: "PUT".to_string(),
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource {
    allowed_methods: vec!["PUT"],
    resource_exists: callback(&|_, _| true),
    process_put: callback(&|context, _| {
      context.response.body = Some("body".as_bytes().to_vec());
      Ok(true)
    }),
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(200));
}

#[test]
fn execute_state_machine_returns_204_if_put_request_to_existing_resource_with_no_response_body() {
  let mut context = WebmachineContext {
    request: WebmachineRequest {
      method: "PUT".to_string(),
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource {
    allowed_methods: vec!["PUT"],
    resource_exists: callback(&|_, _| true),
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(204));
}

#[test]
fn execute_state_machine_returns_300_if_multiple_choices_is_true() {
  let mut context = WebmachineContext {
    request: WebmachineRequest {
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource {
    resource_exists: callback(&|_, _| true),
    multiple_choices: callback(&|_, _| true),
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(300));
}

#[test]
fn execute_state_machine_returns_204_if_delete_was_enacted_and_response_has_no_body() {
  let mut context = WebmachineContext {
    request: WebmachineRequest {
      method: "DELETE".to_string(),
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource {
    resource_exists: callback(&|_, _| true),
    delete_resource: callback(&|_, _| Ok(true)),
    allowed_methods: vec!["DELETE"],
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(204));
}

#[test]
fn execute_state_machine_returns_200_if_delete_was_enacted_and_response_has_a_body() {
  let mut context = WebmachineContext {
    request: WebmachineRequest {
      method: "DELETE".to_string(),
      ..WebmachineRequest::default()
    },
    ..WebmachineContext::default()
  };
  let resource = WebmachineResource {
    resource_exists: callback(&|_, _| true),
    delete_resource: callback(&|context, _| {
      context.response.body = Some("body".as_bytes().to_vec());
      Ok(true)
    }),
    allowed_methods: vec!["DELETE"],
    ..WebmachineResource::default()
  };
  execute_state_machine(&mut context, &resource);
  expect(context.response.status).to(be_equal_to(200));
}

#[test]
fn parse_query_string_test() {
  let query = "a=b&c=d".to_string();
  let expected = hashmap! {
    "a".to_string() => vec!["b".to_string()],
    "c".to_string() => vec!["d".to_string()]
  };
  expect!(parse_query(&query)).to(be_equal_to(expected));
}

#[test]
fn parse_query_string_handles_empty_string() {
  let query = "".to_string();
  expect!(parse_query(&query)).to(be_equal_to(hashmap! {}));
}

#[test]
fn parse_query_string_handles_missing_values() {
  let query = "a=&c=d".to_string();
  let expected = hashmap! {
    "a".to_string() => vec!["".to_string()],
    "c".to_string() => vec!["d".to_string()]
  };
  expect!(parse_query(&query)).to(be_equal_to(expected));
}

#[test]
fn parse_query_string_handles_equals_in_values() {
  let query = "a=b&c=d=e=f".to_string();
  let expected = hashmap! {
    "a".to_string() => vec!["b".to_string()],
    "c".to_string() => vec!["d=e=f".to_string()]
  };
  expect!(parse_query(&query)).to(be_equal_to(expected));
}

#[test]
fn parse_query_string_decodes_values() {
  let query = "a=a%20b%20c".to_string();
  let expected = hashmap! {
    "a".to_string() => vec!["a b c".to_string()]
  };
  expect!(parse_query(&query)).to(be_equal_to(expected));
}
