//! Utilities for matching URI paths

use itertools::EitherOrBoth::{Both, Left};
use itertools::Itertools;

/// Maps a request path against a template path, populating any variables from the template.
/// Returns None if the paths don't match.
pub fn map_path(path: &str, path_template: &str) -> Option<Vec<(String, Option<String>)>> {
  if path.is_empty() || path_template.is_empty() {
    return None;
  }

  let path_in = path.split('/').filter(|part| !part.is_empty()).collect_vec();
  let path_template = path_template.split('/').filter(|part| !part.is_empty()).collect_vec();
  if path_in.len() >= path_template.len() {
    let mut path_map = vec![];
    for item in path_in.iter().zip_longest(path_template) {
      if let Both(a, b) = item {
        if b.starts_with('{') && b.ends_with('}') {
          path_map.push((a.to_string(), Some(b[1..(b.len() - 1)].to_string())));
        } else if *a == b {
          path_map.push((a.to_string(), None));
        } else {
          return None
        }
      } else if let Left(a) = item {
        path_map.push((a.to_string(), None));
      } else {
        return None
      }
    }
    Some(path_map)
  } else {
    None
  }
}

#[cfg(test)]
mod tests {
  use expectest::prelude::*;

  use crate::paths::map_path;

  #[test]
  fn map_path_simple_values() {
    expect!(map_path("", "")).to(be_none());
    expect!(map_path("/", "/")).to(be_equal_to(Some(vec![])));
    expect!(map_path("/a", "/a")).to(be_equal_to(Some(vec![("a".to_string(), None)])));
    expect!(map_path("/a", "/a")).to(be_equal_to(Some(vec![("a".to_string(), None)])));
    expect!(map_path("/a/", "/a")).to(be_equal_to(Some(vec![("a".to_string(), None)])));
    expect!(map_path("/a/b", "/a/b")).to(be_equal_to(Some(vec![("a".to_string(), None),
      ("b".to_string(), None)])));
    expect!(map_path("/a/b/c", "/a/b/c")).to(be_equal_to(Some(vec![("a".to_string(), None),
      ("b".to_string(), None), ("c".to_string(), None)])));

    expect!(map_path("", "/")).to(be_none());
    expect!(map_path("/", "")).to(be_none());
    expect!(map_path("/", "/a")).to(be_none());
    expect!(map_path("/a", "/")).to(be_some().value(vec![("a".to_string(), None)]));
    expect!(map_path("/a/b", "/a")).to(be_some().value(vec![("a".to_string(), None), ("b".to_string(), None)]));
    expect!(map_path("/a/b", "/a/b/c")).to(be_none());
  }

  #[test]
  fn map_path_with_variables() {
    expect!(map_path("/a", "/{id}")).to(be_equal_to(Some(vec![("a".to_string(), Some("id".to_string()))])));
    expect!(map_path("/a/", "/{id}")).to(be_equal_to(Some(vec![("a".to_string(), Some("id".to_string()))])));
    expect!(map_path("/a", "/{id}/")).to(be_equal_to(Some(vec![("a".to_string(), Some("id".to_string()))])));
    expect!(map_path("/a/b", "/a/{id}")).to(be_equal_to(Some(vec![("a".to_string(), None),
      ("b".to_string(), Some("id".to_string()))])));
    expect!(map_path("/a/b", "/{id}/b")).to(be_equal_to(Some(vec![("a".to_string(), Some("id".to_string())),
      ("b".to_string(), None)])));
    expect!(map_path("/a/b", "/{id}/{id}")).to(be_equal_to(Some(vec![("a".to_string(), Some("id".to_string())),
      ("b".to_string(), Some("id".to_string()))])));
    expect!(map_path("/a/b/c", "/a/{b}/c")).to(be_equal_to(Some(vec![("a".to_string(), None),
      ("b".to_string(), Some("b".to_string())), ("c".to_string(), None)])));

    expect!(map_path("/", "/{id}")).to(be_none());
    expect!(map_path("/a/b", "/{id}")).to(be_some().value(vec![
      ("a".to_string(), Some("id".to_string())),
      ("b".to_string(), None)
    ]));
    expect!(map_path("/a", "/{id}/b")).to(be_none());
    expect!(map_path("/a", "/{id}/{id}")).to(be_none());
    expect!(map_path("/a/b/c", "/{id}/{id}")).to(be_some().value(vec![
      ("a".to_string(), Some("id".to_string())),
      ("b".to_string(), Some("id".to_string())),
      ("c".to_string(), None)
    ]));
  }
}
