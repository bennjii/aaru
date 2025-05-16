#![doc = include_str!("../README.md")]
#![allow(dead_code)]

#[cfg(feature = "mimalloc")]
use mimalloc::MiMalloc;
#[cfg_attr(feature = "mimalloc", global_allocator)]
#[cfg(feature = "mimalloc")]
static GLOBAL: MiMalloc = MiMalloc;

pub mod route;
pub use route::*;
