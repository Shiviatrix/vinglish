/// A half-open byte-offset range `[start, end)` in a source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    /// Creates a new `Span` with the given start and end offsets.
    #[inline]
    pub fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    /// A span that points nowhere — used for synthetic tokens.
    #[inline]
    pub fn dummy() -> Self {
        Self { start: 0, end: 0 }
    }

    /// Smallest span that contains both `self` and `other`.
    #[inline]
    pub fn merge(self, other: Self) -> Self {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    /// Returns the length of the span in bytes.
    pub fn len(self) -> u32 {
        self.end.saturating_sub(self.start)
    }

    /// Returns true if the span has a length of 0.
    pub fn is_empty(self) -> bool {
        self.len() == 0
    }
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

/// Any value paired with the source location it came from.
#[derive(Debug, Clone, PartialEq)]
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    /// Creates a new `Spanned` node with the given span.
    #[inline]
    pub fn new(node: T, span: Span) -> Self {
        Self { node, span }
    }

    /// Maps the inner node value to a new type using the provided function, preserving the span.
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Spanned<U> {
        Spanned {
            node: f(self.node),
            span: self.span,
        }
    }
}
