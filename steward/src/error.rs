//! Error types

use abscissa_core::error::{BoxError, Context};
use deep_space::error::{CosmosGrpcError, PrivateKeyError};
use ethers::{contract::ContractError, middleware::gas_oracle::GasOracleError, prelude::*};
use std::{
    fmt::{self, Display},
    io,
    ops::Deref,
};
use thiserror::Error;

use tonic::transport::Error as TonicError;

/// Kinds of errors
#[derive(Copy, Clone, Debug, Eq, Error, PartialEq)]
pub enum ErrorKind {
    /// Error in configuration file
    #[error("config error")]
    Config,
    /// Contract error
    #[error("contract error")]
    ContractError,
    /// Input/output error
    #[error("I/O error")]
    Io,
    /// Gas Oracle error
    #[error("gas error")]
    GasOracle,
    /// Provider error
    #[error("grpc error")]
    GrpcError,
    /// Input/output error
    #[error("http error")]
    Http,
    /// Cryptographic Keys error
    #[error("key related error")]
    KeysError,
    /// Miscellaneous error
    ///
    /// Errors that are returned with types that provide no
    /// categorical information, such as String
    #[error("allocation error")]
    MiscError,
    /// Provider error
    #[error("provider error")]
    ProviderError,
}

impl ErrorKind {
    /// Create an error context from this error
    pub fn context(self, source: impl Into<BoxError>) -> Context<ErrorKind> {
        Context::new(self, Some(source.into()))
    }
}

/// Error type
#[derive(Debug)]
pub struct Error(Box<Context<ErrorKind>>);

impl Deref for Error {
    type Target = Context<ErrorKind>;

    fn deref(&self) -> &Context<ErrorKind> {
        &self.0
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self {
        Context::new(kind, None).into()
    }
}

impl From<Context<ErrorKind>> for Error {
    fn from(context: Context<ErrorKind>) -> Self {
        Error(Box::new(context))
    }
}

impl From<CosmosGrpcError> for Error {
    fn from(err: CosmosGrpcError) -> Self {
        ErrorKind::GrpcError.context(err).into()
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        ErrorKind::Io.context(err).into()
    }
}

impl From<iqhttp::Error> for Error {
    fn from(err: iqhttp::Error) -> Self {
        ErrorKind::Http.context(err).into()
    }
}

impl From<GasOracleError> for Error {
    fn from(err: GasOracleError) -> Self {
        ErrorKind::GasOracle.context(err).into()
    }
}

impl<T: 'static + Middleware> From<ContractError<T>> for Error {
    fn from(err: ContractError<T>) -> Self {
        let err: BoxError = err.into();
        ErrorKind::ContractError.context(err).into()
    }
}

impl From<PrivateKeyError> for Error {
    fn from(err: PrivateKeyError) -> Self {
        ErrorKind::KeysError.context(err).into()
    }
}

impl From<ProviderError> for Error {
    fn from(err: ProviderError) -> Self {
        let err: BoxError = err.into();
        ErrorKind::ContractError.context(err).into()
    }
}

impl From<String> for Error {
    fn from(msg: String) -> Self {
        ErrorKind::MiscError.context(msg).into()
    }
}

impl From<TonicError> for Error {
    fn from(err: TonicError) -> Self {
        let err: BoxError = err.into();
        ErrorKind::GrpcError.context(err).into()
    }
}