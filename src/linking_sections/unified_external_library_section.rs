// Copyright (c) 2025 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions.
// For more details, see the LICENSE, LICENSE.additional, and CONTRIBUTING files.

// "Unified External Library Section" binary layout:
//
//              |-----------------------------------------------------------|
//              | item count (u32) | extra header length (u32)              |
//              |-----------------------------------------------------------|
//  item 0 -->  | library name offset 0 (u32) | library name length 0 (u32) | <-- table
//              | value offset 0 (u32) | value length 0 (u32)               |
//              | library type 0 (u8) | padding (3 bytes)                   |
//  item 1 -->  | library name offset 1       | library name length 1       |
//              | value offset 1 (u32) | value length 1 (u32)               |
//              | library type 1 (u8) | padding (3 bytes)                   |
//              | ...                                                       |
//              |-----------------------------------------------------------|
// offset 0 --> | library name string 0 (UTF-8) | value string 0 (UTF-8)    | <-- data
// offset 1 --> | library name string 1         | value string 1            |
//              | ...                                                       |
//              |-----------------------------------------------------------|
//
// The binary layout of this section is identical to `ExternalLibrarySection`.

use anc_isa::{ExternalLibraryDependency, ExternalLibraryDependencyType};

use crate::{
    datatableaccess::{
        read_section_with_table_and_data_area, write_section_with_table_and_data_area,
    },
    entry::ExternalLibraryEntry,
    module_image::{ModuleSectionId, SectionEntry},
};

#[derive(Debug, PartialEq, Default)]
pub struct UnifiedExternalLibrarySection<'a> {
    pub items: &'a [ExternalLibraryItem], // Array of library items (metadata).
    pub items_data: &'a [u8],             // Raw data area containing library names and values.
}

#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct ExternalLibraryItem {
    pub name_offset: u32,  // Offset of the library name string in the data area.
    pub name_length: u32,  // Length (in bytes) of the library name string.
    pub value_offset: u32, // Offset of the value string in the data area.
    pub value_length: u32, // Length (in bytes) of the value string.
    pub external_library_dependent_type: ExternalLibraryDependencyType, // Type of dependency (e.g., Local, Remote).
    _padding0: [u8; 3],                                                 // Padding for alignment.
}

impl ExternalLibraryItem {
    pub fn new(
        name_offset: u32,
        name_length: u32,
        value_offset: u32,
        value_length: u32,
        external_library_dependent_type: ExternalLibraryDependencyType,
    ) -> Self {
        Self {
            name_offset,
            name_length,
            value_offset,
            value_length,
            external_library_dependent_type,
            _padding0: [0; 3],
        }
    }
}

impl<'a> SectionEntry<'a> for UnifiedExternalLibrarySection<'a> {
    fn read(section_data: &'a [u8]) -> Self {
        // Reads the section data and splits it into items (metadata) and the data area.
        let (items, items_data) =
            read_section_with_table_and_data_area::<ExternalLibraryItem>(section_data);
        UnifiedExternalLibrarySection { items, items_data }
    }

    fn write(&'a self, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        // Writes the section data, including the table and data area, to the writer.
        write_section_with_table_and_data_area(self.items, self.items_data, writer)
    }

    fn id(&'a self) -> ModuleSectionId {
        // Returns the section ID for UnifiedExternalLibrary.
        ModuleSectionId::UnifiedExternalLibrary
    }
}

impl<'a> UnifiedExternalLibrarySection<'a> {
    pub fn get_item_name_and_external_library_dependent_type_and_value(
        &'a self,
        idx: usize,
    ) -> (&'a str, ExternalLibraryDependencyType, &'a [u8]) {
        let items = self.items;
        let items_data = self.items_data;

        let item = &items[idx];
        let name_data =
            &items_data[item.name_offset as usize..(item.name_offset + item.name_length) as usize];
        let value_data = &items_data
            [item.value_offset as usize..(item.value_offset + item.value_length) as usize];

        (
            std::str::from_utf8(name_data).unwrap(),
            item.external_library_dependent_type,
            value_data,
        )
    }

    pub fn convert_from_entries(
        entries: &[ExternalLibraryEntry],
    ) -> (Vec<ExternalLibraryItem>, Vec<u8>) {
        // Converts a list of `ExternalLibraryEntry` into a table of items and a data area.
        let mut name_bytes = entries
            .iter()
            .map(|entry| entry.name.as_bytes().to_vec())
            .collect::<Vec<Vec<u8>>>();

        let mut value_bytes = entries
            .iter()
            .map(|entry| {
                let value = entry.value.as_ref();
                let value_string = ason::to_string(value).unwrap();
                value_string.as_bytes().to_vec()
            })
            .collect::<Vec<Vec<u8>>>();

        let mut next_offset: u32 = 0;

        let items = entries
            .iter()
            .enumerate()
            .map(|(idx, entry)| {
                let name_length = name_bytes[idx].len() as u32;
                let value_length = value_bytes[idx].len() as u32;
                let name_offset = next_offset;
                let value_offset = name_offset + name_length;
                next_offset = value_offset + value_length; // Update offset for the next item.

                let external_library_dependent_type = match entry.value.as_ref() {
                    ExternalLibraryDependency::Local(_) => ExternalLibraryDependencyType::Local,
                    ExternalLibraryDependency::Remote(_) => ExternalLibraryDependencyType::Remote,
                    ExternalLibraryDependency::Share(_) => ExternalLibraryDependencyType::Share,
                    ExternalLibraryDependency::Runtime => ExternalLibraryDependencyType::Runtime,
                };

                ExternalLibraryItem::new(
                    name_offset,
                    name_length,
                    value_offset,
                    value_length,
                    external_library_dependent_type,
                )
            })
            .collect::<Vec<ExternalLibraryItem>>();

        let items_data = name_bytes
            .iter_mut()
            .zip(value_bytes.iter_mut())
            .flat_map(|(name_bytes, value_bytes)| {
                name_bytes.append(value_bytes);
                name_bytes.to_owned()
            })
            .collect::<Vec<u8>>();

        (items, items_data)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use anc_isa::{
        DependencyCondition, DependencyLocal, DependencyRemote, ExternalLibraryDependency,
        ExternalLibraryDependencyType,
    };

    use crate::{
        common_sections::external_library_section::{ExternalLibraryItem, ExternalLibrarySection},
        entry::ExternalLibraryEntry,
        module_image::SectionEntry,
    };

    #[test]
    fn test_read_section() {
        // Test reading a section and verifying its contents.
        let mut section_data = vec![
            2u8, 0, 0, 0, // item count
            0, 0, 0, 0, // extra section header length (u32)
            //
            0, 0, 0, 0, // name offset (item 0)
            3, 0, 0, 0, // name length
            3, 0, 0, 0, // value offset
            5, 0, 0, 0, // value length
            0, // library dependent type
            0, 0, 0, // padding
            //
            8, 0, 0, 0, // name offset (item 1)
            4, 0, 0, 0, // name length
            12, 0, 0, 0, // value offset
            6, 0, 0, 0, // value length
            1, // library dependent type
            0, 0, 0, // padding
        ];

        section_data.extend_from_slice(b"foo");
        section_data.extend_from_slice(b"hello");
        section_data.extend_from_slice(b".bar");
        section_data.extend_from_slice(b".world");

        let section = ExternalLibrarySection::read(&section_data);

        assert_eq!(section.items.len(), 2);
        assert_eq!(
            section.items[0],
            ExternalLibraryItem::new(0, 3, 3, 5, ExternalLibraryDependencyType::Local)
        );
        assert_eq!(
            section.items[1],
            ExternalLibraryItem::new(8, 4, 12, 6, ExternalLibraryDependencyType::Remote)
        );
        assert_eq!(section.items_data, "foohello.bar.world".as_bytes())
    }

    #[test]
    fn test_write_section() {
        // Test writing a section and verifying the output data.
        let items = vec![
            ExternalLibraryItem::new(0, 3, 3, 5, ExternalLibraryDependencyType::Local),
            ExternalLibraryItem::new(8, 4, 12, 6, ExternalLibraryDependencyType::Remote),
        ];

        let section = ExternalLibrarySection {
            items: &items,
            items_data: b"foohello.bar.world",
        };

        let mut section_data: Vec<u8> = vec![];
        section.write(&mut section_data).unwrap();

        let mut expect_data = vec![
            2u8, 0, 0, 0, // item count
            0, 0, 0, 0, // extra section header length (u32)
            //
            0, 0, 0, 0, // name offset (item 0)
            3, 0, 0, 0, // name length
            3, 0, 0, 0, // value offset
            5, 0, 0, 0, // value length
            0, // library dependent type
            0, 0, 0, // padding
            //
            8, 0, 0, 0, // name offset (item 1)
            4, 0, 0, 0, // name length
            12, 0, 0, 0, // value offset
            6, 0, 0, 0, // value length
            1, // library dependent type
            0, 0, 0, // padding
        ];

        expect_data.extend_from_slice(b"foo");
        expect_data.extend_from_slice(b"hello");
        expect_data.extend_from_slice(b".bar");
        expect_data.extend_from_slice(b".world");

        // Append padding for 4-byte alignment.
        expect_data.extend_from_slice(&[0, 0]);

        assert_eq!(section_data, expect_data);
    }

    #[test]
    fn test_convert() {
        // Test converting entries into a section and verifying the result.
        let entries = vec![
            ExternalLibraryEntry::new(
                "foobar".to_owned(),
                Box::new(ExternalLibraryDependency::Local(Box::new(
                    DependencyLocal {
                        path: "libhello.so.1".to_owned(),
                        condition: DependencyCondition::True,
                        parameters: HashMap::default(),
                    },
                ))),
            ),
            ExternalLibraryEntry::new(
                "helloworld".to_owned(),
                Box::new(ExternalLibraryDependency::Remote(Box::new(
                    DependencyRemote {
                        url: "http://a.b/c".to_owned(),
                        dir: Some("/modules/helloworld".to_owned()),
                        reversion: "v1.0.1".to_owned(),
                        condition: DependencyCondition::True,
                        parameters: HashMap::default(),
                    },
                ))),
            ),
        ];

        let (items, items_data) = ExternalLibrarySection::convert_from_entries(&entries);
        let section = ExternalLibrarySection {
            items: &items,
            items_data: &items_data,
        };

        let (name0, type0, value0) =
            section.get_item_name_and_external_library_dependent_type_and_value(0);
        let (name1, type1, value1) =
            section.get_item_name_and_external_library_dependent_type_and_value(1);

        assert_eq!(
            (name0, type0),
            ("foobar", ExternalLibraryDependencyType::Local)
        );
        assert_eq!(
            (name1, type1),
            ("helloworld", ExternalLibraryDependencyType::Remote)
        );

        let v0: ExternalLibraryDependency = ason::from_reader(value0).unwrap();
        assert_eq!(&v0, entries[0].value.as_ref());

        let v1: ExternalLibraryDependency = ason::from_reader(value1).unwrap();
        assert_eq!(&v1, entries[1].value.as_ref());
    }
}
