//! The `headers` deals with parsing and formatting request and response headers

use std::collections::HashMap;
use std::str::Chars;
use std::iter::Peekable;
use itertools::Itertools;

const SEPERATORS: [char; 10] = ['(', ')', '<', '>', '@', ',', ';', '=', '{', '}'];

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

// value -> [^SEP]*
fn header_value(chars: &mut Peekable<Chars>, seperators: &[char]) -> String {
    let mut value = String::new();
    while chars.peek().is_some() && !seperators.contains(chars.peek().unwrap()) {
        value.push(chars.next().unwrap())
    }
    s!(value.trim())
}

// header -> value [; parameters]
fn parse_header(s: String) -> Vec<String> {
    let mut chars = s.chars().peekable();
    let header_value = header_value(&mut chars, &SEPERATORS);
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
#[derive(Debug, Clone)]
pub struct HeaderValue {
    /// Value of the header
    pub value: String,
    /// Map of header value parameters
    pub params: HashMap<String, String>
}

impl HeaderValue {
    /// Parses a header value string into a HeaderValue struct
    pub fn parse_string(s: String) -> HeaderValue {
        if s.contains(';') {
            let values = parse_header(s);
            let split = values.split_first().unwrap();
            p!(split);
            HeaderValue {
                value: split.0.clone(),
                params: batch(split.1).iter()
                    .fold(HashMap::new(), |mut map, ref params| {
                        if !params.0.is_empty() {
                            map.insert(params.0.clone(), params.1.clone());
                        }
                        map
                    })
            }
        } else {
            HeaderValue::basic(&s)
        }
    }

    /// Creates a basic header value that has no parameters
    pub fn basic(s: &String) -> HeaderValue {
        HeaderValue {
            value: s.clone(),
            params: HashMap::new()
        }
    }

    /// Converts this header value into a string representation
    pub fn to_string(&self) -> String {
        if self.params.is_empty() {
            self.value.clone()
        } else {
            format!("{}; {}", self.value, self.params.iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .join("; "))
        }
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
            params: hashmap!{ s!("B") => s!("") }
        }));
        expect!(HeaderValue::parse_string(s!("text/html;charset=utf-8"))).to(be_equal_to(HeaderValue {
            value: s!("text/html"),
            params: hashmap!{ s!("charset") => s!("utf-8") }
        }));
        expect!(HeaderValue::parse_string(s!("text/html;charset=UTF-8"))).to(be_equal_to(HeaderValue {
            value: s!("text/html"),
            params: hashmap!{ s!("charset") => s!("UTF-8") }
        }));
        expect!(HeaderValue::parse_string(s!("Text/HTML;Charset= \"utf-8\""))).to(be_equal_to(HeaderValue {
            value: s!("Text/HTML"),
            params: hashmap!{ s!("Charset") => s!("utf-8") }
        }));
        expect!(HeaderValue::parse_string(s!("text/html; charset = \" utf-8 \""))).to(be_equal_to(HeaderValue {
            value: s!("text/html"),
            params: hashmap!{ s!("charset") => s!(" utf-8 ") }
        }));
        expect!(HeaderValue::parse_string(s!(";"))).to(be_equal_to(HeaderValue {
            value: s!(""),
            params: hashmap!{}
        }));
        expect!(HeaderValue::parse_string(s!("A;b=c=d"))).to(be_equal_to(HeaderValue {
            value: s!("A"),
            params: hashmap!{ s!("b") => s!("c=d") }
        }));
        expect!(HeaderValue::parse_string(s!("A;b=\"c;d\""))).to(be_equal_to(HeaderValue {
            value: s!("A"),
            params: hashmap!{ s!("b") => s!("c;d") }
        }));
        expect!(HeaderValue::parse_string(s!("A;b=\"c\\\"d\""))).to(be_equal_to(HeaderValue {
            value: s!("A"),
            params: hashmap!{ s!("b") => s!("c\"d") }
        }));
        expect!(HeaderValue::parse_string(s!("A;b=\"c,d\""))).to(be_equal_to(HeaderValue {
            value: s!("A"),
            params: hashmap!{ s!("b") => s!("c,d") }
        }));
    }
}
