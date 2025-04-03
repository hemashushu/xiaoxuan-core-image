// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions.
// For more details, see the LICENSE, LICENSE.additional, and CONTRIBUTING files.

pub mod bytecode_reader;
pub mod bytecode_writer;
pub mod common_sections;
pub mod datatableaccess;
pub mod entry;
pub mod entry_reader;
pub mod entry_writer;
pub mod index_sections;
pub mod module_image;

// Conditional compilation for debug utilities.
// See: https://doc.rust-lang.org/reference/conditional-compilation.html#debug_assertions
// See: https://doc.rust-lang.org/reference/conditional-compilation.html#test
#[cfg(debug_assertions)]
pub mod utils;

use std::{
    fmt::Display,
    hash::{DefaultHasher, Hasher},
};

// Represents the hash of parameters and compile environment variables.
// This is used in Local/Remote/Share dependencies.
//
// By default, the hash is computed using Rust's default hasher (SipHash).
// Reference: https://en.wikipedia.org/wiki/SipHash
//
// Alternatively, the hash can be computed using FNV.
// Reference: https://en.wikipedia.org/wiki/Fowler-Noll-Vo_hash_function
//
// Note: Not all bits of the hash are always used. By default, only the first 64 bits are utilized.
pub type DependencyHash = [u8; 32];

// A constant representing a zeroed dependency hash.
pub const DEPENDENCY_HASH_ZERO: DependencyHash = [0u8; 32];

#[derive(Debug)]
pub struct ImageError {
    // Represents the type of error encountered.
    pub error_type: ImageErrorType,
}

#[derive(Debug)]
pub enum ImageErrorType {
    // Indicates that the module image is invalid.
    InvalidImage,
    // Indicates that the module image requires a newer runtime version.
    RequireNewVersionRuntime,
}

impl ImageError {
    // Creates a new ImageError with the specified error type.
    pub fn new(error_type: ImageErrorType) -> Self {
        Self { error_type }
    }
}

impl Display for ImageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.error_type {
            ImageErrorType::InvalidImage => write!(f, "Not a valid module image."),
            ImageErrorType::RequireNewVersionRuntime => {
                write!(
                    f,
                    "The version of the module image is newer than the runtime."
                )
            }
        }
    }
}

impl std::error::Error for ImageError {}

// Computes a dependency hash from the given string input.
// The hash is generated using Rust's default hasher (SipHash).
pub fn compute_dependency_hash(values: &str) -> DependencyHash {
    let mut hasher = DefaultHasher::new();
    hasher.write(values.as_bytes());
    let value = hasher.finish();

    let mut buf = DEPENDENCY_HASH_ZERO;
    let bytes = value.to_le_bytes();
    let src = bytes.as_ptr();
    let dst = buf.as_mut_ptr();
    unsafe { std::ptr::copy(src, dst, bytes.len()) };
    buf
}

// Formats the first 64 bits of a dependency hash as a hexadecimal string.
pub fn format_dependency_hash(hash: &DependencyHash) -> String {
    hash[..8]
        .iter()
        .map(|value| format!("{:02x}", value))
        .collect::<Vec<String>>()
        .join("")
}
