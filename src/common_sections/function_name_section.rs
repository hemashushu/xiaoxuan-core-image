// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

//! this section list only the internal functions.

// "function name section" binary layout
//
//              |------------------------------------------------------------------------------------------------|
//              | item count (u32) | (4 bytes padding)                                                           |
//              |------------------------------------------------------------------------------------------------|
//  item 0 -->  | name offset 0 (u32) | name length 0 (u32) | fn_pub_index 0 (u32) | export 0 (u8) | pad 3 bytes | <-- table
//  item 1 -->  | name offset 1       | name length 1       | fn_pub_index 1       | export 1      | pad 3 bytes |
//              | ...                                                                                            |
//              |------------------------------------------------------------------------------------------------|
// offset 0 --> | name string 0 (UTF-8)                                                                          | <-- data area
// offset 1 --> | name string 1                                                                                  |
//              | ...                                                                                            |
//              |------------------------------------------------------------------------------------------------|

use crate::{
    entry::FunctionNameEntry,
    module_image::{ModuleSectionId, SectionEntry},
    tableaccess::{load_section_with_table_and_data_area, save_section_with_table_and_data_area},
};

#[derive(Debug, PartialEq, Default)]
pub struct FunctionNameSection<'a> {
    pub items: &'a [FunctionNameItem],
    pub names_data: &'a [u8],
}

// this table only contains the internal functions,
// imported functions will not be list in this table.
#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct FunctionNameItem {
    /*
     the value of 'name' may be a name path, e.g.
     "namespace::identifier"
     note that name path is a path relative to the module,
     it does not include the name of module.
    */
    pub name_offset: u32,
    pub name_length: u32,

    // // pub function_public_index: u32, // this field is used for bridge function call

    // Used to indicate the visibility of this item when this
    // module is used as a shared module.
    // Note that in the case of static linking, the item is always
    // visible to other modules, regardless of the value of this property.
    //
    // 0=false, 1=true
    pub export: u8,
    _padding0: [u8; 3],
}

impl FunctionNameItem {
    pub fn new(
        name_offset: u32,
        name_length: u32,
        /*function_public_index: u32,*/ export: u8,
    ) -> Self {
        Self {
            name_offset,
            name_length,
            // function_public_index,
            export,
            _padding0: [0, 0, 0],
        }
    }
}

impl<'a> SectionEntry<'a> for FunctionNameSection<'a> {
    fn load(section_data: &'a [u8]) -> Self {
        let (items, names_data) =
            load_section_with_table_and_data_area::<FunctionNameItem>(section_data);
        FunctionNameSection { items, names_data }
    }

    fn save(&'a self, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        save_section_with_table_and_data_area(self.items, self.names_data, writer)
    }

    fn id(&'a self) -> ModuleSectionId {
        ModuleSectionId::FunctionName
    }
}

impl<'a> FunctionNameSection<'a> {
    /// the item index is the `function internal index`
    ///
    /// the function public index is mixed by the following items:
    /// - the imported functions
    /// - the internal functions
    ///
    /// therefore:
    /// function_public_index = (all import functions) + function_internal_index
    pub fn get_item_index_and_export(&'a self, expected_name: &str) -> Option<(usize, bool)> {
        let items = self.items;
        let names_data = self.names_data;

        let expected_name_data = expected_name.as_bytes();

        let opt_idx = items.iter().position(|item| {
            let name_data = &names_data
                [item.name_offset as usize..(item.name_offset + item.name_length) as usize];
            name_data == expected_name_data
        });

        opt_idx.map(|idx| {
            let item = &items[idx];
            (
                idx,
                /* item.function_public_index as usize,*/ item.export != 0,
            )
        })
    }

    pub fn convert_from_entries(entries: &[FunctionNameEntry]) -> (Vec<FunctionNameItem>, Vec<u8>) {
        let name_bytes = entries
            .iter()
            .map(|entry| entry.name_path.as_bytes())
            .collect::<Vec<&[u8]>>();

        let mut next_offset: u32 = 0;

        let items = entries
            .iter()
            .enumerate()
            .map(|(idx, entry)| {
                let name_offset = next_offset;
                let name_length = name_bytes[idx].len() as u32;
                next_offset += name_length; // for next offset

                FunctionNameItem::new(
                    name_offset,
                    name_length,
                    // entry.function_public_index as u32,
                    if entry.export { 1 } else { 0 },
                )
            })
            .collect::<Vec<FunctionNameItem>>();

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
        common_sections::function_name_section::{FunctionNameItem, FunctionNameSection},
        entry::FunctionNameEntry,
        module_image::SectionEntry,
    };

    #[test]
    fn test_save_section() {
        let items: Vec<FunctionNameItem> = vec![
            FunctionNameItem::new(0, 3, /* 11,*/ 0),
            FunctionNameItem::new(3, 5, /* 13,*/ 1),
        ];

        let section = FunctionNameSection {
            items: &items,
            names_data: "foohello".as_bytes(),
        };

        let mut section_data: Vec<u8> = Vec::new();
        section.save(&mut section_data).unwrap();

        let mut expect_data = vec![
            2u8, 0, 0, 0, // item count
            0, 0, 0, 0, // 4 bytes padding
            //
            0, 0, 0, 0, // name offset (item 0)
            3, 0, 0, 0, // name length
            // 11, 0, 0, 0, // function pub index
            0, // export
            0, 0, 0, // padding
            //
            3, 0, 0, 0, // name offset (item 1)
            5, 0, 0, 0, // name length
            // 13, 0, 0, 0, // function pub index
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
            // 11, 0, 0, 0, // function pub index
            0, // export
            0, 0, 0, // padding
            //
            3, 0, 0, 0, // name offset (item 1)
            5, 0, 0, 0, // name length
            // 13, 0, 0, 0, // function pub index
            1, // export
            0, 0, 0, // padding
        ];

        section_data.extend_from_slice("foo".as_bytes());
        section_data.extend_from_slice("hello".as_bytes());

        let section = FunctionNameSection::load(&section_data);

        assert_eq!(section.items.len(), 2);
        assert_eq!(section.items[0], FunctionNameItem::new(0, 3, /*11,*/ 0));
        assert_eq!(section.items[1], FunctionNameItem::new(3, 5, /*13,*/ 1));
        assert_eq!(section.names_data, "foohello".as_bytes())
    }

    #[test]
    fn test_convert() {
        let entries: Vec<FunctionNameEntry> = vec![
            FunctionNameEntry::new("foo".to_string(), /*11,*/ false),
            FunctionNameEntry::new("hello".to_string(), /*13,*/ true),
        ];

        let (items, names_data) = FunctionNameSection::convert_from_entries(&entries);
        let section = FunctionNameSection {
            items: &items,
            names_data: &names_data,
        };

        assert_eq!(
            section.get_item_index_and_export("foo"),
            Some((0, /*11,*/ false))
        );

        assert_eq!(
            section.get_item_index_and_export("hello"),
            Some((1, /*13,*/ true))
        );

        assert_eq!(section.get_item_index_and_export("bar"), None);
    }
}
