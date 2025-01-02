// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use crate::module_image::{ModuleSectionId, SectionEntry};

#[repr(C)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct IndexPropertySection {
    pub entry_function_public_index: u32, // u32::max = none
}

impl<'a> SectionEntry<'a> for IndexPropertySection {
    fn read(section_data: &'a [u8]) -> Self {
        let property_section_ptr = unsafe {
            std::mem::transmute::<*const u8, *const IndexPropertySection>(section_data.as_ptr())
        };

        unsafe { *property_section_ptr }
    }

    fn write(&'a self, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        let mut section_data = [0u8; std::mem::size_of::<IndexPropertySection>()];
        let src = self as *const IndexPropertySection as *const u8;
        let dst = section_data.as_mut_ptr();
        unsafe { std::ptr::copy(src, dst, section_data.len()) };

        writer.write_all(&section_data)
    }

    fn id(&'a self) -> ModuleSectionId {
        ModuleSectionId::IndexProperty
    }
}

#[cfg(test)]
mod tests {
    use crate::module_image::SectionEntry;

    use super::IndexPropertySection;

    #[test]
    fn test_write_section() {
        let section = IndexPropertySection {
            entry_function_public_index: 11,
        };

        let mut section_data: Vec<u8> = vec![];
        section.write(&mut section_data).unwrap();

        let mut expect_data = vec![
            11, 0, 0, 0, // entry function public index
        ];

        expect_data.resize(std::mem::size_of::<IndexPropertySection>(), 0);
        assert_eq!(section_data, expect_data);
    }

    #[test]
    fn test_read_section() {
        let mut section_data = vec![
            11, 0, 0, 0, // entry function public index
        ];

        section_data.resize(std::mem::size_of::<IndexPropertySection>(), 0);

        let section = IndexPropertySection::read(&section_data);
        assert_eq!(section.entry_function_public_index, 11);
    }
}
