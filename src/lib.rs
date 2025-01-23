// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

pub mod bytecode_reader;
pub mod bytecode_writer;
pub mod common_sections;
pub mod entry;
pub mod entry_reader;
pub mod entry_writer;
pub mod index_sections;
pub mod module_image;
pub mod datatableaccess;

// https://doc.rust-lang.org/reference/conditional-compilation.html#debug_assertions
// https://doc.rust-lang.org/reference/conditional-compilation.html#test
#[cfg(debug_assertions)]
pub mod utils;

use std::{
    fmt::Display,
    hash::{DefaultHasher, Hasher},
};

// the hash of parameters and compile environment variables,
// only exists in Local/Remote/Share dependencies
//
// by default the hash is computed by the Rust default
// hasher (SipHash, https://en.wikipedia.org/wiki/SipHash),
// it can be also by the FNV (https://en.wikipedia.org/wiki/Fowler-Noll-Vo_hash_function).
//
// not all bits are always used, only the first 64 bits are used by default.
pub type DependencyHash = [u8; 32];

pub const ZERO_DEPENDENCY_HASH: DependencyHash = [0u8; 32];

#[derive(Debug)]
pub struct ImageError {
    // message: String,
    pub error_type: ImageErrorType,
}

#[derive(Debug)]
pub enum ImageErrorType {
    InvalidImage,
    RequireNewVersionRuntime,
}

impl ImageError {
    pub fn new(error_type: ImageErrorType) -> Self {
        Self { error_type }
    }
}

impl Display for ImageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.error_type {
            ImageErrorType::InvalidImage => write!(f, "Not a valid module image."),
            ImageErrorType::RequireNewVersionRuntime => {
                write!(f, "The version of module image is newer than runtime.")
            }
        }
    }
}

impl std::error::Error for ImageError {}

pub fn compute_dependency_hash(values: &str) -> DependencyHash {
    let mut hasher = DefaultHasher::new();
    hasher.write(values.as_bytes());
    let value = hasher.finish();

    let mut buf = ZERO_DEPENDENCY_HASH;
    let bytes = value.to_le_bytes();
    let src = bytes.as_ptr();
    let dst = buf.as_mut_ptr();
    unsafe { std::ptr::copy(src, dst, bytes.len()) };
    buf
}

pub fn format_dependency_hash(hash: &DependencyHash) -> String {
    hash[..8]
        .iter()
        .map(|value| format!("{:02x}", value))
        .collect::<Vec<String>>()
        .join("")
}
