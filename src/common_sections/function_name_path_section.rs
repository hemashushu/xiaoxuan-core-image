// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

//! this section list only the internal functions.

// "function name section" binary layout
//
//              |----------------------------------------------------------------------------------------------------------|
//              | item count (u32) | (4 bytes padding)                                                                     |
//              |----------------------------------------------------------------------------------------------------------|
//  item 0 -->  | name path offset 0 (u32) | name path length 0 (u32) | fn_pub_index 0 (u32) | export 0 (u8) | pad 3 bytes | <-- table
//  item 1 -->  | name path offset 1       | name path length 1       | fn_pub_index 1       | export 1      | pad 3 bytes |
//              | ...                                                                                                      |
//              |----------------------------------------------------------------------------------------------------------|
// offset 0 --> | name path string 0 (UTF-8)                                                                               | <-- data area
// offset 1 --> | name path string 1                                                                                       |
//              | ...                                                                                                      |
//              |----------------------------------------------------------------------------------------------------------|

use crate::{
    entry::FunctionNamePathEntry,
    module_image::{ModuleSectionId, SectionEntry},
    tableaccess::{load_section_with_table_and_data_area, save_section_with_table_and_data_area},
};

#[derive(Debug, PartialEq, Default)]
pub struct FunctionNamePathSection<'a> {
    pub items: &'a [FunctionNamePathItem],
    pub name_paths_data: &'a [u8],
}

// this table only contains the internal functions,
// imported functions will not be list in this table.
#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct FunctionNamePathItem {
    // about the "full_name" and "name_path"
    // -------------------------------------
    // - "full_name" = "module_name::name_path"
    // - "name_path" = "namespace::identifier"
    // - "namespace" = "sub_module_name"{0,N}
    pub name_path_offset: u32,
    pub name_path_length: u32,

    // Used to indicate the visibility of this item when this
    // module is used as a shared module.
    // Note that in the case of static linking, the item is always
    // visible to other modules, regardless of the value of this property.
    //
    // 0=false, 1=true
    pub export: u8,
    _padding0: [u8; 3],
}

impl FunctionNamePathItem {
    pub fn new(name_path_offset: u32, name_path_length: u32, export: u8) -> Self {
        Self {
            name_path_offset,
            name_path_length,
            export,
            _padding0: [0, 0, 0],
        }
    }
}

impl<'a> SectionEntry<'a> for FunctionNamePathSection<'a> {
    fn load(section_data: &'a [u8]) -> Self {
        let (items, name_paths_data) =
            load_section_with_table_and_data_area::<FunctionNamePathItem>(section_data);
        FunctionNamePathSection {
            items,
            name_paths_data,
        }
    }

    fn save(&'a self, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        save_section_with_table_and_data_area(self.items, self.name_paths_data, writer)
    }

    fn id(&'a self) -> ModuleSectionId {
        ModuleSectionId::FunctionNamePath
    }
}

impl<'a> FunctionNamePathSection<'a> {
    /// the item index is the `function internal index`
    ///
    /// the function public index is mixed by the following items:
    /// - the imported functions
    /// - the internal functions
    ///
    /// therefore:
    /// function_public_index = (all import functions) + function_internal_index
    pub fn get_item_index_and_export(&'a self, expected_name_path: &str) -> Option<(usize, bool)> {
        let items = self.items;
        let name_paths_data = self.name_paths_data;

        let expected_name_path_data = expected_name_path.as_bytes();

        let opt_idx = items.iter().position(|item| {
            let name_path_data = &name_paths_data[item.name_path_offset as usize
                ..(item.name_path_offset + item.name_path_length) as usize];
            name_path_data == expected_name_path_data
        });

        opt_idx.map(|idx| {
            let item = &items[idx];
            (idx, item.export != 0)
        })
    }

    pub fn get_item_name_and_export(&self, function_internal_index: usize) -> (&str, bool) {
        let items = self.items;
        let name_paths_data = self.name_paths_data;

        let item = &items[function_internal_index];
        let name_path_data = &name_paths_data[item.name_path_offset as usize
            ..(item.name_path_offset + item.name_path_length) as usize];
        let name = unsafe { std::str::from_utf8_unchecked(name_path_data) };
        (name, item.export != 0)
    }

    pub fn convert_from_entries(
        entries: &[FunctionNamePathEntry],
    ) -> (Vec<FunctionNamePathItem>, Vec<u8>) {
        let name_path_bytes = entries
            .iter()
            .map(|entry| entry.name_path.as_bytes())
            .collect::<Vec<&[u8]>>();

        let mut next_offset: u32 = 0;

        let items = entries
            .iter()
            .enumerate()
            .map(|(idx, entry)| {
                let name_path_offset = next_offset;
                let name_path_length = name_path_bytes[idx].len() as u32;
                next_offset += name_path_length; // for next offset

                FunctionNamePathItem::new(
                    name_path_offset,
                    name_path_length,
                    if entry.export { 1 } else { 0 },
                )
            })
            .collect::<Vec<FunctionNamePathItem>>();

        let name_paths_data = name_path_bytes
            .iter()
            .flat_map(|bytes| bytes.to_vec())
            .collect::<Vec<u8>>();

        (items, name_paths_data)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        common_sections::function_name_path_section::{
            FunctionNamePathItem, FunctionNamePathSection,
        },
        entry::FunctionNamePathEntry,
        module_image::SectionEntry,
    };

    #[test]
    fn test_save_section() {
        let items: Vec<FunctionNamePathItem> = vec![
            FunctionNamePathItem::new(0, 3, 0),
            FunctionNamePathItem::new(3, 5, 1),
        ];

        let section = FunctionNamePathSection {
            items: &items,
            name_paths_data: "foohello".as_bytes(),
        };

        let mut section_data: Vec<u8> = Vec::new();
        section.save(&mut section_data).unwrap();

        let mut expect_data = vec![
            2u8, 0, 0, 0, // item count
            0, 0, 0, 0, // 4 bytes padding
            //
            0, 0, 0, 0, // name offset (item 0)
            3, 0, 0, 0, // name length
            0, // export
            0, 0, 0, // padding
            //
            3, 0, 0, 0, // name offset (item 1)
            5, 0, 0, 0, // name length
            1, // export
            0, 0, 0, // padding
        ];

        expect_data.extend_from_slice(b"foo");
        expect_data.extend_from_slice(b"hello");

        assert_eq!(section_data, expect_data);
    }

    #[test]
    fn test_load_section() {
        let mut section_data = vec![
            2u8, 0, 0, 0, // item count
            0, 0, 0, 0, // 4 bytes padding
            //
            0, 0, 0, 0, // name offset (item 0)
            3, 0, 0, 0, // name length
            0, // export
            0, 0, 0, // padding
            //
            3, 0, 0, 0, // name offset (item 1)
            5, 0, 0, 0, // name length
            1, // export
            0, 0, 0, // padding
        ];

        section_data.extend_from_slice("foo".as_bytes());
        section_data.extend_from_slice("hello".as_bytes());

        let section = FunctionNamePathSection::load(&section_data);

        assert_eq!(section.items.len(), 2);
        assert_eq!(section.items[0], FunctionNamePathItem::new(0, 3, /*11,*/ 0));
        assert_eq!(section.items[1], FunctionNamePathItem::new(3, 5, /*13,*/ 1));
        assert_eq!(section.name_paths_data, "foohello".as_bytes())
    }

    #[test]
    fn test_convert() {
        let entries: Vec<FunctionNamePathEntry> = vec![
            FunctionNamePathEntry::new("foo".to_string(), /*11,*/ false),
            FunctionNamePathEntry::new("hello".to_string(), /*13,*/ true),
        ];

        let (items, names_data) = FunctionNamePathSection::convert_from_entries(&entries);
        let section = FunctionNamePathSection {
            items: &items,
            name_paths_data: &names_data,
        };

        assert_eq!(section.get_item_index_and_export("foo"), Some((0, false)));
        assert_eq!(section.get_item_index_and_export("hello"), Some((1, true)));
        assert_eq!(section.get_item_index_and_export("bar"), None);

        assert_eq!(section.get_item_name_and_export(0), ("foo", false));
        assert_eq!(section.get_item_name_and_export(1), ("hello", true));
    }
}
