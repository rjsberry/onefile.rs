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

//! An opinionated zero-copy `no_std` .INI parser.
//!
//! # Installation
//!
//! Copy **[ini.rs]** into your Rust project and include it for compilation:
//!
//! ```ignore
//! mod ini;
//! ```
//!
//! # Usage
//!
//! Use [qini::parse] to iterate through key/value pairs in your .INI
//! configuration file.
//!
//! # Grammar
//!
//! * Comments begin with `;` or `#` and must exist on their own line (inline
//!   comments are not supported).
//! * Global key/value pairs can exist outside sections.
//! * Values are delimited by the first `=` or `:` character encountered.
//! * Multi-line values are not supported.
//! * Indentation is ignored.
//! * Section and key names must contain only ASCII alphanumerics,
//!   underscores, and periods.
//! * Keys can have no value, but a valid delimiter must be present on the
//!   line.
//! * Duplicate sections and keys do not cause errors.
//!
//! [ini.rs]: ../src/ini/ini.rs.html
//! [qini::parse]: fn.parse.html

#![no_std]

use core::iter::{Enumerate, Iterator};
use core::str::Lines;

use self::ErrorKind::*;

/// Parses .INI configuration.
///
/// # Examples
///
/// ```
/// const CONFIGURATION: &str = r#"
/// description = google's primary dns server
///
/// [server]
/// ip   = 8.8.8.8
/// port = 53
/// "#;
///
/// #[derive(Default)]
/// struct Config<'a> {
///     description: &'a str,
///     ip: (u8, u8, u8, u8),
///     port: u16,
/// }
///
/// let mut config = Config::default();
/// let mut iter = qini::parse(CONFIGURATION);
///
/// while let Some(Ok(qini::Param { section, key, value })) = iter.next() {
///     match (section, key) {
///         ("", "description") => config.description = value,
///
///         ("server", "ip") => {
///             let mut value_iter = value.split('.');
///             let mut next = || value_iter.next().and_then(|val| val.parse().ok());
///             let chain = [next(), next(), next(), next(), next()];
///             if let [Some(a), Some(b), Some(c), Some(d), None] = chain {
///                 config.ip = (a, b, c, d);
///             }
///         }
///
///         ("server", "port") => {
///             if let Ok(port) = value.parse() {
///                 config.port = port;
///             }
///         }
///
///         _ => (),
///     }
/// }
///
/// assert_eq!(config.description, "google's primary dns server");
/// assert_eq!(config.ip, (8, 8, 8, 8));
/// assert_eq!(config.port, 53);
/// ```
pub fn parse(ini: &str) -> impl Iterator<Item = Result<Param<'_>, Error>> {
    Parser::new(ini)
}

/// .INI configuration parameter.
#[derive(Debug)]
pub struct Param<'a> {
    /// The section the parameter was found in.
    ///
    /// Global key/value parameters have no section; will have an empty string
    /// in this field.
    pub section: &'a str,

    /// The parameter key.
    pub key: &'a str,

    /// The parameter value.
    ///
    /// Parameters with no value will have an empty string in this field.
    pub value: &'a str,
}

/// Error encountered while parsing .INI configuration files.
#[derive(Debug, Clone)]
pub struct Error {
    lineno: usize,
    kind: ErrorKind,
}

/// Specific types of errors.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ErrorKind {
    /// Section contains invalid characters.
    InvalidSection,

    /// Key contains invalid characters.
    InvalidKey,

    /// The parser reached the end of the line.
    UnexpectedEol,
}

struct Parser<'a> {
    lines: Enumerate<Lines<'a>>,
    section: &'a str,
}

fn is_valid_ident(ident: &str) -> bool {
    !ident.is_empty()
        && !ident.contains(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '.'))
}

// impl Error

impl Error {
    /// Retrieves the line number the error was encountered on.
    pub fn lineno(&self) -> usize {
        self.lineno
    }

    /// Retrieves the kind of error that occurred.
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
}

// impl Parser

impl<'a> Parser<'a> {
    fn new(src: &'a str) -> Self {
        Self {
            lines: src.lines().enumerate(),
            section: "",
        }
    }

    fn parse_section(&mut self, section_start: &'a str) -> Result<(), ErrorKind> {
        let section = section_start.strip_suffix(']').ok_or(UnexpectedEol)?.trim();

        if !is_valid_ident(section) {
            return Err(InvalidSection);
        }

        self.section = section;
        Ok(())
    }

    fn parse_param(&self, line: &'a str) -> Result<Param<'a>, ErrorKind> {
        let (mut prefix, mut suffix) = line.split_once(['=', ':']).ok_or(UnexpectedEol)?;

        prefix = prefix.trim();
        suffix = suffix.trim();

        if !is_valid_ident(prefix) {
            return Err(InvalidKey);
        }

        Ok(Param {
            section: self.section,
            key: prefix,
            value: suffix,
        })
    }
}

impl<'a> Iterator for Parser<'a> {
    type Item = Result<Param<'a>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (lineno, mut line) = self.lines.next()?;
            let map_err = |kind| Error {
                lineno: lineno + 1,
                kind,
            };

            line = line.trim();

            if !matches!(line.chars().next(), Some('#') | Some(';') | None) {
                if let Some(section_start) = line.strip_prefix('[') {
                    if let Err(kind) = self.parse_section(section_start) {
                        return Some(Err(map_err(kind)));
                    }
                } else {
                    return Some(self.parse_param(line).map_err(map_err));
                }
            }
        }
    }
}
