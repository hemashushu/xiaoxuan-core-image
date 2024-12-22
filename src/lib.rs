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
    message: String,
}

impl ImageError {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_owned(),
        }
    }
}

impl Display for ImageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Binary error: {}", self.message)
    }
}

impl std::error::Error for ImageError {}
