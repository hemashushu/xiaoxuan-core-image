// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

pub mod bytecode_reader;
pub mod bytecode_writer;
pub mod common_sections;
pub mod entry;
pub mod index_sections;
pub mod module_image;
pub mod tableaccess;

// https://doc.rust-lang.org/reference/conditional-compilation.html#debug_assertions
// https://doc.rust-lang.org/reference/conditional-compilation.html#test
#[cfg(debug_assertions)]
pub mod utils;

use std::fmt::Display;

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
