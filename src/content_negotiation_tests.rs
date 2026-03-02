use content_negotiation::*;
use super::*;
use headers::*;
use context::*;
use expectest::prelude::*;

#[test]
fn matches_if_no_accept_header_is_provided() {
  let resource = WebmachineResource {
    .. WebmachineResource::default()
  };
  let request = WebmachineRequest {
    .. WebmachineRequest::default()
  };
  expect!(matching_content_type(&resource, &request)).to(be_some().value("application/json"));
}

#[test]
fn matches_exact_media_types() {
  let resource = WebmachineResource {
    ..WebmachineResource::default()
  };
  let request = WebmachineRequest {
    headers: hashmap! {
      "Accept".to_string() => vec![HeaderValue::basic("application/json")]
    },
    ..WebmachineRequest::default()
  };
  expect!(matching_content_type(&resource, &request)).to(be_some().value("application/json"));
}

#[test]
fn matches_wild_card_subtype() {
  let resource = WebmachineResource {
    ..WebmachineResource::default()
  };
  let request = WebmachineRequest {
    headers: hashmap! {
      "Accept".to_string() => vec![HeaderValue::basic("application/*")]
    },
    ..WebmachineRequest::default()
  };
  expect!(matching_content_type(&resource, &request)).to(be_some().value("application/json"));
}

#[test]
fn matches_wild_card_type() {
  let resource = WebmachineResource {
    ..WebmachineResource::default()
  };
  let request = WebmachineRequest {
    headers: hashmap! {
      "Accept".to_string() => vec![HeaderValue::basic("*/json")]
    },
    ..WebmachineRequest::default()
  };
  expect!(matching_content_type(&resource, &request)).to(be_some().value("application/json"));
}

#[test]
fn matches_wild_card() {
  let resource = WebmachineResource {
    ..WebmachineResource::default()
  };
  let request = WebmachineRequest {
    headers: hashmap! {
      "Accept".to_string() => vec![HeaderValue::basic("*/*")]
    },
    ..WebmachineRequest::default()
  };
  expect!(matching_content_type(&resource, &request)).to(be_some().value("application/json"));
}

#[test]
fn matches_most_specific() {
  let resource1 = WebmachineResource {
    ..WebmachineResource::default()
  };
  let resource2 = WebmachineResource {
    produces: owned_vec(&["application/pdf"]),
    ..WebmachineResource::default()
  };
  let resource3 = WebmachineResource {
    produces: owned_vec(&["text/plain"]),
    ..WebmachineResource::default()
  };
  let resource4 = WebmachineResource {
    produces: owned_vec(&["text/plain", "application/pdf", "application/json"]),
    ..WebmachineResource::default()
  };
  let resource5 = WebmachineResource {
    produces: owned_vec(&["text/plain", "application/pdf"]),
    ..WebmachineResource::default()
  };
  let request = WebmachineRequest {
    headers: hashmap! {
      "Accept".to_string() => vec![
        HeaderValue::basic("*/*"),
        HeaderValue::basic("application/*"),
        HeaderValue::basic("application/json")
      ]
    },
    ..WebmachineRequest::default()
  };
  expect!(matching_content_type(&resource1, &request)).to(be_some().value("application/json"));
  expect!(matching_content_type(&resource2, &request)).to(be_some().value("application/pdf"));
  expect!(matching_content_type(&resource3, &request)).to(be_some().value("text/plain"));
  expect!(matching_content_type(&resource4, &request)).to(be_some().value("application/json"));
  expect!(matching_content_type(&resource5, &request)).to(be_some().value("application/pdf"));
}

#[test]
fn sort_media_types_basic_test() {
  expect!(sort_media_types(&vec![h!("text/plain")])).to(be_equal_to(vec![h!("text/plain")]));
  expect!(sort_media_types(&vec![h!("text/plain"), h!("text/html")])).to(be_equal_to(vec![h!("text/plain"), h!("text/html")]));
  expect!(sort_media_types(&vec![h!("text/*"), h!("text/html")])).to(be_equal_to(vec![h!("text/html"), h!("text/*")]));
  expect!(sort_media_types(&vec![h!("*/*"), h!("text/*"), h!("text/html")])).to(be_equal_to(vec![h!("text/html"), h!("text/*"), h!("*/*")]));
}

#[test]
fn sort_media_types_with_quality_weighting() {
  expect!(sort_media_types(&vec![h!("text/plain;q=0.2")])).to(be_equal_to(vec![h!("text/plain;q=0.2")]));
  expect!(sort_media_types(&vec![h!("text/plain;q=0.2"), h!("text/html;q=0.3")])).to(be_equal_to(vec![h!("text/html;q=0.3"), h!("text/plain;q=0.2")]));
  expect!(sort_media_types(&vec![h!("text/plain;q=0.2"), h!("text/html")])).to(be_equal_to(vec![h!("text/html"), h!("text/plain;q=0.2")]));
  expect!(sort_media_types(&vec![h!("audio/*; q=0.2"), h!("audio/basic")])).to(be_equal_to(vec![h!("audio/basic"), h!("audio/*;q=0.2")]));
  expect!(sort_media_types(&vec![h!("audio/*;q=1"), h!("audio/basic;q=0.5")])).to(be_equal_to(vec![h!("audio/*;q=1"), h!("audio/basic;q=0.5")]));
  expect!(sort_media_types(&vec![h!("text/plain; q=0.5"), h!("text/html"), h!("text/x-dvi; q=0.8"), h!("text/x-c")]))
    .to(be_equal_to(vec![h!("text/html"), h!("text/x-c"), h!("text/x-dvi;q=0.8"), h!("text/plain;q=0.5")]));
}

#[test]
fn parse_media_type_test() {
  expect!(MediaType::parse_string("text/plain")).to(be_equal_to(MediaType { main: "text".to_string(), sub: "plain".to_string(), weight: 1.0 }));
  expect!(MediaType::parse_string("text/*")).to(be_equal_to(MediaType { main: "text".to_string(), sub: "*".to_string(), weight: 1.0 }));
  expect!(MediaType::parse_string("*/*")).to(be_equal_to(MediaType { main: "*".to_string(), sub: "*".to_string(), weight: 1.0 }));
  expect!(MediaType::parse_string("text/")).to(be_equal_to(MediaType { main: "text".to_string(), sub: "*".to_string(), weight: 1.0 }));
  expect!(MediaType::parse_string("text")).to(be_equal_to(MediaType { main: "text".to_string(), sub: "*".to_string(), weight: 1.0 }));
  expect!(MediaType::parse_string("")).to(be_equal_to(MediaType { main: "*".to_string(), sub: "*".to_string(), weight: 1.0 }));
}

#[test]
fn media_type_matches_test() {
  let media_type = MediaType { main: "application".to_string(), sub: "json".to_string(), weight: 1.0 };
  expect!(media_type.matches(&MediaType { main: "application".to_string(), sub: "json".to_string(), weight: 1.0 })).to(be_equal_to(MediaTypeMatch::Full));
  expect!(media_type.matches(&MediaType { main: "application".to_string(), sub: "*".to_string(), weight: 1.0 })).to(be_equal_to(MediaTypeMatch::SubStar));
  expect!(media_type.matches(&MediaType { main: "*".to_string(), sub: "*".to_string(), weight: 1.0 })).to(be_equal_to(MediaTypeMatch::Star));
  expect!(media_type.matches(&MediaType { main: "application".to_string(), sub: "application".to_string(), weight: 1.0 })).to(be_equal_to(MediaTypeMatch::None));
  expect!(media_type.matches(&MediaType { main: "*".to_string(), sub: "json".to_string(), weight: 1.0 })).to(be_equal_to(MediaTypeMatch::Star));
  expect!(media_type.matches(&MediaType { main: "*".to_string(), sub: "xml".to_string(), weight: 1.0 })).to(be_equal_to(MediaTypeMatch::None));
}

#[test]
fn matching_language_matches_if_no_accept_header_is_provided() {
  let resource = WebmachineResource {
    ..WebmachineResource::default()
  };
  let request = WebmachineRequest {
    ..WebmachineRequest::default()
  };
  expect!(matching_language(&resource, &request)).to(be_some().value("*"));
}

#[test]
fn matching_language_matches_if_the_resource_does_not_define_any_language() {
  let resource = WebmachineResource {
    ..WebmachineResource::default()
  };
  let request = WebmachineRequest {
    headers: hashmap! {
      "Accept-Language".to_string() => vec![h!("en-gb")]
    },
    ..WebmachineRequest::default()
  };
  expect!(matching_language(&resource, &request)).to(be_some().value("en-gb"));
}

#[test]
fn matching_language_matches_if_the_request_language_is_empty() {
  let resource = WebmachineResource {
    languages_provided: owned_vec(&["x-pig-latin"]),
    ..WebmachineResource::default()
  };
  let request = WebmachineRequest {
    headers: hashmap! {
      "Accept-Language".to_string() => Vec::new()
    },
    ..WebmachineRequest::default()
  };
  expect!(matching_language(&resource, &request)).to(be_some().value("x-pig-latin"));
}

#[test]
fn matching_language_matches_exact_language() {
  let resource = WebmachineResource {
    languages_provided: owned_vec(&["en-gb"]),
    ..WebmachineResource::default()
  };
  let request = WebmachineRequest {
    headers: hashmap! {
      "Accept-Language".to_string() => vec![h!("en-gb")]
    },
    ..WebmachineRequest::default()
  };
  expect!(matching_language(&resource, &request)).to(be_some().value("en-gb"));
}

#[test]
fn matching_language_wild_card() {
  let resource = WebmachineResource {
    languages_provided: owned_vec(&["en-gb"]),
    ..WebmachineResource::default()
  };
  let request = WebmachineRequest {
    headers: hashmap! {
      "Accept-Language".to_string() => vec![h!("*")]
    },
    ..WebmachineRequest::default()
  };
  expect!(matching_language(&resource, &request)).to(be_some().value("en-gb"));
}

#[test]
fn matching_language_matches_prefix() {
  let resource = WebmachineResource {
    languages_provided: owned_vec(&["en"]),
    ..WebmachineResource::default()
  };
  let request = WebmachineRequest {
    headers: hashmap! {
      "Accept-Language".to_string() => vec![h!("en-gb")]
    },
    ..WebmachineRequest::default()
  };
  expect!(matching_language(&resource, &request)).to(be_some().value("en"));
}

#[test]
fn matching_language_does_not_match_prefix_if_it_does_not_end_with_dash() {
  let resource = WebmachineResource {
    languages_provided: owned_vec(&["e"]),
    ..WebmachineResource::default()
  };
  let request = WebmachineRequest {
    headers: hashmap! {
      "Accept-Language".to_string() => vec![h!("en-gb")]
    },
    ..WebmachineRequest::default()
  };
  expect!(matching_language(&resource, &request)).to(be_none());
}

#[test]
fn matching_language_does_not_match_if_quality_is_zero() {
  let resource = WebmachineResource {
    languages_provided: owned_vec(&["en"]),
    ..WebmachineResource::default()
  };
  let request = WebmachineRequest {
    headers: hashmap! {
      "Accept-Language".to_string() => vec![h!("en-gb;q=0")]
    },
    ..WebmachineRequest::default()
  };
  expect!(matching_language(&resource, &request)).to(be_none());
}

#[test]
fn matching_language_does_not_match_wildcard_if_quality_is_zero() {
  let resource = WebmachineResource {
    languages_provided: owned_vec(&["en"]),
    ..WebmachineResource::default()
  };
  let request = WebmachineRequest {
    headers: hashmap! {
      "Accept-Language".to_string() => vec![h!("*;q=0")]
    },
    ..WebmachineRequest::default()
  };
  expect!(matching_language(&resource, &request)).to(be_none());
}

#[test]
fn matches_most_specific_language() {
  let resource1 = WebmachineResource {
    ..WebmachineResource::default()
  };
  let resource2 = WebmachineResource {
    languages_provided: owned_vec(&["en-gb"]),
    ..WebmachineResource::default()
  };
  let resource3 = WebmachineResource {
    languages_provided: owned_vec(&["en"]),
    ..WebmachineResource::default()
  };
  let resource4 = WebmachineResource {
    languages_provided: owned_vec(&["en-gb", "da"]),
    ..WebmachineResource::default()
  };
  let request = WebmachineRequest {
    headers: hashmap! {
      "Accept-Language".to_string() => vec![
        h!("da"),
        h!("en-gb;q=0.8"),
        h!("en;q=0.7")
      ]
    },
    ..WebmachineRequest::default()
  };
  expect!(matching_language(&resource1, &request)).to(be_some().value("da"));
  expect!(matching_language(&resource2, &request)).to(be_some().value("en-gb"));
  expect!(matching_language(&resource3, &request)).to(be_some().value("en"));
  expect!(matching_language(&resource4, &request)).to(be_some().value("da"));
}

#[test]
fn language_matches_test() {
  expect!(MediaLanguage::parse_string("en").matches(&MediaLanguage::parse_string("en"))).to(be_true());
  expect!(MediaLanguage::parse_string("en").matches(&MediaLanguage::parse_string("dn"))).to(be_false());
  expect!(MediaLanguage::parse_string("en-gb").matches(&MediaLanguage::parse_string("en-gb"))).to(be_true());
  expect!(MediaLanguage::parse_string("en-gb").matches(&MediaLanguage::parse_string("*"))).to(be_true());
  expect!(MediaLanguage::parse_string("en").matches(&MediaLanguage::parse_string("en-gb"))).to(be_true());
}

#[test]
fn matching_charset_matches_if_no_accept_header_is_provided() {
  let resource = WebmachineResource {
    ..WebmachineResource::default()
  };
  let request = WebmachineRequest {
    ..WebmachineRequest::default()
  };
  expect!(matching_charset(&resource, &request)).to(be_some().value("ISO-8859-1"));
}

#[test]
fn matching_charset_matches_if_the_resource_does_not_define_any_charset() {
  let resource = WebmachineResource {
    ..WebmachineResource::default()
  };
  let request = WebmachineRequest {
    headers: hashmap! {
      "Accept-Charset".to_string() => vec![h!("ISO-8859-5")]
    },
    ..WebmachineRequest::default()
  };
  expect!(matching_charset(&resource, &request)).to(be_some().value("ISO-8859-5"));
}

#[test]
fn matching_charset_matches_if_the_request_language_is_empty() {
  let resource = WebmachineResource {
    charsets_provided: owned_vec(&["Shift-JIS"]),
    ..WebmachineResource::default()
  };
  let request = WebmachineRequest {
    headers: hashmap! {
      "Accept-Charset".to_string() => Vec::new()
    },
    ..WebmachineRequest::default()
  };
  expect!(matching_charset(&resource, &request)).to(be_some().value("Shift-JIS"));
}

#[test]
fn matching_charset_matches_exact_charset() {
  let resource = WebmachineResource {
    charsets_provided: owned_vec(&["ISO-8859-5"]),
    ..WebmachineResource::default()
  };
  let request = WebmachineRequest {
    headers: hashmap! {
      "Accept-Charset".to_string() => vec![h!("ISO-8859-5")]
    },
    ..WebmachineRequest::default()
  };
  expect!(matching_charset(&resource, &request)).to(be_some().value("ISO-8859-5"));
}

#[test]
fn matching_charset_wild_card() {
  let resource = WebmachineResource {
    charsets_provided: owned_vec(&["US-ASCII"]),
    ..WebmachineResource::default()
  };
  let request = WebmachineRequest {
    headers: hashmap! {
      "Accept-Charset".to_string() => vec![h!("*")]
    },
    ..WebmachineRequest::default()
  };
  expect!(matching_charset(&resource, &request)).to(be_some().value("US-ASCII"));
}

#[test]
fn matching_charset_does_not_match_if_quality_is_zero() {
  let resource = WebmachineResource {
    charsets_provided: owned_vec(&["US-ASCII"]),
    ..WebmachineResource::default()
  };
  let request = WebmachineRequest {
    headers: hashmap! {
      "Accept-Charset".to_string() => vec![h!("US-ASCII;q=0")]
    },
    ..WebmachineRequest::default()
  };
  expect!(matching_charset(&resource, &request)).to(be_none());
}

#[test]
fn matches_most_specific_charset() {
  let resource1 = WebmachineResource {
    ..WebmachineResource::default()
  };
  let resource2 = WebmachineResource {
    charsets_provided: owned_vec(&["US-ASCII"]),
    ..WebmachineResource::default()
  };
  let resource3 = WebmachineResource {
    charsets_provided: owned_vec(&["UTF-8"]),
    ..WebmachineResource::default()
  };
  let resource4 = WebmachineResource {
    charsets_provided: owned_vec(&["UTF-8", "US-ASCII"]),
    ..WebmachineResource::default()
  };
  let request = WebmachineRequest {
    headers: hashmap! {
      "Accept-Charset".to_string() => vec![
        h!("ISO-8859-1"),
        h!("UTF-8;q=0.8"),
        h!("US-ASCII;q=0.7")
      ]
    },
    ..WebmachineRequest::default()
  };
  expect!(matching_charset(&resource1, &request)).to(be_some().value("ISO-8859-1"));
  expect!(matching_charset(&resource2, &request)).to(be_some().value("US-ASCII"));
  expect!(matching_charset(&resource3, &request)).to(be_some().value("UTF-8"));
  expect!(matching_charset(&resource4, &request)).to(be_some().value("UTF-8"));
}

#[test]
fn sort_charsets_with_quality_weighting() {
  expect!(sort_media_charsets(&vec![h!("iso-8859-5")]))
    .to(be_equal_to(vec![Charset::parse_string("iso-8859-5"), Charset::parse_string("ISO-8859-1")]));
  expect!(sort_media_charsets(&vec![h!("unicode-1-1;q=0.8"), h!("iso-8859-5")]))
    .to(be_equal_to(vec![Charset::parse_string("iso-8859-5"),
      Charset::parse_string("ISO-8859-1"), Charset::parse_string("unicode-1-1").with_weight("0.8")]));
  expect!(sort_media_charsets(&vec![h!("US-ASCII;q=0.8"), h!("*;q=0.5")]))
    .to(be_equal_to(vec![Charset::parse_string("US-ASCII").with_weight("0.8"),
      Charset::parse_string("*").with_weight("0.5")]));
  expect!(sort_media_charsets(&vec![h!("iso-8859-1; q=0.2"), h!("iso-8859-5")]))
    .to(be_equal_to(vec![Charset::parse_string("iso-8859-5"),
      Charset::parse_string("iso-8859-1").with_weight("0.2")]));
}

#[test]
fn charset_matches_test() {
  expect!(Charset::parse_string("iso-8859-5").matches(&Charset::parse_string("iso-8859-5"))).to(be_true());
  expect!(Charset::parse_string("iso-8859-5").matches(&Charset::parse_string("iso-8859-1"))).to(be_false());
  expect!(Charset::parse_string("iso-8859-5").matches(&Charset::parse_string("ISO-8859-5"))).to(be_true());
  expect!(Charset::parse_string("iso-8859-5").matches(&Charset::parse_string("*"))).to(be_true());
}

#[test]
fn matching_encoding_matches_if_no_accept_header_is_provided() {
  let resource = WebmachineResource {
    ..WebmachineResource::default()
  };
  let request = WebmachineRequest {
    ..WebmachineRequest::default()
  };
  expect!(matching_encoding(&resource, &request)).to(be_some().value("identity"));
}

#[test]
fn matching_encoding_matches_if_the_resource_does_not_define_any_encoding_and_if_no_accept_header_is_provided() {
  let resource = WebmachineResource {
    encodings_provided: Vec::new(),
    ..WebmachineResource::default()
  };
  let request = WebmachineRequest {
    ..WebmachineRequest::default()
  };
  expect!(matching_encoding(&resource, &request)).to(be_some().value("identity"));
}

#[test]
fn matching_encoding_does_not_match_if_the_resource_does_not_define_any_encoding() {
  let resource = WebmachineResource {
    encodings_provided: Vec::new(),
    ..WebmachineResource::default()
  };
  let request = WebmachineRequest {
    headers: hashmap! {
      "Accept-Encoding".to_string() => vec![h!("compress"), h!("*;q=0")]
    },
    ..WebmachineRequest::default()
  };
  expect!(matching_encoding(&resource, &request)).to(be_none());
}

#[test]
fn matching_encoding_matches_if_the_request_encoding_is_empty_and_the_resource_provides_identity() {
  let resource = WebmachineResource {
    encodings_provided: owned_vec(&["compress", "identity"]),
    ..WebmachineResource::default()
  };
  let request = WebmachineRequest {
    headers: hashmap! {
      "Accept-Encoding".to_string() => Vec::new()
    },
    ..WebmachineRequest::default()
  };
  expect!(matching_encoding(&resource, &request)).to(be_some().value("identity"));
}

#[test]
fn matching_encoding_does_not_match_if_the_request_encoding_is_empty_and_the_resource_does_not_provide_identity() {
  let resource = WebmachineResource {
    encodings_provided: owned_vec(&["compress", "gzip"]),
    ..WebmachineResource::default()
  };
  let request = WebmachineRequest {
    headers: hashmap! {
      "Accept-Encoding".to_string() => Vec::new()
    },
    ..WebmachineRequest::default()
  };
  expect!(matching_encoding(&resource, &request)).to(be_none());
}

#[test]
fn matching_encoding_matches_exact_encoding() {
  let resource = WebmachineResource {
    encodings_provided: owned_vec(&["gzip"]),
    ..WebmachineResource::default()
  };
  let request = WebmachineRequest {
    headers: hashmap! {
      "Accept-Encoding".to_string() => vec![h!("gzip")]
    },
    ..WebmachineRequest::default()
  };
  expect!(matching_encoding(&resource, &request)).to(be_some().value("gzip"));
}

#[test]
fn matching_encoding_wild_card() {
  let resource = WebmachineResource {
    encodings_provided: owned_vec(&["compress"]),
    ..WebmachineResource::default()
  };
  let request = WebmachineRequest {
    headers: hashmap! {
      "Accept-Encoding".to_string() => vec![h!("*")]
    },
    ..WebmachineRequest::default()
  };
  expect!(matching_encoding(&resource, &request)).to(be_some().value("compress"));
}

#[test]
fn matching_encoding_does_not_match_if_quality_is_zero() {
  let resource = WebmachineResource {
    encodings_provided: owned_vec(&["gzip"]),
    ..WebmachineResource::default()
  };
  let request = WebmachineRequest {
    headers: hashmap! {
      "Accept-Encoding".to_string() => vec![h!("gzip;q=0")]
    },
    ..WebmachineRequest::default()
  };
  expect!(matching_encoding(&resource, &request)).to(be_none());
}

#[test]
fn matching_encoding_does_not_match_if_star_quality_is_zero() {
  let resource = WebmachineResource {
    encodings_provided: owned_vec(&["identity"]),
    ..WebmachineResource::default()
  };
  let request = WebmachineRequest {
    headers: hashmap! {
      "Accept-Encoding".to_string() => vec![h!("*;q=0")]
    },
    ..WebmachineRequest::default()
  };
  expect!(matching_encoding(&resource, &request)).to(be_none());
}

#[test]
fn matching_encoding_always_matches_if_identity_is_available() {
  let resource = WebmachineResource {
    encodings_provided: owned_vec(&["identity"]),
    ..WebmachineResource::default()
  };
  let request = WebmachineRequest {
    headers: hashmap! {
      "Accept-Encoding".to_string() => vec![h!("gzip")]
    },
    ..WebmachineRequest::default()
  };
  expect!(matching_encoding(&resource, &request)).to(be_some().value("identity"));
}

#[test]
fn matches_most_specific_encoding() {
  let resource1 = WebmachineResource {
    ..WebmachineResource::default()
  };
  let resource2 = WebmachineResource {
    encodings_provided: owned_vec(&["gzip"]),
    ..WebmachineResource::default()
  };
  let resource3 = WebmachineResource {
    encodings_provided: owned_vec(&["compress", "identity"]),
    ..WebmachineResource::default()
  };
  let resource4 = WebmachineResource {
    encodings_provided: owned_vec(&["compress", "gzip", "identity"]),
    ..WebmachineResource::default()
  };
  let request = WebmachineRequest {
    headers: hashmap! {
        "Accept-Encoding".to_string() => vec![
          h!("gzip;q=1.0"),
          h!("*;q=0"),
          h!("identity; q=0.5")
        ]
      },
    ..WebmachineRequest::default()
  };
  expect!(matching_encoding(&resource1, &request)).to(be_some().value("identity"));
  expect!(matching_encoding(&resource2, &request)).to(be_some().value("gzip"));
  expect!(matching_encoding(&resource3, &request)).to(be_some().value("identity"));
  expect!(matching_encoding(&resource4, &request)).to(be_some().value("gzip"));
}

#[test]
fn sort_encodings_with_quality_weighting() {
  expect!(sort_encodings(&vec![h!("gzip")]))
    .to(be_equal_to(vec![Encoding::parse_string("gzip"), Encoding::parse_string("identity")]));
  expect!(sort_encodings(&vec![h!("gzip;q=0.8"), h!("compress")]))
    .to(be_equal_to(vec![Encoding::parse_string("compress"),
                         Encoding::parse_string("identity"),
                         Encoding::parse_string("gzip").with_weight("0.8")]));
  expect!(sort_encodings(&vec![h!("gzip;q=0.8"), h!("*;q=0.5")]))
    .to(be_equal_to(vec![Encoding::parse_string("gzip").with_weight("0.8"),
                         Encoding::parse_string("*").with_weight("0.5")]));
  expect!(sort_encodings(&vec![h!("gzip; q=0.2"), h!("compress;q=0"), h!("*;q=0")]))
    .to(be_equal_to(vec![Encoding::parse_string("gzip").with_weight("0.2")]));
}

#[test]
fn encoding_matches_test() {
  expect!(Encoding::parse_string("identity").matches(&Encoding::parse_string("identity"))).to(be_true());
  expect!(Encoding::parse_string("identity").matches(&Encoding::parse_string("gzip"))).to(be_false());
  expect!(Encoding::parse_string("gzip").matches(&Encoding::parse_string("GZip"))).to(be_true());
  expect!(Encoding::parse_string("compress").matches(&Encoding::parse_string("*"))).to(be_true());
}
