//! The `headers` deals with parsing and formatting request and response headers

use std::collections::HashMap;
use std::str::Chars;
use std::iter::Peekable;
use std::hash::{Hash, Hasher};
use itertools::Itertools;

const SEPERATORS: [char; 10] = ['(', ')', '<', '>', '@', ',', ';', '=', '{', '}'];
const VALUE_SEPERATORS: [char; 9] = ['(', ')', '<', '>', '@', ',', ';', '{', '}'];

fn batch(values: &[String]) -> Vec<(String, String)> {
    values.into_iter().batching(|mut it| {
        match it.next() {
           None => None,
           Some(x) => match it.next() {
               None => Some((s!(x), s!(""))),
               Some(y) => Some((s!(x), s!(y))),
           }
        }
    }).collect()
}

// value -> [^SEP]* | quoted-string
fn header_value(chars: &mut Peekable<Chars>, seperators: &[char]) -> String {
    let mut value = String::new();
    skip_whitespace(chars);
    if chars.peek().is_some() && chars.peek().unwrap() == &'"' {
        chars.next();
        while chars.peek().is_some() && chars.peek().unwrap() != &'"' {
            let ch = chars.next().unwrap();
            match ch {
                '\\' => {
                    if chars.peek().is_some() {
                        value.push(chars.next().unwrap());
                    } else {
                        value.push(ch);
                    }
                },
                _ => value.push(ch)
            }
        }
        if chars.peek().is_some() {
            chars.next();
        }
    } else {
        while chars.peek().is_some() && !seperators.contains(chars.peek().unwrap()) {
            value.push(chars.next().unwrap())
        }
    }
    s!(value.trim())
}

// header -> value [; parameters]
fn parse_header(s: String) -> Vec<String> {
    let mut chars = s.chars().peekable();
    let header_value = header_value(&mut chars, &VALUE_SEPERATORS);
    let mut values = vec![header_value];
    if chars.peek().is_some() && chars.peek().unwrap() == &';' {
        chars.next();
        parse_header_parameters(&mut chars, &mut values);
    }
    values
}

// parameters -> parameter [; parameters]
fn parse_header_parameters(chars: &mut Peekable<Chars>, values: &mut Vec<String>) {
    parse_header_parameter(chars, values);
    if chars.peek().is_some() && chars.peek().unwrap() == &';' {
        chars.next();
        parse_header_parameters(chars, values);
    }
}

// parameter -> attribute [= [value]]
fn parse_header_parameter(chars: &mut Peekable<Chars>, values: &mut Vec<String>) {
    values.push(header_value(chars, &SEPERATORS));
    if chars.peek().is_some() && chars.peek().unwrap() == &'=' {
        chars.next();
        parse_header_parameter_value(chars, values);
    }
}

// parameter_value -> value | quoted-string
fn parse_header_parameter_value(chars: &mut Peekable<Chars>, values: &mut Vec<String>) {
    skip_whitespace(chars);
    if chars.peek().is_some() && chars.peek().unwrap() == &'"' {
        chars.next();
        let mut value = String::new();
        while chars.peek().is_some() && chars.peek().unwrap() != &'"' {
            let ch = chars.next().unwrap();
            match ch {
                '\\' => {
                    if chars.peek().is_some() {
                        value.push(chars.next().unwrap());
                    } else {
                        value.push(ch);
                    }
                },
                _ => value.push(ch)
            }
        }
        if chars.peek().is_some() {
            chars.next();
        }
        values.push(s!(value));
    } else {
        values.push(header_value(chars, &[';']));
    }
}

fn skip_whitespace(chars: &mut Peekable<Chars>) {
    while chars.peek().is_some() && chars.peek().unwrap().is_whitespace() {
        chars.next();
    }
}


/// Struct to represent a header value and a map of header value parameters
#[derive(Debug, Clone, Eq)]
pub struct HeaderValue {
    /// Value of the header
    pub value: String,
    /// Map of header value parameters
    pub params: HashMap<String, String>,
    /// If the header should be qouted
    pub quote: bool
}

impl HeaderValue {
    /// Parses a header value string into a HeaderValue struct
    pub fn parse_string(s: String) -> HeaderValue {
        let values = parse_header(s.clone());
        let split = values.split_first().unwrap();
        if split.1.is_empty() {
            HeaderValue::basic(&split.0)
        } else {
            HeaderValue {
                value: split.0.clone(),
                params: batch(split.1).iter()
                    .fold(HashMap::new(), |mut map, ref params| {
                        if !params.0.is_empty() {
                            map.insert(params.0.clone(), params.1.clone());
                        }
                        map
                    }),
                quote: false
            }
        }
    }

    /// Creates a basic header value that has no parameters
    pub fn basic(s: &String) -> HeaderValue {
        HeaderValue {
            value: s.clone(),
            params: HashMap::new(),
            quote: false
        }
    }

    /// Converts this header value into a string representation
    pub fn to_string(&self) -> String {
        let sparams = self.params.iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .join("; ");
        if self.quote {
            if sparams.is_empty() {
                format!("\"{}\"", self.value)
            } else {
                format!("\"{}\"; {}", self.value, sparams)
            }
        } else {
            if self.params.is_empty() {
                self.value.clone()
            } else {
                format!("{}; {}", self.value, sparams)
            }
        }
    }

    /// Parses a weak ETag value. Weak etags are in the form W/<quoted-string>. Returns the
    /// contents of the qouted string if it matches, otherwise returns None.
    pub fn weak_etag(&self) -> Option<String> {
        if self.value.starts_with("W/") {
            Some(parse_header(s!(self.value[2..]))[0].clone())
        } else {
            None
        }
    }

    /// Convertes this header value into a quoted header value
    pub fn quote(mut self) -> HeaderValue {
        self.quote = true;
        self
    }
}

impl PartialEq<HeaderValue> for HeaderValue {
    fn eq(&self, other: &HeaderValue) -> bool {
        self.value == other.value && self.params == other.params
    }
}

impl PartialEq<String> for HeaderValue {
    fn eq(&self, other: &String) -> bool {
        self.value == *other
    }
}

impl PartialEq<str> for HeaderValue {
    fn eq(&self, other: &str) -> bool {
        self.value == *other
    }
}

impl Hash for HeaderValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash(state);
        for (k, v) in self.params.clone() {
            k.hash(state);
            v.hash(state);
        }
    }
}

/// Simple macro to convert a string to a `HeaderValue` struct.
#[macro_export]
macro_rules! h {
    ($e:expr) => (HeaderValue::parse_string($e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use expectest::prelude::*;

    #[test]
    fn parse_header_value_test() {
        expect!(HeaderValue::parse_string(s!(""))).to(be_equal_to(s!("")));
        expect!(HeaderValue::parse_string(s!("A B"))).to(be_equal_to(s!("A B")));
        expect!(HeaderValue::parse_string(s!("A; B"))).to(be_equal_to(HeaderValue {
            value: s!("A"),
            params: hashmap!{ s!("B") => s!("") },
            quote: false
        }));
        expect!(HeaderValue::parse_string(s!("text/html;charset=utf-8"))).to(be_equal_to(HeaderValue {
            value: s!("text/html"),
            params: hashmap!{ s!("charset") => s!("utf-8") },
            quote: false
        }));
        expect!(HeaderValue::parse_string(s!("text/html;charset=UTF-8"))).to(be_equal_to(HeaderValue {
            value: s!("text/html"),
            params: hashmap!{ s!("charset") => s!("UTF-8") },
            quote: false
        }));
        expect!(HeaderValue::parse_string(s!("Text/HTML;Charset= \"utf-8\""))).to(be_equal_to(HeaderValue {
            value: s!("Text/HTML"),
            params: hashmap!{ s!("Charset") => s!("utf-8") },
            quote: false
        }));
        expect!(HeaderValue::parse_string(s!("text/html; charset = \" utf-8 \""))).to(be_equal_to(HeaderValue {
            value: s!("text/html"),
            params: hashmap!{ s!("charset") => s!(" utf-8 ") },
            quote: false
        }));
        expect!(HeaderValue::parse_string(s!(";"))).to(be_equal_to(HeaderValue {
            value: s!(""),
            params: hashmap!{},
            quote: false
        }));
        expect!(HeaderValue::parse_string(s!("A;b=c=d"))).to(be_equal_to(HeaderValue {
            value: s!("A"),
            params: hashmap!{ s!("b") => s!("c=d") },
            quote: false
        }));
        expect!(HeaderValue::parse_string(s!("A;b=\"c;d\""))).to(be_equal_to(HeaderValue {
            value: s!("A"),
            params: hashmap!{ s!("b") => s!("c;d") },
            quote: false
        }));
        expect!(HeaderValue::parse_string(s!("A;b=\"c\\\"d\""))).to(be_equal_to(HeaderValue {
            value: s!("A"),
            params: hashmap!{ s!("b") => s!("c\"d") },
            quote: false
        }));
        expect!(HeaderValue::parse_string(s!("A;b=\"c,d\""))).to(be_equal_to(HeaderValue {
            value: s!("A"),
            params: hashmap!{ s!("b") => s!("c,d") },
            quote: false
        }));
        expect!(HeaderValue::parse_string(s!("en;q=0.0"))).to(be_equal_to(HeaderValue {
            value: s!("en"),
            params: hashmap!{ s!("q") => s!("0.0") },
            quote: false
        }));
    }

    #[test]
    fn parse_qouted_header_value_test() {
        expect!(HeaderValue::parse_string(s!("\"*\""))).to(be_equal_to(HeaderValue {
            value: s!("*"),
            params: hashmap!{},
            quote: false
        }));
        expect!(HeaderValue::parse_string(s!(" \"quoted; value\""))).to(be_equal_to(HeaderValue {
            value: s!("quoted; value"),
            params: hashmap!{},
            quote: false
        }));
    }

    #[test]
    fn parse_etag_header_value_test() {
        let etag = s!("\"1234567890\"");
        let weak_etag = s!("W/\"1234567890\"");

        let header = HeaderValue::parse_string(etag);
        expect!(header.clone()).to(be_equal_to(HeaderValue {
            value: s!("1234567890"),
            params: hashmap!{},
            quote: false
        }));
        expect!(header.weak_etag()).to(be_none());

        let weak_etag_value = HeaderValue::parse_string(weak_etag.clone());
        expect!(weak_etag_value.clone()).to(be_equal_to(HeaderValue {
            value: weak_etag.clone(),
            params: hashmap!{},
            quote: false
        }));
        expect!(weak_etag_value.weak_etag()).to(be_some().value("1234567890"));
    }
}
