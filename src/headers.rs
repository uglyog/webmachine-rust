//! The `headers` deals with parsing and formatting request and response headers

use std::collections::HashMap;
use std::str::Chars;
use std::iter::Peekable;
use std::hash::{Hash, Hasher};
use itertools::Itertools;

const SEPERATORS: [char; 10] = ['(', ')', '<', '>', '@', ',', ';', '=', '{', '}'];
const VALUE_SEPERATORS: [char; 9] = ['(', ')', '<', '>', '@', ',', ';', '{', '}'];

fn batch(values: &[String]) -> Vec<(String, String)> {
  values.into_iter().batching(|it| {
    match it.next() {
     None => None,
     Some(x) => match it.next() {
       None => Some((x.to_string(), "".to_string())),
       Some(y) => Some((x.to_string(), y.to_string())),
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
    value.trim().to_string()
}

// header -> value [; parameters]
fn parse_header(s: &str) -> Vec<String> {
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
        values.push(value.to_string());
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
  pub fn parse_string(s: &str) -> HeaderValue {
    let values = parse_header(s);
    let (first, second) = values.split_first().unwrap();
    if second.is_empty() {
      HeaderValue::basic(first.as_str())
    } else {
      HeaderValue {
        value: first.clone(),
        params: batch(second).iter()
          .fold(HashMap::new(), |mut map, params| {
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
    pub fn basic<S: Into<String>>(s: S) -> HeaderValue {
      HeaderValue {
        value: s.into(),
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
        Some(parse_header(&self.value[2..])[0].clone())
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
  ($e:expr) => (HeaderValue::parse_string($e.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use expectest::prelude::*;

    #[test]
    fn parse_header_value_test() {
        expect!(HeaderValue::parse_string("")).to(be_equal_to("".to_string()));
        expect!(HeaderValue::parse_string("A B")).to(be_equal_to("A B".to_string()));
        expect!(HeaderValue::parse_string("A; B")).to(be_equal_to(HeaderValue {
            value: "A".to_string(),
            params: hashmap!{ "B".to_string() => "".to_string() },
            quote: false
        }));
        expect!(HeaderValue::parse_string("text/html;charset=utf-8")).to(be_equal_to(HeaderValue {
            value: "text/html".to_string(),
            params: hashmap!{ "charset".to_string() => "utf-8".to_string() },
            quote: false
        }));
        expect!(HeaderValue::parse_string("text/html;charset=UTF-8")).to(be_equal_to(HeaderValue {
            value: "text/html".to_string(),
            params: hashmap!{ "charset".to_string() => "UTF-8".to_string() },
            quote: false
        }));
        expect!(HeaderValue::parse_string("Text/HTML;Charset= \"utf-8\"")).to(be_equal_to(HeaderValue {
            value: "Text/HTML".to_string(),
            params: hashmap!{ "Charset".to_string() => "utf-8".to_string() },
            quote: false
        }));
        expect!(HeaderValue::parse_string("text/html; charset = \" utf-8 \"")).to(be_equal_to(HeaderValue {
            value: "text/html".to_string(),
            params: hashmap!{ "charset".to_string() => " utf-8 ".to_string() },
            quote: false
        }));
        expect!(HeaderValue::parse_string(";")).to(be_equal_to(HeaderValue {
            value: "".to_string(),
            params: hashmap!{},
            quote: false
        }));
        expect!(HeaderValue::parse_string("A;b=c=d")).to(be_equal_to(HeaderValue {
            value: "A".to_string(),
            params: hashmap!{ "b".to_string() => "c=d".to_string() },
            quote: false
        }));
        expect!(HeaderValue::parse_string("A;b=\"c;d\"")).to(be_equal_to(HeaderValue {
            value: "A".to_string(),
            params: hashmap!{ "b".to_string() => "c;d".to_string() },
            quote: false
        }));
        expect!(HeaderValue::parse_string("A;b=\"c\\\"d\"")).to(be_equal_to(HeaderValue {
            value: "A".to_string(),
            params: hashmap!{ "b".to_string() => "c\"d".to_string() },
            quote: false
        }));
        expect!(HeaderValue::parse_string("A;b=\"c,d\"")).to(be_equal_to(HeaderValue {
            value: "A".to_string(),
            params: hashmap!{ "b".to_string() => "c,d".to_string() },
            quote: false
        }));
        expect!(HeaderValue::parse_string("en;q=0.0")).to(be_equal_to(HeaderValue {
            value: "en".to_string(),
            params: hashmap!{ "q".to_string() => "0.0".to_string() },
            quote: false
        }));
    }

    #[test]
    fn parse_qouted_header_value_test() {
        expect!(HeaderValue::parse_string("\"*\"")).to(be_equal_to(HeaderValue {
            value: "*".to_string(),
            params: hashmap!{},
            quote: false
        }));
        expect!(HeaderValue::parse_string(" \"quoted; value\"")).to(be_equal_to(HeaderValue {
            value: "quoted; value".to_string(),
            params: hashmap!{},
            quote: false
        }));
    }

    #[test]
    fn parse_etag_header_value_test() {
        let etag = "\"1234567890\"";
        let weak_etag = "W/\"1234567890\"";

        let header = HeaderValue::parse_string(etag);
        expect!(header.clone()).to(be_equal_to(HeaderValue {
            value: "1234567890".to_string(),
            params: hashmap!{},
            quote: false
        }));
        expect!(header.weak_etag()).to(be_none());

        let weak_etag_value = HeaderValue::parse_string(weak_etag.clone());
        expect!(weak_etag_value.clone()).to(be_equal_to(HeaderValue {
            value: weak_etag.to_string(),
            params: hashmap!{},
            quote: false
        }));
        expect!(weak_etag_value.weak_etag()).to(be_some().value("1234567890"));
    }
}
