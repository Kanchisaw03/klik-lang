// KLIK Standard Library
// Core modules: io, math, strings, collections, filesystem, time, networking

pub mod collections;
pub mod fs;
pub mod io;
pub mod math;
pub mod net;
pub mod strings;
pub mod time;

/// Standard library initialization
pub fn init() {
    klik_runtime::init_runtime();
}
