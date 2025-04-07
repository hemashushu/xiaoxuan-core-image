// Copyright (c) 2025 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions.
// For more details, see the LICENSE, LICENSE.additional, and CONTRIBUTING files.

// "Unified External Function Section" binary layout:
//
//              |-----------------------------------------------------|
//              | item count (u32) | extra header length (u32)        |
//              |-----------------------------------------------------|
//  item 0 -->  | fn name offset 0 (u32) | fn name length 0 (u32)     |
//              | external library index 0 (u32) | type index 0 (u32) | <-- table
//  item 1 -->  | fn name offset 1       | fn name length 1           |
//              | external library index 1       | type index 1       |
//              | ...                                                 |
//              |-----------------------------------------------------|
// offset 0 --> | function name string 0 (UTF-8)                      | <-- data
// offset 1 --> | function name string 1                              |
//              | ...                                                 |
//              |-----------------------------------------------------|
//
// The binary layout of this section is identical to `ExternalFunctionSection`.

use crate::{
    datatableaccess::{
        read_section_with_table_and_data_area, write_section_with_table_and_data_area,
    },
    entry::ExternalFunctionEntry,
    module_image::{ModuleSectionId, SectionEntry},
};

#[derive(Debug, PartialEq, Default)]
pub struct UnifiedExternalFunctionSection<'a> {
    // Array of function items, each representing a function's metadata.
    pub items: &'a [ExternalFunctionItem],
    // Raw UTF-8 encoded data containing function names.
    pub names_data: &'a [u8],
}

#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct ExternalFunctionItem {
    // Offset of the function name string in the data area.
    pub name_offset: u32,
    // Length (in bytes) of the function name string in the data area.
    pub name_length: u32,
    // Index of the external library, referencing the "unified external library" section.
    pub external_library_index: u32,
    // Index of the function type, referencing the "unified function type" section.
    pub type_index: u32,
}

impl ExternalFunctionItem {
    pub fn new(
        name_offset: u32,
        name_length: u32,
        external_library_index: u32,
        type_index: u32,
    ) -> Self {
        Self {
            name_offset,
            name_length,
            external_library_index,
            type_index,
        }
    }
}

impl<'a> SectionEntry<'a> for UnifiedExternalFunctionSection<'a> {
    fn read(section_data: &'a [u8]) -> Self {
        // Reads the section data and splits it into a table of items and a data area.
        let (items, names_data) =
            read_section_with_table_and_data_area::<ExternalFunctionItem>(section_data);
        UnifiedExternalFunctionSection { items, names_data }
    }

    fn write(&'a self, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        // Writes the section data, including the table of items and the data area.
        write_section_with_table_and_data_area(self.items, self.names_data, writer)
    }

    fn id(&'a self) -> ModuleSectionId {
        // Returns the section ID for the unified external function section.
        ModuleSectionId::UnifiedExternalFunction
    }
}

impl<'a> UnifiedExternalFunctionSection<'a> {
    pub fn get_item_name_and_external_library_index_and_type_index(
        &'a self,
        idx: usize,
    ) -> (&'a str, usize, usize) {
        // Retrieves the function name, external library index, and type index for a given item.
        let items = self.items;
        let names_data = self.names_data;

        let item = &items[idx];
        let name_data =
            &names_data[item.name_offset as usize..(item.name_offset + item.name_length) as usize];

        (
            std::str::from_utf8(name_data).unwrap(),
            item.external_library_index as usize,
            item.type_index as usize,
        )
    }

    pub fn convert_from_entries(
        entries: &[ExternalFunctionEntry],
    ) -> (Vec<ExternalFunctionItem>, Vec<u8>) {
        // Converts a list of `ExternalFunctionEntry` into a table of items and a data area.
        let name_bytes = entries
            .iter()
            .map(|entry| entry.name.as_bytes())
            .collect::<Vec<&[u8]>>();

        let mut next_offset: u32 = 0;

        let items = entries
            .iter()
            .enumerate()
            .map(|(idx, entry)| {
                let name_offset = next_offset;
                let name_length = name_bytes[idx].len() as u32;
                next_offset += name_length; // Update offset for the next entry.

                ExternalFunctionItem::new(
                    name_offset,
                    name_length,
                    entry.external_library_index as u32,
                    entry.type_index as u32,
                )
            })
            .collect::<Vec<ExternalFunctionItem>>();

        let names_data = name_bytes
            .iter()
            .flat_map(|bytes| bytes.to_vec())
            .collect::<Vec<u8>>();

        (items, names_data)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        common_sections::external_function_section::{
            ExternalFunctionItem, ExternalFunctionSection,
        },
        entry::ExternalFunctionEntry,
        module_image::SectionEntry,
    };

    #[test]
    fn test_read_section() {
        // Tests reading a section from binary data.
        let mut section_data = vec![
            2u8, 0, 0, 0, // item count
            0, 0, 0, 0, // extra section header length (u32)
            //
            0, 0, 0, 0, // name offset (item 0)
            3, 0, 0, 0, // name length
            11, 0, 0, 0, // external library index
            13, 0, 0, 0, // type index
            //
            3, 0, 0, 0, // name offset (item 1)
            5, 0, 0, 0, // name length
            15, 0, 0, 0, // external library index
            17, 0, 0, 0, // type index
        ];

        section_data.extend_from_slice(b"foo");
        section_data.extend_from_slice(b"hello");

        let section = ExternalFunctionSection::read(&section_data);

        assert_eq!(section.items.len(), 2);
        assert_eq!(section.items[0], ExternalFunctionItem::new(0, 3, 11, 13,));
        assert_eq!(section.items[1], ExternalFunctionItem::new(3, 5, 15, 17));
        assert_eq!(section.names_data, "foohello".as_bytes())
    }

    #[test]
    fn test_write_section() {
        // Tests writing a section to binary data.
        let items = vec![
            ExternalFunctionItem::new(0, 3, 11, 13),
            ExternalFunctionItem::new(3, 5, 15, 17),
        ];

        let section = ExternalFunctionSection {
            items: &items,
            names_data: b"foohello",
        };

        let mut section_data: Vec<u8> = vec![];
        section.write(&mut section_data).unwrap();

        let mut expect_data = vec![
            2u8, 0, 0, 0, // item count
            0, 0, 0, 0, // extra section header length (u32)
            //
            0, 0, 0, 0, // name offset (item 0)
            3, 0, 0, 0, // name length
            11, 0, 0, 0, // external library index
            13, 0, 0, 0, // type index
            //
            3, 0, 0, 0, // name offset (item 1)
            5, 0, 0, 0, // name length
            15, 0, 0, 0, // external library index
            17, 0, 0, 0, // type index
        ];

        expect_data.extend_from_slice(b"foo");
        expect_data.extend_from_slice(b"hello");

        assert_eq!(section_data, expect_data);
    }

    #[test]
    fn test_convert() {
        // Tests converting entries into a section.
        let entries = vec![
            ExternalFunctionEntry::new("foobar".to_string(), 17, 19),
            ExternalFunctionEntry::new("helloworld".to_string(), 23, 29),
        ];

        let (items, names_data) = ExternalFunctionSection::convert_from_entries(&entries);
        let section = ExternalFunctionSection {
            items: &items,
            names_data: &names_data,
        };

        assert_eq!(
            section.get_item_name_and_external_library_index_and_type_index(0),
            ("foobar", 17, 19)
        );

        assert_eq!(
            section.get_item_name_and_external_library_index_and_type_index(1),
            ("helloworld", 23, 29)
        );
    }
}
