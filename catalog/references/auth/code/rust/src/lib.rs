//! The callers of the API, verified from short-lived access tokens signed by the service, and
//! longer sessions kept by refresh tokens that the server stores and replaces at each use.

pub mod sessions;
pub mod settings;
pub mod tokens;
