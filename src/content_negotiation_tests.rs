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
        .. WebmachineResource::default()
    };
    let request = WebmachineRequest {
        headers: hashmap!{
            s!("Accept") => vec![HeaderValue::basic(&s!("application/json"))]
        },
        .. WebmachineRequest::default()
    };
    expect!(matching_content_type(&resource, &request)).to(be_some().value("application/json"));
}

#[test]
fn matches_wild_card_subtype() {
    let resource = WebmachineResource {
        .. WebmachineResource::default()
    };
    let request = WebmachineRequest {
        headers: hashmap!{
            s!("Accept") => vec![HeaderValue::basic(&s!("application/*"))]
        },
        .. WebmachineRequest::default()
    };
    expect!(matching_content_type(&resource, &request)).to(be_some().value("application/json"));
}

#[test]
fn matches_wild_card_type() {
    let resource = WebmachineResource {
        .. WebmachineResource::default()
    };
    let request = WebmachineRequest {
        headers: hashmap!{
            s!("Accept") => vec![HeaderValue::basic(&s!("*/json"))]
        },
        .. WebmachineRequest::default()
    };
    expect!(matching_content_type(&resource, &request)).to(be_some().value("application/json"));
}

#[test]
fn matches_wild_card() {
    let resource = WebmachineResource {
        .. WebmachineResource::default()
    };
    let request = WebmachineRequest {
        headers: hashmap!{
            s!("Accept") => vec![HeaderValue::basic(&s!("*/*"))]
        },
        .. WebmachineRequest::default()
    };
    expect!(matching_content_type(&resource, &request)).to(be_some().value("application/json"));
}

#[test]
fn matches_most_specific() {
    let resource1 = WebmachineResource {
        .. WebmachineResource::default()
    };
    let resource2 = WebmachineResource {
        produces: vec![s!("application/pdf")],
        .. WebmachineResource::default()
    };
    let resource3 = WebmachineResource {
        produces: vec![s!("text/plain")],
        .. WebmachineResource::default()
    };
    let resource4 = WebmachineResource {
        produces: vec![s!("text/plain"), s!("application/pdf"), s!("application/json")],
        .. WebmachineResource::default()
    };
    let resource5 = WebmachineResource {
        produces: vec![s!("text/plain"), s!("application/pdf")],
        .. WebmachineResource::default()
    };
    let request = WebmachineRequest {
        headers: hashmap!{
            s!("Accept") => vec![
                HeaderValue::basic(&s!("*/*")),
                HeaderValue::basic(&s!("application/*")),
                HeaderValue::basic(&s!("application/json"))
            ]
        },
        .. WebmachineRequest::default()
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
    expect!(MediaType::parse_string(&s!("text/plain"))).to(be_equal_to(MediaType{ main: s!("text"), sub: s!("plain"), weight: 1.0 }));
    expect!(MediaType::parse_string(&s!("text/*"))).to(be_equal_to(MediaType{ main: s!("text"), sub: s!("*"), weight: 1.0 }));
    expect!(MediaType::parse_string(&s!("*/*"))).to(be_equal_to(MediaType{ main: s!("*"), sub: s!("*"), weight: 1.0 }));
    expect!(MediaType::parse_string(&s!("text/"))).to(be_equal_to(MediaType{ main: s!("text"), sub: s!("*"), weight: 1.0 }));
    expect!(MediaType::parse_string(&s!("text"))).to(be_equal_to(MediaType{ main: s!("text"), sub: s!("*"), weight: 1.0 }));
    expect!(MediaType::parse_string(&s!(""))).to(be_equal_to(MediaType{ main: s!("*"), sub: s!("*"), weight: 1.0 }));
}

#[test]
fn media_type_matches_test() {
    let media_type = MediaType { main: s!("application"), sub: s!("json"), weight: 1.0 };
    expect!(media_type.matches(&MediaType { main: s!("application"), sub: s!("json"), weight: 1.0 })).to(be_equal_to(MediaTypeMatch::Full));
    expect!(media_type.matches(&MediaType { main: s!("application"), sub: s!("*"), weight: 1.0 })).to(be_equal_to(MediaTypeMatch::SubStar));
    expect!(media_type.matches(&MediaType { main: s!("*"), sub: s!("*"), weight: 1.0 })).to(be_equal_to(MediaTypeMatch::Star));
    expect!(media_type.matches(&MediaType { main: s!("application"), sub: s!("application"), weight: 1.0 })).to(be_equal_to(MediaTypeMatch::None));
}

#[test]
fn matching_language_matches_if_no_accept_header_is_provided() {
    let resource = WebmachineResource {
        .. WebmachineResource::default()
    };
    let request = WebmachineRequest {
        .. WebmachineRequest::default()
    };
    expect!(matching_language(&resource, &request)).to(be_some().value("*"));
}

#[test]
fn matching_language_matches_if_the_resource_does_not_define_any_language() {
    let resource = WebmachineResource {
        .. WebmachineResource::default()
    };
    let request = WebmachineRequest {
        headers: hashmap!{
            s!("Accept-Language") => vec![h!("en-gb")]
        },
        .. WebmachineRequest::default()
    };
    expect!(matching_language(&resource, &request)).to(be_some().value("en-gb"));
}

#[test]
fn matching_language_matches_if_the_request_language_is_empty() {
    let resource = WebmachineResource {
        languages_provided: vec![s!("x-pig-latin")],
        .. WebmachineResource::default()
    };
    let request = WebmachineRequest {
        headers: hashmap!{
            s!("Accept-Language") => Vec::new()
        },
        .. WebmachineRequest::default()
    };
    expect!(matching_language(&resource, &request)).to(be_some().value("x-pig-latin"));
}

#[test]
fn matching_language_matches_exact_language() {
    let resource = WebmachineResource {
        languages_provided: vec![s!("en-gb")],
        .. WebmachineResource::default()
    };
    let request = WebmachineRequest {
        headers: hashmap!{
            s!("Accept-Language") => vec![h!("en-gb")]
        },
        .. WebmachineRequest::default()
    };
    expect!(matching_language(&resource, &request)).to(be_some().value("en-gb"));
}

#[test]
fn matching_language_wild_card() {
    let resource = WebmachineResource {
        languages_provided: vec![s!("en-gb")],
        .. WebmachineResource::default()
    };
    let request = WebmachineRequest {
        headers: hashmap!{
            s!("Accept-Language") => vec![h!("*")]
        },
        .. WebmachineRequest::default()
    };
    expect!(matching_language(&resource, &request)).to(be_some().value("en-gb"));
}

#[test]
fn matching_language_matches_prefix() {
    let resource = WebmachineResource {
        languages_provided: vec![s!("en")],
        .. WebmachineResource::default()
    };
    let request = WebmachineRequest {
        headers: hashmap!{
            s!("Accept-Language") => vec![h!("en-gb")]
        },
        .. WebmachineRequest::default()
    };
    expect!(matching_language(&resource, &request)).to(be_some().value("en"));
}

#[test]
fn matching_language_does_not_match_prefix_if_it_does_not_end_with_dash() {
    let resource = WebmachineResource {
        languages_provided: vec![s!("e")],
        .. WebmachineResource::default()
    };
    let request = WebmachineRequest {
        headers: hashmap!{
            s!("Accept-Language") => vec![h!("en-gb")]
        },
        .. WebmachineRequest::default()
    };
    expect!(matching_language(&resource, &request)).to(be_none());
}

#[test]
fn matching_language_does_not_match_if_quality_is_zero() {
    let resource = WebmachineResource {
        languages_provided: vec![s!("en")],
        .. WebmachineResource::default()
    };
    let request = WebmachineRequest {
        headers: hashmap!{
            s!("Accept-Language") => vec![h!("en-gb;q=0")]
        },
        .. WebmachineRequest::default()
    };
    expect!(matching_language(&resource, &request)).to(be_none());
}

#[test]
fn matching_language_does_not_match_wildcard_if_quality_is_zero() {
    let resource = WebmachineResource {
        languages_provided: vec![s!("en")],
        .. WebmachineResource::default()
    };
    let request = WebmachineRequest {
        headers: hashmap!{
            s!("Accept-Language") => vec![h!("*;q=0")]
        },
        .. WebmachineRequest::default()
    };
    expect!(matching_language(&resource, &request)).to(be_none());
}

#[test]
fn matches_most_specific_language() {
    let resource1 = WebmachineResource {
        .. WebmachineResource::default()
    };
    let resource2 = WebmachineResource {
        languages_provided: vec![s!("en-gb")],
        .. WebmachineResource::default()
    };
    let resource3 = WebmachineResource {
        languages_provided: vec![s!("en")],
        .. WebmachineResource::default()
    };
    let resource4 = WebmachineResource {
        languages_provided: vec![s!("en-gb"), s!("da")],
        .. WebmachineResource::default()
    };
    let request = WebmachineRequest {
        headers: hashmap!{
            s!("Accept-Language") => vec![
                h!("da"),
                h!("en-gb;q=0.8"),
                h!("en;q=0.7")
            ]
        },
        .. WebmachineRequest::default()
    };
    expect!(matching_language(&resource1, &request)).to(be_some().value("da"));
    expect!(matching_language(&resource2, &request)).to(be_some().value("en-gb"));
    expect!(matching_language(&resource3, &request)).to(be_some().value("en"));
    expect!(matching_language(&resource4, &request)).to(be_some().value("da"));
}

#[test]
fn language_matches_test() {
    expect!(MediaLanguage::parse_string(&s!("en")).matches(&MediaLanguage::parse_string(&s!("en")))).to(be_true());
    expect!(MediaLanguage::parse_string(&s!("en")).matches(&MediaLanguage::parse_string(&s!("dn")))).to(be_false());
    expect!(MediaLanguage::parse_string(&s!("en-gb")).matches(&MediaLanguage::parse_string(&s!("en-gb")))).to(be_true());
    expect!(MediaLanguage::parse_string(&s!("en-gb")).matches(&MediaLanguage::parse_string(&s!("*")))).to(be_true());
    expect!(MediaLanguage::parse_string(&s!("en")).matches(&MediaLanguage::parse_string(&s!("en-gb")))).to(be_true());
}

#[test]
fn matching_charset_matches_if_no_accept_header_is_provided() {
    let resource = WebmachineResource {
        .. WebmachineResource::default()
    };
    let request = WebmachineRequest {
        .. WebmachineRequest::default()
    };
    expect!(matching_charset(&resource, &request)).to(be_some().value("ISO-8859-1"));
}

#[test]
fn matching_charset_matches_if_the_resource_does_not_define_any_charset() {
    let resource = WebmachineResource {
        .. WebmachineResource::default()
    };
    let request = WebmachineRequest {
        headers: hashmap!{
            s!("Accept-Charset") => vec![h!("ISO-8859-5")]
        },
        .. WebmachineRequest::default()
    };
    expect!(matching_charset(&resource, &request)).to(be_some().value("ISO-8859-5"));
}

#[test]
fn matching_charset_matches_if_the_request_language_is_empty() {
    let resource = WebmachineResource {
        charsets_provided: vec![s!("Shift-JIS")],
        .. WebmachineResource::default()
    };
    let request = WebmachineRequest {
        headers: hashmap!{
            s!("Accept-Charset") => Vec::new()
        },
        .. WebmachineRequest::default()
    };
    expect!(matching_charset(&resource, &request)).to(be_some().value("Shift-JIS"));
}

#[test]
fn matching_charset_matches_exact_charset() {
    let resource = WebmachineResource {
        charsets_provided: vec![s!("ISO-8859-5")],
        .. WebmachineResource::default()
    };
    let request = WebmachineRequest {
        headers: hashmap!{
            s!("Accept-Charset") => vec![h!("ISO-8859-5")]
        },
        .. WebmachineRequest::default()
    };
    expect!(matching_charset(&resource, &request)).to(be_some().value("ISO-8859-5"));
}

#[test]
fn matching_charset_wild_card() {
    let resource = WebmachineResource {
        charsets_provided: vec![s!("US-ASCII")],
        .. WebmachineResource::default()
    };
    let request = WebmachineRequest {
        headers: hashmap!{
            s!("Accept-Charset") => vec![h!("*")]
        },
        .. WebmachineRequest::default()
    };
    expect!(matching_charset(&resource, &request)).to(be_some().value("US-ASCII"));
}

#[test]
fn matching_charset_does_not_match_if_quality_is_zero() {
    let resource = WebmachineResource {
        charsets_provided: vec![s!("US-ASCII")],
        .. WebmachineResource::default()
    };
    let request = WebmachineRequest {
        headers: hashmap!{
            s!("Accept-Charset") => vec![h!("US-ASCII;q=0")]
        },
        .. WebmachineRequest::default()
    };
    expect!(matching_charset(&resource, &request)).to(be_none());
}

#[test]
fn matches_most_specific_charset() {
    let resource1 = WebmachineResource {
        .. WebmachineResource::default()
    };
    let resource2 = WebmachineResource {
        charsets_provided: vec![s!("US-ASCII")],
        .. WebmachineResource::default()
    };
    let resource3 = WebmachineResource {
        charsets_provided: vec![s!("UTF-8")],
        .. WebmachineResource::default()
    };
    let resource4 = WebmachineResource {
        charsets_provided: vec![s!("UTF-8"), s!("US-ASCII")],
        .. WebmachineResource::default()
    };
    let request = WebmachineRequest {
        headers: hashmap!{
            s!("Accept-Charset") => vec![
                h!("ISO-8859-1"),
                h!("UTF-8;q=0.8"),
                h!("US-ASCII;q=0.7")
            ]
        },
        .. WebmachineRequest::default()
    };
    expect!(matching_charset(&resource1, &request)).to(be_some().value("ISO-8859-1"));
    expect!(matching_charset(&resource2, &request)).to(be_some().value("US-ASCII"));
    expect!(matching_charset(&resource3, &request)).to(be_some().value("UTF-8"));
    expect!(matching_charset(&resource4, &request)).to(be_some().value("UTF-8"));
}

#[test]
fn sort_charsets_with_quality_weighting() {
    expect!(sort_media_charsets(&vec![h!("iso-8859-5")]))
        .to(be_equal_to(vec![Charset::parse_string(&s!("iso-8859-5")), Charset::parse_string(&s!("ISO-8859-1"))]));
    expect!(sort_media_charsets(&vec![h!("unicode-1-1;q=0.8"), h!("iso-8859-5")]))
        .to(be_equal_to(vec![Charset::parse_string(&s!("iso-8859-5")),
        Charset::parse_string(&s!("ISO-8859-1")), Charset::parse_string(&s!("unicode-1-1")).with_weight(&s!("0.8"))]));
    expect!(sort_media_charsets(&vec![h!("US-ASCII;q=0.8"), h!("*;q=0.5")]))
        .to(be_equal_to(vec![Charset::parse_string(&s!("US-ASCII")).with_weight(&s!("0.8")),
        Charset::parse_string(&s!("*")).with_weight(&s!("0.5"))]));
    expect!(sort_media_charsets(&vec![h!("iso-8859-1; q=0.2"), h!("iso-8859-5")]))
        .to(be_equal_to(vec![Charset::parse_string(&s!("iso-8859-5")),
        Charset::parse_string(&s!("iso-8859-1")).with_weight(&s!("0.2"))]));
}

#[test]
fn charset_matches_test() {
    expect!(Charset::parse_string(&s!("iso-8859-5")).matches(&Charset::parse_string(&s!("iso-8859-5")))).to(be_true());
    expect!(Charset::parse_string(&s!("iso-8859-5")).matches(&Charset::parse_string(&s!("iso-8859-1")))).to(be_false());
    expect!(Charset::parse_string(&s!("iso-8859-5")).matches(&Charset::parse_string(&s!("ISO-8859-5")))).to(be_true());
    expect!(Charset::parse_string(&s!("iso-8859-5")).matches(&Charset::parse_string(&s!("*")))).to(be_true());
}
