//! Over a socket, because everything else here passed with the server deleted.

#[path = "wire/common.rs"]
pub mod wire_common;

#[path = "wire/protocol_and_http.rs"]
mod protocol_and_http;

#[path = "wire/tools_and_save.rs"]
mod tools_and_save;

#[path = "wire/library_and_sets.rs"]
mod library_and_sets;

#[path = "wire/routing.rs"]
mod routing;

#[path = "wire/operations.rs"]
mod operations;
