use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// dxc (invoked as a command line tool) failed to compile a shader.
    #[error("dxc failed: {message}")]
    Dxc { message: String },

    /// A spirv-cross C API call failed.
    #[error("spirv-cross: {function} failed: {message}")]
    Spvc {
        function: &'static str,
        message: String,
    },

    /// Invalid parameter or malformed input.
    #[error("{0}")]
    InvalidParameter(String),

    /// A required dependency could not be found or executed.
    #[error("could not run {path}: {source}")]
    DxcLaunch {
        path: PathBuf,
        source: std::io::Error,
    },

    /// Destination format that this build cannot produce (DXBC).
    #[error(
        "DXBC (shader model 5) output is not supported by this build; \
         only dxc-based DXIL output is available"
    )]
    UnsupportedDestination,

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
