//! The `content_negotiation` module deals with handling media types, languages, charsets and
//! encodings as per https://www.w3.org/Protocols/rfc2616/rfc2616-sec14.html.

use itertools::Itertools;
use std::cmp::Ordering;
use headers::{self, HeaderValue};
use WebmachineResource;
use context::WebmachineRequest;

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

/// Sorts the list of media types by their weights
pub fn sort_media_types(media_types: &Vec<HeaderValue>) -> Vec<HeaderValue> {
    media_types.into_iter().cloned().sorted_by(|a, b| {
        let media_a = a.as_media_type().weight();
        let media_b = b.as_media_type().weight();
        let order = media_a.0.partial_cmp(&media_b.0).unwrap_or(Ordering::Greater);
        if order == Ordering::Equal {
            Ord::cmp(&media_a.1, &media_b.1)
        } else {
            order.reverse()
        }
    }).collect()
}

/// Determines if the media types produced by the resource matches the acceptable media types
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
            .filter(|val| val.2 != MediaTypeMatch::None)
            .next().map(|result| result.0.to_string())
    } else {
        resource.produces.first().cloned()
    }
}

/// Struct to represent a media language
#[derive(Debug, Clone, PartialEq)]
pub struct MediaLanguage {
    /// Main type of the media language
    pub main: String,
    /// Sub type of the media language
    pub sub: String,
    /// Weight associated with the media language
    pub weight: f32
}

impl MediaLanguage {
    /// Parse a string into a MediaLanguage struct
    pub fn parse_string(language: &String) -> MediaLanguage {
        let types: Vec<&str> = language.splitn(2, '-').collect_vec();
        if types.is_empty() || types[0].is_empty() {
            MediaLanguage {
                main: s!("*"),
                sub: s!(""),
                weight: 1.0
            }
        } else {
            MediaLanguage {
                main: s!(types[0]),
                sub: if types.len() == 1 || types[1].is_empty() { s!("") } else { s!(types[1]) },
                weight: 1.0
            }
        }
    }

    /// Adds a quality weight to the media language
    pub fn with_weight(&self, weight: &String) -> MediaLanguage {
        MediaLanguage {
            main: self.main.clone(),
            sub: self.sub.clone(),
            weight: weight.parse().unwrap_or(1.0)
        }
    }

    /// If this media language matches the other media language
    pub fn matches(&self, other: &MediaLanguage) -> bool {
        if other.main == "*" || (self.main == other.main && self.sub == other.sub) {
            true
        } else {
            let check = format!("{}-", self.to_string());
            other.to_string().starts_with(&check)
        }
    }

    /// Converts this media language into a string
    pub fn to_string(&self) -> String {
        if self.sub.is_empty() {
            self.main.clone()
        } else {
            format!("{}-{}", self.main, self.sub)
        }
    }
}

impl HeaderValue {
    /// Converts the header value into a media type
    pub fn as_media_language(&self) -> MediaLanguage {
        if self.params.contains_key(&s!("q")) {
            MediaLanguage::parse_string(&self.value).with_weight(self.params.get(&s!("q")).unwrap())
        } else {
            MediaLanguage::parse_string(&self.value)
        }
    }
}

/// Sorts the list of media types by weighting
pub fn sort_media_languages(media_languages: &Vec<HeaderValue>) -> Vec<MediaLanguage> {
    media_languages.iter()
        .cloned()
        .map(|lang| lang.as_media_language())
        .filter(|lang| lang.weight > 0.0)
        .sorted_by(|a, b| {
            let weight_a = a.weight;
            let weight_b = b.weight;
            weight_b.partial_cmp(&weight_a).unwrap_or(Ordering::Greater)
        })
      .collect()
}

/// Determines if the languages produced by the resource matches the acceptable languages
/// provided by the client. Returns the match if there is one.
pub fn matching_language(resource: &WebmachineResource, request: &WebmachineRequest) -> Option<String> {
    if request.has_accept_language_header() && !request.accept_language().is_empty() {
        let acceptable_languages = sort_media_languages(&request.accept_language());
        if resource.languages_provided.is_empty() {
            acceptable_languages.first().map(|lang| lang.to_string())
        } else {
            acceptable_languages.iter()
                .cartesian_product(resource.languages_provided.iter())
                .map(|(acceptable_language, produced_language)| {
                    let produced_language = MediaLanguage::parse_string(produced_language);
                    (produced_language.clone(), produced_language.matches(&acceptable_language))
                })
                .find(|val| val.1)
                .map(|result| result.0.to_string())
        }
    } else if resource.languages_provided.is_empty() {
        Some(s!("*"))
    } else {
        resource.languages_provided.first().cloned()
    }
}

/// Struct to represent a character set
#[derive(Debug, Clone, PartialEq)]
pub struct Charset {
    /// Charset code
    pub charset: String,
    /// Weight associated with the charset
    pub weight: f32
}

impl Charset {
    /// Parse a string into a Charset struct
    pub fn parse_string(charset: &String) -> Charset {
        Charset {
            charset: charset.clone(),
            weight: 1.0
        }
    }

    /// Adds a quality weight to the charset
    pub fn with_weight(&self, weight: &String) -> Charset {
        Charset {
            charset: self.charset.clone(),
            weight: weight.parse().unwrap_or(1.0)
        }
    }

    /// If this media charset matches the other media charset
    pub fn matches(&self, other: &Charset) -> bool {
        other.charset == "*" || (self.charset.to_uppercase() == other.charset.to_uppercase())
    }

    /// Converts this charset into a string
    pub fn to_string(&self) -> String {
        self.charset.clone()
    }
}

impl HeaderValue {
    /// Converts the header value into a media type
    pub fn as_charset(&self) -> Charset {
        if self.params.contains_key(&s!("q")) {
            Charset::parse_string(&self.value).with_weight(self.params.get(&s!("q")).unwrap())
        } else {
            Charset::parse_string(&self.value)
        }
    }
}

/// Sorts the list of charsets by weighting as per https://tools.ietf.org/html/rfc2616#section-14.2.
/// Note that ISO-8859-1 is added as a default with a weighting of 1 if not all ready supplied.
pub fn sort_media_charsets(charsets: &Vec<HeaderValue>) -> Vec<Charset> {
    let mut charsets = charsets.clone();
    if charsets.iter().find(|cs| cs.value == "*" || cs.value.to_uppercase() == "ISO-8859-1").is_none() {
        charsets.push(h!("ISO-8859-1"));
    }
    charsets.into_iter()
        .map(|cs| cs.as_charset())
        .filter(|cs| cs.weight > 0.0)
        .sorted_by(|a, b| {
            let weight_a = a.weight;
            let weight_b = b.weight;
            weight_b.partial_cmp(&weight_a).unwrap_or(Ordering::Greater)
        })
      .collect()
}

/// Determines if the charsets produced by the resource matches the acceptable charsets
/// provided by the client. Returns the match if there is one.
pub fn matching_charset(resource: &WebmachineResource, request: &WebmachineRequest) -> Option<String> {
    if request.has_accept_charset_header() && !request.accept_charset().is_empty() {
        let acceptable_charsets = sort_media_charsets(&request.accept_charset());
        if resource.charsets_provided.is_empty() {
            acceptable_charsets.first().map(|cs| cs.to_string())
        } else {
            acceptable_charsets.iter()
                .cartesian_product(resource.charsets_provided.iter())
                .map(|(acceptable_charset, provided_charset)| {
                    let provided_charset = Charset::parse_string(provided_charset);
                    (provided_charset.clone(), provided_charset.matches(&acceptable_charset))
                })
                .find(|val| val.1)
                .map(|result| result.0.to_string())
        }
    } else if resource.charsets_provided.is_empty() {
        Some(s!("ISO-8859-1"))
    } else {
        resource.charsets_provided.first().cloned()
    }
}

/// Struct to represent an encoding
#[derive(Debug, Clone, PartialEq)]
pub struct Encoding {
    /// Encoding string
    pub encoding: String,
    /// Weight associated with the encoding
    pub weight: f32
}

impl Encoding {
    /// Parse a string into a Charset struct
    pub fn parse_string(encoding: &String) -> Encoding {
        Encoding {
            encoding: encoding.clone(),
            weight: 1.0
        }
    }

    /// Adds a quality weight to the charset
    pub fn with_weight(&self, weight: &String) -> Encoding {
        Encoding {
            encoding: self.encoding.clone(),
            weight: weight.parse().unwrap_or(1.0)
        }
    }

    /// If this encoding matches the other encoding
    pub fn matches(&self, other: &Encoding) -> bool {
        other.encoding == "*" || (self.encoding.to_lowercase() == other.encoding.to_lowercase())
    }

    /// Converts this encoding into a string
    pub fn to_string(&self) -> String {
        self.encoding.clone()
    }
}

impl HeaderValue {
    /// Converts the header value into a media type
    pub fn as_encoding(&self) -> Encoding {
        if self.params.contains_key(&s!("q")) {
            Encoding::parse_string(&self.value).with_weight(self.params.get(&s!("q")).unwrap())
        } else {
            Encoding::parse_string(&self.value)
        }
    }
}

/// Sorts the list of encodings by weighting as per https://tools.ietf.org/html/rfc2616#section-14.3.
/// Note that identity encoding is awlays added with a weight of 1 if not already present.
pub fn sort_encodings(encodings: &Vec<HeaderValue>) -> Vec<Encoding> {
    let mut encodings = encodings.clone();
    if encodings.iter().find(|e| e.value == "*" || e.value.to_lowercase() == "identity").is_none() {
        encodings.push(h!("identity"));
    }
    encodings.into_iter()
        .map(|encoding| encoding.as_encoding())
        .filter(|encoding| encoding.weight > 0.0)
        .sorted_by(|a, b| {
            let weight_a = a.weight;
            let weight_b = b.weight;
            weight_b.partial_cmp(&weight_a).unwrap_or(Ordering::Greater)
        })
      .collect()
}

/// Determines if the encodings supported by the resource matches the acceptable encodings
/// provided by the client. Returns the match if there is one.
pub fn matching_encoding(resource: &WebmachineResource, request: &WebmachineRequest) -> Option<String> {
    let identity = Encoding::parse_string(&s!("identity"));
    if request.has_accept_encoding_header() {
        let acceptable_encodings = sort_encodings(&request.accept_encoding());
        if resource.encodings_provided.is_empty() {
            if acceptable_encodings.contains(&identity) {
                Some(s!("identity"))
            } else {
                None
            }
        } else {
            acceptable_encodings.iter()
                .cartesian_product(resource.encodings_provided.iter())
                .map(|(acceptable_encoding, provided_encoding)| {
                    let provided_encoding = Encoding::parse_string(provided_encoding);
                    (provided_encoding.clone(), provided_encoding.matches(&acceptable_encoding))
                })
                .find(|val| val.1)
                .map(|result| { result.0.to_string() })
        }
    } else if resource.encodings_provided.is_empty() {
        Some(s!("identity"))
    } else {
        resource.encodings_provided.first().cloned()
    }
}
