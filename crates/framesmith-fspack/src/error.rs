//! Error types for FSPK parsing.

/// Errors that can occur when parsing an FSPK pack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The input data is too short to contain a valid header.
    TooShort,
    /// The magic bytes do not match the expected FSPK signature.
    InvalidMagic,
    /// An offset or length field points outside the data.
    OutOfBounds,
    /// Unsupported version/flags; re-export with a compatible writer.
    UnsupportedVersion,
    /// Invalid section layout, record size or data reference.
    InvalidFormat,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::TooShort => "FSPK header is truncated",
            Self::InvalidMagic => "invalid FSPK magic",
            Self::OutOfBounds => "FSPK reference is out of bounds",
            Self::UnsupportedVersion => "unsupported FSPK version/flags",
            Self::InvalidFormat => "invalid FSPK record layout or reference",
        })
    }
}

impl core::error::Error for Error {}
