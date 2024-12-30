// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

//! this section list all internal functions.

// "function name section" binary layout
//
//              |---------------------------------------------------------------------------------------|
//              | item count (u32) | (4 bytes padding)                                                  |
//              |---------------------------------------------------------------------------------------|
//  item 0 -->  | full name offset 0 (u32) | full name length 0 (u32) | visibility 0 (u8) | pad 3 bytes | <-- table
//  item 1 -->  | full name offset 1       | full name length 1       | visibility 1      | pad 3 bytes |
//              | ...                                                                                   |
//              |---------------------------------------------------------------------------------------|
// offset 0 --> | full name string 0 (UTF-8)                                                            | <-- data area
// offset 1 --> | full name string 1                                                                    |
//              | ...                                                                                   |
//              |---------------------------------------------------------------------------------------|

use crate::{
    entry::ExportFunctionEntry,
    module_image::{ModuleSectionId, SectionEntry, Visibility},
    tableaccess::{read_section_with_table_and_data_area, write_section_with_table_and_data_area},
};

#[derive(Debug, PartialEq, Default)]
pub struct ExportFunctionSection<'a> {
    pub items: &'a [ExportFunctionItem],
    pub full_names_data: &'a [u8],
}

// this table only contains the internal functions,
// imported functions will not be list in this table.
#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct ExportFunctionItem {
    // about the "full_name" and "name_path"
    // -------------------------------------
    // - "full_name" = "module_name::name_path"
    // - "name_path" = "namespace::identifier"
    // - "namespace" = "sub_module_name"{0,N}
    //
    // e.g.
    // the name path of function "add" in submodule "myapp:utils" is "utils::add",
    // and the full name is "myapp::utils::add"
    pub full_name_offset: u32,
    pub full_name_length: u32,

    pub visibility: Visibility,
    _padding0: [u8; 3],
}

impl ExportFunctionItem {
    pub fn new(full_name_offset: u32, full_name_length: u32, visibility: Visibility) -> Self {
        Self {
            full_name_offset,
            full_name_length,
            visibility,
            _padding0: [0, 0, 0],
        }
    }
}

impl<'a> SectionEntry<'a> for ExportFunctionSection<'a> {
    fn read(section_data: &'a [u8]) -> Self {
        let (items, full_names_data) =
            read_section_with_table_and_data_area::<ExportFunctionItem>(section_data);
        ExportFunctionSection {
            items,
            full_names_data,
        }
    }

    fn write(&'a self, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        write_section_with_table_and_data_area(self.items, self.full_names_data, writer)
    }

    fn id(&'a self) -> ModuleSectionId {
        ModuleSectionId::ExportFunction
    }
}

impl<'a> ExportFunctionSection<'a> {
    /// the item index is the `function internal index`
    ///
    /// the function public index is mixed by the following items:
    /// - the imported functions
    /// - the internal functions
    ///
    /// therefore:
    /// function_public_index = (all import functions) + function_internal_index
    pub fn get_item_index_and_visibility(
        &'a self,
        expected_full_name: &str,
    ) -> Option<(usize, Visibility)> {
        let items = self.items;
        let full_names_data = self.full_names_data;

        let expected_full_name_data = expected_full_name.as_bytes();

        let opt_idx = items.iter().position(|item| {
            let full_name_data = &full_names_data[item.full_name_offset as usize
                ..(item.full_name_offset + item.full_name_length) as usize];
            full_name_data == expected_full_name_data
        });

        opt_idx.map(|idx| {
            let item = &items[idx];
            (idx, item.visibility)
        })
    }

    pub fn get_item_full_name_and_visibility(
        &self,
        function_internal_index: usize,
    ) -> (&str, Visibility) {
        let items = self.items;
        let full_names_data = self.full_names_data;

        let item = &items[function_internal_index];
        let full_name_data = &full_names_data[item.full_name_offset as usize
            ..(item.full_name_offset + item.full_name_length) as usize];
        let full_name = std::str::from_utf8(full_name_data).unwrap();
        (full_name, item.visibility)
    }

    pub fn convert_to_entries(&self) -> Vec<ExportFunctionEntry> {
        let items = self.items;
        let full_names_data = self.full_names_data;

        items
            .iter()
            .map(|item| {
                let full_name_data = &full_names_data[item.full_name_offset as usize
                    ..(item.full_name_offset + item.full_name_length) as usize];

                let full_name = std::str::from_utf8(full_name_data).unwrap().to_owned();
                ExportFunctionEntry::new(full_name.to_owned(), item.visibility)
            })
            .collect()
    }

    pub fn convert_from_entries(
        entries: &[ExportFunctionEntry],
    ) -> (Vec<ExportFunctionItem>, Vec<u8>) {
        let full_name_bytes = entries
            .iter()
            .map(|entry| entry.full_name.as_bytes())
            .collect::<Vec<&[u8]>>();

        let mut next_offset: u32 = 0;

        let items = entries
            .iter()
            .enumerate()
            .map(|(idx, entry)| {
                let full_name_offset = next_offset;
                let full_name_length = full_name_bytes[idx].len() as u32;
                next_offset += full_name_length; // for next offset

                ExportFunctionItem::new(full_name_offset, full_name_length, entry.visibility)
            })
            .collect::<Vec<ExportFunctionItem>>();

        let full_names_data = full_name_bytes
            .iter()
            .flat_map(|bytes| bytes.to_vec())
            .collect::<Vec<u8>>();

        (items, full_names_data)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        common_sections::export_function_section::{ExportFunctionItem, ExportFunctionSection},
        entry::ExportFunctionEntry,
        module_image::{SectionEntry, Visibility},
    };

    #[test]
    fn test_write_section() {
        let items: Vec<ExportFunctionItem> = vec![
            ExportFunctionItem::new(0, 3, Visibility::Private),
            ExportFunctionItem::new(3, 5, Visibility::Public),
        ];

        let section = ExportFunctionSection {
            items: &items,
            full_names_data: "foohello".as_bytes(),
        };

        let mut section_data: Vec<u8> = vec![];
        section.write(&mut section_data).unwrap();

        let mut expect_data = vec![
            2u8, 0, 0, 0, // item count
            0, 0, 0, 0, // 4 bytes padding
            //
            0, 0, 0, 0, // name offset (item 0)
            3, 0, 0, 0, // name length
            0, // visibility
            0, 0, 0, // padding
            //
            3, 0, 0, 0, // name offset (item 1)
            5, 0, 0, 0, // name length
            1, // visibility
            0, 0, 0, // padding
        ];

        expect_data.extend_from_slice(b"foo");
        expect_data.extend_from_slice(b"hello");

        assert_eq!(section_data, expect_data);
    }

    #[test]
    fn test_read_section() {
        let mut section_data = vec![
            2u8, 0, 0, 0, // item count
            0, 0, 0, 0, // 4 bytes padding
            //
            0, 0, 0, 0, // name offset (item 0)
            3, 0, 0, 0, // name length
            0, // visibility
            0, 0, 0, // padding
            //
            3, 0, 0, 0, // name offset (item 1)
            5, 0, 0, 0, // name length
            1, // visibility
            0, 0, 0, // padding
        ];

        section_data.extend_from_slice("foo".as_bytes());
        section_data.extend_from_slice("hello".as_bytes());

        let section = ExportFunctionSection::read(&section_data);

        assert_eq!(section.items.len(), 2);
        assert_eq!(
            section.items[0],
            ExportFunctionItem::new(0, 3, /*11,*/ Visibility::Private)
        );
        assert_eq!(
            section.items[1],
            ExportFunctionItem::new(3, 5, /*13,*/ Visibility::Public)
        );
        assert_eq!(section.full_names_data, "foohello".as_bytes())
    }

    #[test]
    fn test_convert() {
        let entries: Vec<ExportFunctionEntry> = vec![
            ExportFunctionEntry::new("foo".to_string(), /*11,*/ Visibility::Private),
            ExportFunctionEntry::new("hello".to_string(), /*13,*/ Visibility::Public),
        ];

        let (items, names_data) = ExportFunctionSection::convert_from_entries(&entries);
        let section = ExportFunctionSection {
            items: &items,
            full_names_data: &names_data,
        };

        assert_eq!(
            section.get_item_index_and_visibility("foo"),
            Some((0, Visibility::Private))
        );
        assert_eq!(
            section.get_item_index_and_visibility("hello"),
            Some((1, Visibility::Public))
        );
        assert_eq!(section.get_item_index_and_visibility("bar"), None);

        assert_eq!(
            section.get_item_full_name_and_visibility(0),
            ("foo", Visibility::Private)
        );
        assert_eq!(
            section.get_item_full_name_and_visibility(1),
            ("hello", Visibility::Public)
        );

        let entries_restore = section.convert_to_entries();
        assert_eq!(entries, entries_restore);
    }
}
