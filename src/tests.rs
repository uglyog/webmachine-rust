use super::*;
use super::sanitise_path;
use expectest::prelude::*;

#[test]
fn path_matcher_test() {
    let dispatcher = WebmachineDispatcher {
        routes: btreemap!{
            "/".to_string() => WebmachineResource {},
            "/path1".to_string() => WebmachineResource {},
            "/path2".to_string() => WebmachineResource {},
            "/path1/path3".to_string() => WebmachineResource {}
        }
    };
    expect!(dispatcher.match_paths(&"/path1".to_string())).to(be_equal_to(vec!["/", "/path1"]));
    expect!(dispatcher.match_paths(&"/path1/".to_string())).to(be_equal_to(vec!["/", "/path1"]));
    expect!(dispatcher.match_paths(&"/path1/path3".to_string())).to(be_equal_to(vec!["/", "/path1", "/path1/path3"]));
    expect!(dispatcher.match_paths(&"/path1/path3/path4".to_string())).to(be_equal_to(vec!["/", "/path1", "/path1/path3"]));
    expect!(dispatcher.match_paths(&"/path1/other".to_string())).to(be_equal_to(vec!["/", "/path1"]));
    expect!(dispatcher.match_paths(&"/path12".to_string())).to(be_equal_to(vec!["/"]));
    expect!(dispatcher.match_paths(&"/".to_string())).to(be_equal_to(vec!["/"]));
}

#[test]
fn sanitise_path_test() {
    expect!(sanitise_path(&"/".to_string()).iter()).to(be_empty());
    expect!(sanitise_path(&"//".to_string()).iter()).to(be_empty());
    expect!(sanitise_path(&"/a/b/c".to_string())).to(be_equal_to(vec!["a", "b", "c"]));
    expect!(sanitise_path(&"/a/b/c/".to_string())).to(be_equal_to(vec!["a", "b", "c"]));
    expect!(sanitise_path(&"/a//b/c".to_string())).to(be_equal_to(vec!["a", "b", "c"]));
}
