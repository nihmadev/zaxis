//! Why a material was rejected.

use std::{error::Error, fmt, sync::Arc};

/// Which stage rejected a material. New kinds may be added.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MaterialErrorKind {
    /// The parameter schema is invalid or too large, or values do not match it.
    Schema,
    /// The WGSL source does not parse.
    Syntax,
    /// The WGSL source parses but is not a valid module.
    Validation,
    /// The source declares something the fixed binding set does not allow: a resource,
    /// an override, or an entry point of its own.
    Forbidden,
    /// The registry holds [`MAX_MATERIALS`](super::MAX_MATERIALS) materials already.
    Limit,
}

/// A material that cannot be used, with a message that points into the author's own source.
///
/// `line` and `column` count from 1 in the fragment source passed to
/// [`Material::new`](super::Material::new), not in the assembled module; they are `None`
/// for errors that have no source position.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaterialError {
    kind: MaterialErrorKind,
    label: Arc<str>,
    message: String,
    line: Option<u32>,
    column: Option<u32>,
}

impl MaterialError {
    pub(crate) fn new(kind: MaterialErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            label: Arc::from(""),
            message: message.into(),
            line: None,
            column: None,
        }
    }

    pub(crate) fn at(mut self, line: u32, column: u32) -> Self {
        self.line = Some(line);
        self.column = Some(column);
        self
    }

    pub(crate) fn labeled(mut self, label: &Arc<str>) -> Self {
        self.label = Arc::clone(label);
        self
    }

    pub fn kind(&self) -> MaterialErrorKind {
        self.kind
    }
    /// The label of the [`Material`](super::Material), empty before it is known.
    pub fn label(&self) -> &str {
        &self.label
    }
    pub fn message(&self) -> &str {
        &self.message
    }
    pub fn line(&self) -> Option<u32> {
        self.line
    }
    pub fn column(&self) -> Option<u32> {
        self.column
    }
}

impl fmt::Display for MaterialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "material")?;
        if !self.label.is_empty() {
            write!(f, " `{}`", self.label)?;
        }
        if let (Some(line), Some(column)) = (self.line, self.column) {
            write!(f, " (line {line}, column {column})")?;
        }
        write!(f, ": {}", self.message)
    }
}

impl Error for MaterialError {}
