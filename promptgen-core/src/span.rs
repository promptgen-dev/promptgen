use std::ops::Range;

/// Span in the original source (byte offsets).
/// You can add line/col later or compute them on demand.
pub type Span = Range<usize>;

/// A value annotated with its span.
pub type Spanned<T> = (T, Span);
