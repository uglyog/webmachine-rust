//! The `media_types` module deals with handling media types as per https://www.w3.org/Protocols/rfc2616/rfc2616-sec14.html.

use headers::*;
use context::*;
use super::*;
use itertools::Itertools;
use std::cmp::Ordering;

/// Enum to represent a match with media types
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum MediaTypeMatch {
    /// Full match
    Full,
    /// Match where the sub-type was a wild card
    SubStar,
    /// Full whild card match (type and sub-type)
    Star,
    /// Does not match
    None
}

/// Struct to represent a media type
#[derive(Debug, Clone, PartialEq)]
pub struct MediaType {
    /// Main type of the media type
    pub main: String,
    /// Sub type of the media type
    pub sub: String,
    /// Weight associated with the media type
    pub weight: f32
}

impl MediaType {
    /// Parse a string into a MediaType struct
    pub fn parse_string(media_type: &String) -> MediaType {
        let types: Vec<&str> = media_type.splitn(2, '/').collect_vec();
        if types.is_empty() || types[0].is_empty() {
            MediaType {
                main: s!("*"),
                sub: s!("*"),
                weight: 1.0
            }
        } else {
            MediaType {
                main: s!(types[0]),
                sub: if types.len() == 1 || types[1].is_empty() { s!("*") } else { s!(types[1]) },
                weight: 1.0
            }
        }
    }

    /// Adds a quality weight to the media type
    pub fn with_weight(&self, weight: &String) -> MediaType {
        MediaType {
            main: self.main.clone(),
            sub: self.sub.clone(),
            weight: weight.parse().unwrap_or(1.0)
        }
    }

    /// Returns a weighting for this media type
    pub fn weight(&self) -> (f32, u8) {
        if self.main == "*" && self.sub == "*" {
            (self.weight, 2)
        } else if self.sub == "*" {
            (self.weight, 1)
        } else {
            (self.weight, 0)
        }
    }

    /// If this media type matches the other media type
    pub fn matches(&self, other: &MediaType) -> MediaTypeMatch {
        if other.main == "*" {
            MediaTypeMatch::Star
        } else if self.main == other.main && other.sub == "*" {
            MediaTypeMatch::SubStar
        } else if self.main == other.main && self.sub == other.sub {
            MediaTypeMatch::Full
        } else {
            MediaTypeMatch::None
        }
    }

    /// Converts this media type into a string
    pub fn to_string(&self) -> String {
        format!("{}/{}", self.main, self.sub)
    }
}

impl HeaderValue {
    /// Converts the header value into a media type
    pub fn as_media_type(&self) -> MediaType {
        if self.params.contains_key(&s!("q")) {
            MediaType::parse_string(&self.value).with_weight(self.params.get(&s!("q")).unwrap())
        } else {
            MediaType::parse_string(&self.value)
        }
    }
}

fn sort_media_types(media_types: &Vec<HeaderValue>) -> Vec<HeaderValue> {
    media_types.into_iter().cloned().sorted_by(|a, b| {
        let media_a = a.as_media_type().weight();
        let media_b = b.as_media_type().weight();
        let order = media_a.0.partial_cmp(&media_b.0).unwrap_or(Ordering::Greater);
        if order == Ordering::Equal {
            Ord::cmp(&media_a.1, &media_b.1)
        } else {
            order.reverse()
        }
    })
}

/// Determines if the media types produces by the resource matches the acceptable media types
/// provided by the client. Returns the match if there is one.
pub fn matching_content_type(resource: &WebmachineResource, request: &WebmachineRequest) -> Option<String> {
    if request.has_accept_header() {
        let acceptable_media_types = sort_media_types(&request.accept());
        resource.produces.iter()
            .cloned()
            .cartesian_product(acceptable_media_types.iter())
            .map(|(produced, acceptable)| {
                let acceptable_media_type = acceptable.as_media_type();
                let produced_media_type =  MediaType::parse_string(&produced);
                (produced_media_type.clone(), acceptable_media_type.clone(), produced_media_type.matches(&acceptable_media_type))
            })
            .sorted_by(|a, b| Ord::cmp(&a.2, &b.2))
            .iter()
            .filter(|val| val.2 != MediaTypeMatch::None)
            .next().map(|result| result.0.to_string())
    } else {
        resource.produces.first().cloned()
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use super::sort_media_types;
    use super::super::*;
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
}
