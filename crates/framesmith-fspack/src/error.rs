//! Error types for FSPK parsing.

/// Errors that can occur when parsing an FSPK pack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The input data is too short to contain a valid header.
    TooShort,
    /// The magic bytes do not match the expected FSPK signature.
    InvalidMagic,
    /// The version is not supported.
    UnsupportedVersion,
    /// An offset or length field points outside the data.
    OutOfBounds,
}
