//! Feeds generated WGSL through a real front end.

#[path = "naga/common.rs"]
pub mod naga_common;

#[path = "naga/primitives_and_blends.rs"]
mod primitives_and_blends;

#[path = "naga/camera_and_amplify.rs"]
mod camera_and_amplify;

#[path = "naga/fields_and_layouts.rs"]
mod fields_and_layouts;

#[path = "naga/post_and_textures.rs"]
mod post_and_textures;
