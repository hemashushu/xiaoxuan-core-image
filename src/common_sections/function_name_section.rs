// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

// In the default VM implementation, this section lists all internal functions,
// including both public and private functions. While it is not mandatory,
// public functions should be listed to enable export and archive linking.
//
// Private function names are primarily used for debugging purposes, and are not
// essential for the VM to function correctly.

// Functions are accessed using the `function_public_index`, which is calculated as:
// `function_public_index = (number of all imported functions) + function_internal_index`
//
// The diagram below illustrates the relationship between the
// function internal index and the function public index:
//
//               /--------------------\ <--\
// number of     |                    |    |
// imported  --> | Imported functions |    |
// functions     |                    |    |
//               |--------------------|    |
//               |                    |    |
// function  --> | Internal functions |    | <-- function public index
// internal      |                    |    |
// index         |                    |    |
//               \--------------------/ <--/

// "Export Function Section" binary layout:
//
//              |-----------------------------------------------------|
//              | item count (u32) | extra header length (u32)        |
//              |-----------------------------------------------------|
//  item 0 -->  | full name offset 0 (u32) | full name length 0 (u32) |
//              | visibility 0 (u8) | pad 3 bytes                     | <-- table
//              | internal_index (u32)                                |
//              |                                                     |
//  item 1 -->  | full name offset 1       | full name length 1       |
//              | visibility 1      | pad 3 bytes |                   |
//              | internal_index (u32)                                |
//              |                                                     |
//              | ...                                                 |
//              |-----------------------------------------------------|
// offset 0 --> | full name string 0 (UTF-8)                          | <-- data
// offset 1 --> | full name string 1                                  |
//              | ...                                                 |
//              |-----------------------------------------------------|

use crate::{
    datatableaccess::{
        read_section_with_table_and_data_area, write_section_with_table_and_data_area,
    },
    entry::FunctionNameEntry,
    module_image::{ModuleSectionId, SectionEntry, Visibility},
};

#[derive(Debug, PartialEq, Default)]
pub struct FunctionNameSection<'a> {
    pub items: &'a [FunctionNameItem],
    pub full_names_data: &'a [u8],
}

// This table only contains internal functions.
// Imported functions are not listed in this table.
#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct FunctionNameItem {
    // Explanation of "full_name" and "name_path":
    // ------------------------------------------
    // - "full_name"  = "module_name::name_path"
    // - "name_path"  = "namespaces::identifier"
    // - "namespaces" = "sub_module_name"{0,N}
    //
    // For example, assuming there is an object named "config" in the submodule "myapp::settings":
    // - The full name is "myapp::settings::config".
    // - The module name is "myapp".
    // - The name path is "settings::config".
    pub full_name_offset: u32,
    pub full_name_length: u32,
    pub visibility: Visibility,
    _padding0: [u8; 3],

    /// The function index in the function section.
    pub internal_index: u32,
}

impl FunctionNameItem {
    pub fn new(
        full_name_offset: u32,
        full_name_length: u32,
        visibility: Visibility,
        internal_index: u32,
    ) -> Self {
        Self {
            full_name_offset,
            full_name_length,
            visibility,
            _padding0: [0, 0, 0],
            internal_index,
        }
    }
}

impl<'a> SectionEntry<'a> for FunctionNameSection<'a> {
    fn read(section_data: &'a [u8]) -> Self {
        let (items, full_names_data) =
            read_section_with_table_and_data_area::<FunctionNameItem>(section_data);
        FunctionNameSection {
            items,
            full_names_data,
        }
    }

    fn write(&'a self, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        write_section_with_table_and_data_area(self.items, self.full_names_data, writer)
    }

    fn id(&'a self) -> ModuleSectionId {
        ModuleSectionId::FunctionName
    }
}

impl<'a> FunctionNameSection<'a> {
    /// Retrieves `(visibility, function_internal_index)` by the full name.
    pub fn get_item_visibility_and_function_internal_index(
        &'a self,
        expected_full_name: &str,
    ) -> Option<(
        Visibility,
        usize, // function_internal_index
    )> {
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
            (item.visibility, item.internal_index as usize)
        })
    }

    /// Retrieves `(full_name, visibility)` by the function internal index.
    pub fn get_item_full_name_and_visibility(
        &self,
        function_internal_index: usize,
    ) -> Option<(&str, Visibility)> {
        let items = self.items;
        let full_names_data = self.full_names_data;

        let opt_idx = items
            .iter()
            .position(|item| item.internal_index as usize == function_internal_index);

        opt_idx.map(|idx| {
            let item = &items[idx];
            let full_name_data = &full_names_data[item.full_name_offset as usize
                ..(item.full_name_offset + item.full_name_length) as usize];
            let full_name = std::str::from_utf8(full_name_data).unwrap();
            (full_name, item.visibility)
        })
    }

    /// Converts the section into a vector of `ExportFunctionEntry`.
    pub fn convert_to_entries(&self) -> Vec<FunctionNameEntry> {
        let items = self.items;
        let full_names_data = self.full_names_data;

        items
            .iter()
            .map(|item| {
                let full_name_data = &full_names_data[item.full_name_offset as usize
                    ..(item.full_name_offset + item.full_name_length) as usize];

                let full_name = std::str::from_utf8(full_name_data).unwrap().to_owned();
                FunctionNameEntry::new(
                    full_name.to_owned(),
                    item.visibility,
                    item.internal_index as usize,
                )
            })
            .collect()
    }

    /// Converts a vector of `ExportFunctionEntry` into section data.
    pub fn convert_from_entries(entries: &[FunctionNameEntry]) -> (Vec<FunctionNameItem>, Vec<u8>) {
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

                FunctionNameItem::new(
                    full_name_offset,
                    full_name_length,
                    entry.visibility,
                    entry.internal_index as u32,
                )
            })
            .collect::<Vec<FunctionNameItem>>();

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
        common_sections::function_name_section::{FunctionNameItem, FunctionNameSection},
        entry::FunctionNameEntry,
        module_image::{SectionEntry, Visibility},
    };

    #[test]
    fn test_write_section() {
        let items: Vec<FunctionNameItem> = vec![
            FunctionNameItem::new(0, 3, Visibility::Private, 11),
            FunctionNameItem::new(3, 5, Visibility::Public, 13),
        ];

        let section = FunctionNameSection {
            items: &items,
            full_names_data: "foohello".as_bytes(),
        };

        let mut section_data: Vec<u8> = vec![];
        section.write(&mut section_data).unwrap();

        let mut expect_data = vec![
            2u8, 0, 0, 0, // item count
            0, 0, 0, 0, // extra section header len (i32)
            //
            0, 0, 0, 0, // name offset (item 0)
            3, 0, 0, 0, // name length
            0, // visibility
            0, 0, 0, // padding
            11, 0, 0, 0, // internal index
            //
            3, 0, 0, 0, // name offset (item 1)
            5, 0, 0, 0, // name length
            1, // visibility
            0, 0, 0, // padding
            13, 0, 0, 0, // internal index
        ];

        expect_data.extend_from_slice(b"foo");
        expect_data.extend_from_slice(b"hello");

        assert_eq!(section_data, expect_data);
    }

    #[test]
    fn test_read_section() {
        let mut section_data = vec![
            2u8, 0, 0, 0, // item count
            0, 0, 0, 0, // extra section header len (i32)
            //
            0, 0, 0, 0, // name offset (item 0)
            3, 0, 0, 0, // name length
            0, // visibility
            0, 0, 0, // padding
            11, 0, 0, 0, // internal index
            //
            3, 0, 0, 0, // name offset (item 1)
            5, 0, 0, 0, // name length
            1, // visibility
            0, 0, 0, // padding
            13, 0, 0, 0, // internal index
        ];

        section_data.extend_from_slice("foo".as_bytes());
        section_data.extend_from_slice("hello".as_bytes());

        let section = FunctionNameSection::read(&section_data);

        assert_eq!(section.items.len(), 2);
        assert_eq!(
            section.items[0],
            FunctionNameItem::new(0, 3, Visibility::Private, 11)
        );
        assert_eq!(
            section.items[1],
            FunctionNameItem::new(3, 5, Visibility::Public, 13)
        );
        assert_eq!(section.full_names_data, "foohello".as_bytes())
    }

    #[test]
    fn test_convert() {
        let entries: Vec<FunctionNameEntry> = vec![
            FunctionNameEntry::new("foo".to_string(), Visibility::Private, 11),
            FunctionNameEntry::new("hello".to_string(), Visibility::Public, 13),
        ];

        let (items, names_data) = FunctionNameSection::convert_from_entries(&entries);
        let section = FunctionNameSection {
            items: &items,
            full_names_data: &names_data,
        };

        assert_eq!(
            section.get_item_visibility_and_function_internal_index("foo"),
            Some((Visibility::Private, 11))
        );
        assert_eq!(
            section.get_item_visibility_and_function_internal_index("hello"),
            Some((Visibility::Public, 13))
        );
        assert_eq!(section.get_item_visibility_and_function_internal_index("bar"), None);

        assert_eq!(
            section.get_item_full_name_and_visibility(11),
            Some(("foo", Visibility::Private))
        );
        assert_eq!(
            section.get_item_full_name_and_visibility(13),
            Some(("hello", Visibility::Public))
        );

        let entries_restore = section.convert_to_entries();
        assert_eq!(entries, entries_restore);
    }
}
