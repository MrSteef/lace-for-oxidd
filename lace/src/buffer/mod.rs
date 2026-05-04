#[cfg(not(feature = "unsafe_buffer"))]
pub mod safe;
#[cfg(not(feature = "unsafe_buffer"))]
pub use safe as picked;

#[cfg(feature = "unsafe_buffer")]
pub mod r#unsafe;
#[cfg(feature = "unsafe_buffer")]
pub use r#unsafe as picked;
