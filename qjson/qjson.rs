// Copyright (C) 2022 by Richard Berry <rjsberry@proton.me>
//
// Permission to use, copy, modify, and/or distribute this software for any
// purpose with or without fee is hereby granted.
//
// THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
// WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
// MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
// ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
// WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
// ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
// OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.

//! A zero allocation recursive descent JSON deserializer.
//!
//! This library is part of [`quick.rs`]; it is a single file, dependency
//! free, `no_std` library with a [0BSD] license (no attribution required).
//!
//! # Installation
//!
//! The quickest way to start using it is to copy the [source code] into
//! your project, then declare it as a module:
//!
//! ```ignore
//! mod qjson;
//! ```
//!
//! Alternatively, the crate is also published to [crates.io] and can be
//! added to your `Cargo.toml` instead:
//!
//! ```toml
//! [dependencies]
//! qjson = "1.0.0-beta.1"
//! ```
//!
//! [`quick.rs`]: https://github.com/rjsberry/quick.rs
//! [0BSD]: https://choosealicense.com/licenses/0bsd/
//! [source code]: ../src/qjson/qjson.rs.html
//! [crates.io]: https://crates.io

#![no_std]

use core::str::Chars;

#[cfg(not(debug_assertions))]
use core::hint;

use self::{ErrorKind::*, Token::*};

/// Deserialize a JSON string.
///
/// The schema `desc` describes how the JSON data should be deserialized;
/// this contains a series of mutable references which are written to as
/// objects are encountered within the data. For more information please
/// consult the crate level documentation.
///
/// `D` specifies the max recursion traversed by the parser before exiting
/// with an error. Use this to maintain strict control over how much stack
/// the parser consumes.
///
/// # Example
///
/// ```
/// # fn _example() -> Result<(), qjson::Error> {
/// #[derive(Default)]
/// struct PowerModule<'a> {
///     description: Option<&'a str>,
///     adc_channels: [Option<i64>; 2],
///     capacity: Option<f64>,
/// }
///
/// let src = r#"{
///     "description": "battery backup",
///     "adc_channels": [1, 2],
///     "capacity": 5000.0
/// }"#;
///
/// let mut power_module = PowerModule::default();
/// let PowerModule {
///     description,
///     adc_channels: [volt, curr],
///     capacity,
/// } = &mut power_module;
///
/// // Create a description of the expected JSON data with references to the
/// // fields of the structure to deserialize into
/// let mut adc_chan_desc = [
///     qjson::Schema::Integer(volt),
///     qjson::Schema::Integer(curr),
/// ];
/// let mut power_module_desc = [
///     ("description", qjson::Schema::Str(description)),
///     ("adc_channels", qjson::Schema::Array(&mut adc_chan_desc)),
///     ("capacity", qjson::Schema::Float(capacity)),
/// ];
///
/// // Specify the max recursion depth at the call site
/// qjson::from_str::<_, 1>(src, &mut power_module_desc)?;
///
/// assert_eq!(power_module.description, Some("battery backup"));
/// assert_eq!(power_module.adc_channels, [Some(1), Some(2)]);
/// assert_eq!(power_module.capacity, Some(5000.0));
/// # Ok(())
/// # }
/// # _example().unwrap();
/// ```
pub fn from_str<'a: 'b, 'b, S, const D: usize>(json: &'a str, desc: S) -> Result<(), Error>
where
    S: Into<Schema<'a, 'b>>,
{
    Parser::<D>::new(json).parse(Some(&mut desc.into()))
}

/// Validate a JSON string.
pub fn validate<'a, const D: usize>(json: &'a str) -> Result<(), Error> {
    Parser::<D>::new(json).parse(None)
}

#[derive(Debug)]
pub enum Schema<'a, 'b> {
    Array(&'b mut [Schema<'a, 'b>]),
    Bool(&'b mut Option<bool>),
    Float(&'b mut Option<f64>),
    Integer(&'b mut Option<i64>),
    Object(&'b mut [(&'b str, Schema<'a, 'b>)]),
    Str(&'b mut Option<&'a str>),
}

#[derive(Debug, Clone)]
pub struct Error {
    lineno: usize,
    col: usize,
    kind: ErrorKind,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ErrorKind {
    InsufficientArrayLength,
    InvalidNumber,
    MaxDepthExceeded,
    MismatchedTypes,
    MissingComma,
    UnexpectedControlCharacterInString,
    UnexpectedEof,
    UnexpectedToken,
    UnexpectedTrailingComma,
    UnknownIdentifier,
    UnknownStartOfToken,
    UnterminatedString,
}

struct Parser<'a, const D: usize> {
    tok: Tokenizer<'a>,
    peek: Option<Token<'a>>,
}

trait Clear {
    fn clear(&mut self);
}

struct Tokenizer<'a> {
    lineno: usize,
    col: usize,
    chars: Chars<'a>,
    prev: &'a str,
}

#[derive(Copy, Clone, PartialEq)]
enum Token<'a> {
    Bool(bool),
    BraceL,
    BraceR,
    BracketL,
    BracketR,
    Colon,
    Comma,
    Float(f64),
    Integer(i64),
    Null,
    Str(&'a str),
}

// impl Error

impl Error {
    /// Retrieves the line number the error was encountered on.
    pub fn lineno(&self) -> usize {
        self.lineno
    }

    /// Retrieves the column the error was encountered on.
    pub fn col(&self) -> usize {
        self.col
    }

    /// Retrieves the kind of error that occurred.
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
}

// impl Schema

impl<'a, 'b> From<&'b mut [Schema<'a, 'b>]> for Schema<'a, 'b> {
    fn from(desc: &'b mut [Schema<'a, 'b>]) -> Self {
        Self::Array(desc)
    }
}

impl<'a, 'b, const N: usize> From<&'b mut [Schema<'a, 'b>; N]> for Schema<'a, 'b> {
    fn from(desc: &'b mut [Schema<'a, 'b>; N]) -> Self {
        Self::Array(&mut desc[..])
    }
}

impl<'b> From<&'b mut Option<bool>> for Schema<'_, 'b> {
    fn from(desc: &'b mut Option<bool>) -> Self {
        Self::Bool(desc)
    }
}

impl<'b> From<&'b mut Option<f64>> for Schema<'_, 'b> {
    fn from(desc: &'b mut Option<f64>) -> Self {
        Self::Float(desc)
    }
}

impl<'b> From<&'b mut Option<i64>> for Schema<'_, 'b> {
    fn from(desc: &'b mut Option<i64>) -> Self {
        Self::Integer(desc)
    }
}

impl<'a, 'b> From<&'b mut [(&'b str, Schema<'a, 'b>)]> for Schema<'a, 'b> {
    fn from(desc: &'b mut [(&'b str, Schema<'a, 'b>)]) -> Self {
        Self::Object(desc)
    }
}

impl<'a, 'b, const N: usize> From<&'b mut [(&'b str, Schema<'a, 'b>); N]> for Schema<'a, 'b> {
    fn from(desc: &'b mut [(&'b str, Schema<'a, 'b>); N]) -> Self {
        Self::Object(&mut desc[..])
    }
}

impl<'a, 'b> From<&'b mut Option<&'a str>> for Schema<'a, 'b> {
    fn from(desc: &'b mut Option<&'a str>) -> Self {
        Self::Str(desc)
    }
}

impl Clear for Option<&mut [(&str, Schema<'_, '_>)]> {
    fn clear(&mut self) {
        if let Some(desc) = self {
            for (_, val) in desc.iter_mut() {
                val.clear();
            }
        }
    }
}

impl Clear for Option<&mut [Schema<'_, '_>]> {
    fn clear(&mut self) {
        if let Some(desc) = self {
            for val in desc.iter_mut() {
                val.clear();
            }
        }
    }
}

impl Schema<'_, '_> {
    fn clear(&mut self) {
        match self {
            Self::Array(a) => {
                for v in a.iter_mut() {
                    v.clear();
                }
            }
            Self::Bool(b) => **b = None,
            Self::Float(f) => **f = None,
            Self::Integer(i) => **i = None,
            Self::Object(desc) => {
                for (_, v) in desc.iter_mut() {
                    v.clear();
                }
            }
            Self::Str(s) => **s = None,
        }
    }
}

// impl Parser

impl<'a, const D: usize> Parser<'a, D> {
    fn new(json: &'a str) -> Self {
        Self {
            tok: Tokenizer::new(json),
            peek: None,
        }
    }

    fn parse(&mut self, desc: Option<&mut Schema<'a, '_>>) -> Result<(), Error> {
        self.parse_value(desc, 0)?;
        self.assume_complete()
    }

    fn parse_value(
        &mut self,
        desc: Option<&mut Schema<'a, '_>>,
        depth: usize,
    ) -> Result<(), Error> {
        match (self.next_tok()?, desc) {
            (BraceL, Some(Schema::Object(v))) => {
                self.parse_obj(Some(*v), depth + 1)?;
            }
            (BraceL, None) => self.parse_obj(None, depth + 1)?,

            (BracketL, Some(Schema::Array(a))) => self.parse_array(Some(a), depth)?,
            (BracketL, None) => self.parse_array(None, depth)?,

            (Bool(b), Some(Schema::Bool(v))) => **v = Some(b),
            (Bool(_), None) => (),

            (Float(f), Some(Schema::Float(v))) => **v = Some(f),
            (Float(_), None) => (),

            (Integer(i), Some(Schema::Float(v))) => **v = Some(i as f64),
            (Integer(i), Some(Schema::Integer(v))) => **v = Some(i),
            (Integer(_), None) => (),

            (Null, Some(v)) => v.clear(),
            (Null, None) => (),

            (Str(s), Some(Schema::Str(v))) => **v = Some(s),
            (Str(_), None) => (),

            (BraceR, _) | (BracketR, _) | (Comma, _) | (Colon, _) => {
                return Err(self.tok.err(UnexpectedToken));
            }

            _ => return Err(self.tok.err(MismatchedTypes)),
        }
        Ok(())
    }

    fn parse_obj(
        &mut self,
        mut obj: Option<&mut [(&str, Schema<'a, '_>)]>,
        depth: usize,
    ) -> Result<(), Error> {
        if depth > D {
            return Err(self.tok.err(MaxDepthExceeded));
        }

        if self.advance_if_tok(BraceR)? {
            obj.clear();
        } else {
            loop {
                let field = self.assume_tok_str()?;
                self.assume_tok_kind(Colon)?;
                let val = obj.as_mut().and_then(|desc| {
                    desc.iter_mut()
                        .find_map(|(k, v)| Some(v).filter(|_| *k == field))
                });

                self.parse_value(val, depth)?;
                if self.end_of_collection(BraceR)? {
                    break;
                }
            }
        }

        Ok(())
    }

    fn parse_array(
        &mut self,
        mut arr: Option<&mut [Schema<'a, '_>]>,
        depth: usize,
    ) -> Result<(), Error> {
        if self.advance_if_tok(BracketR)? {
            arr.clear();
            Ok(())
        } else {
            let mut i = 0;
            loop {
                let val = arr
                    .as_mut()
                    .map(|desc| {
                        desc.get_mut(i)
                            .ok_or_else(|| self.tok.err(InsufficientArrayLength))
                    })
                    .transpose()?;

                self.parse_value(val, depth)?;
                if self.end_of_collection(BracketR)? {
                    return Ok(());
                }

                i += 1;
            }
        }
    }

    fn end_of_collection(&mut self, with: Token<'a>) -> Result<bool, Error> {
        match (self.advance_if_tok(Comma)?, self.advance_if_tok(with)?) {
            (false, true) => Ok(true),
            (true, false) => Ok(false),
            (true, true) => Err(self.tok.err(UnexpectedTrailingComma)),
            (false, false) => Err(self.tok.err(MissingComma)),
        }
    }

    fn assume_tok_kind(&mut self, tok: Token<'_>) -> Result<(), Error> {
        if self.next_tok()? != tok {
            return Err(self.tok.err(UnexpectedToken));
        }
        Ok(())
    }

    fn assume_tok_str(&mut self) -> Result<&'a str, Error> {
        match self.next_tok()? {
            Str(s) => Ok(s),
            _ => Err(self.tok.err(UnexpectedToken)),
        }
    }

    fn assume_complete(&mut self) -> Result<(), Error> {
        if self.tok.next().is_some() {
            return Err(self.tok.err(UnexpectedToken));
        }
        Ok(())
    }

    fn advance_if_tok(&mut self, tok: Token<'_>) -> Result<bool, Error> {
        if *self.peek_next_tok()? == tok {
            self.peek = None;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn next_tok(&mut self) -> Result<Token<'a>, Error> {
        if let Some(tok) = self.peek.take() {
            Ok(tok)
        } else {
            self.tok.next().ok_or_else(|| self.tok.err(UnexpectedEof))?
        }
    }

    fn peek_next_tok(&mut self) -> Result<&Token<'a>, Error> {
        if let Some(ref tok) = self.peek {
            Ok(tok)
        } else {
            self.peek = Some(
                self.tok
                    .next()
                    .transpose()?
                    .ok_or_else(|| self.tok.err(UnexpectedEof))?,
            );

            #[cfg(debug_assertions)]
            let tok = self.peek.as_ref().unwrap();
            #[cfg(not(debug_assertions))]
            let tok = match self.peek {
                Some(ref tok) => tok,
                // Safety: We just set `self.peek` to `Some`
                None => unsafe { hint::unreachable_unchecked() },
            };

            Ok(tok)
        }
    }
}

// impl Tokenizer

impl<'a> Tokenizer<'a> {
    fn new(json: &'a str) -> Self {
        Self {
            lineno: 1,
            col: 0,
            chars: json.chars(),
            prev: json,
        }
    }

    /// The source string *not* including the most recently popped char.
    fn as_str(&self) -> &'a str {
        self.chars.as_str()
    }

    /// The source string *including* the most recently popped char.
    fn as_str_prev(&self) -> &'a str {
        self.prev
    }

    /// Pop the next character from the source iterator if one exists.
    #[inline]
    fn next_char(&mut self) -> Option<char> {
        self.prev = self.chars.as_str();
        let c = self.chars.next()?;
        self.col += 1;
        Some(c)
    }

    /// Emit an error at the current line and column number.
    fn err(&self, kind: ErrorKind) -> Error {
        Error {
            lineno: self.lineno,
            col: self.col,
            kind,
        }
    }

    /// Emit the given token if the sequence of characters is matched.
    fn tok_chars(&mut self, cs: &[char], tok: Token<'a>) -> Result<Token<'a>, Error> {
        for &c in cs {
            if self.next_char().ok_or_else(|| self.err(UnexpectedEof))? != c {
                return Err(self.err(UnknownIdentifier));
            }
        }
        Ok(tok)
    }

    /// Attempt to emit a string token.
    fn tok_string(&mut self) -> Result<Token<'a>, Error> {
        let s = self.as_str();

        let mut len = 0;
        let mut escape = false;

        loop {
            match self
                .next_char()
                .ok_or_else(|| self.err(UnterminatedString))?
            {
                '"' if !escape => break,
                '\\' => escape = true,
                c if c.is_control() => return Err(self.err(UnexpectedControlCharacterInString)),
                _ => escape = false,
            }
            len += 1;
        }

        #[cfg(debug_assertions)]
        let s = &s[..len];
        #[cfg(not(debug_assertions))]
        // Safety: We know `len` is within the length of `s`
        let s = unsafe { s.get_unchecked(..len) };

        Ok(Str(s))
    }

    /// Attempt to emit a numeric (*either* integer or float) token.
    fn tok_number(&mut self) -> Result<Token<'a>, Error> {
        let s = self.as_str_prev();

        let mut len = 0_usize;
        let mut float = false;

        let mut cs = s.chars();
        while let Some(c) = cs.next() {
            if !matches!(c, '0'..='9' | '.' | '-') {
                break;
            }
            len += 1;
        }

        // The iterator impl already advanced past the first character
        let advance_by = len.saturating_sub(1);
        for i in 0..advance_by {
            if self.next_char().ok_or_else(|| self.err(UnexpectedEof))? == '.' {
                float = true;
                // Floats ending with periods parse, but are not valid JSON
                if i == advance_by {
                    return Err(self.err(InvalidNumber));
                }
            }
        }

        #[cfg(debug_assertions)]
        let n = &s[..len];
        #[cfg(not(debug_assertions))]
        // Safety: We know `len` is within the length of `s`
        let n = unsafe { s.get_unchecked(..len) };

        // FIXME: Significant performance hit using `libcore` conversions here
        // FIXME: `f64` parsing from `libcore` has panic paths
        if float {
            let f: f64 = n.parse().map_err(|_| self.err(InvalidNumber))?;
            Ok(Float(f))
        } else {
            let i: i64 = n.parse().map_err(|_| self.err(InvalidNumber))?;
            Ok(Integer(i))
        }
    }
}

impl<'a> Iterator for Tokenizer<'a> {
    type Item = Result<Token<'a>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.next_char()? {
                ' ' | '\t' | '\r' => (),
                '\n' => {
                    self.lineno += 1;
                    self.col = 0;
                }

                '{' => return Some(Ok(BraceL)),
                '}' => return Some(Ok(BraceR)),
                '[' => return Some(Ok(BracketL)),
                ']' => return Some(Ok(BracketR)),
                ':' => return Some(Ok(Colon)),
                ',' => return Some(Ok(Comma)),

                't' => return Some(self.tok_chars(&['r', 'u', 'e'], Bool(true))),
                'f' => return Some(self.tok_chars(&['a', 'l', 's', 'e'], Bool(false))),
                'n' => return Some(self.tok_chars(&['u', 'l', 'l'], Null)),

                '"' => return Some(self.tok_string()),

                '0'..='9' | '-' => return Some(self.tok_number()),

                _ => return Some(Err(self.err(UnknownStartOfToken))),
            }
        }
    }
}
