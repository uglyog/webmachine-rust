use super::*;
use super::sanitise_path;
use super::{execute_state_machine, update_paths_for_resource};
use super::context::*;
use expectest::prelude::*;

fn resource(path: &str) -> WebmachineRequest {
    WebmachineRequest {
        request_path: s!(path),
        base_path: s!("/"),
        method: s!("GET")
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
        s!("OPTIONS"), s!("GET"), s!("HEAD")
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
