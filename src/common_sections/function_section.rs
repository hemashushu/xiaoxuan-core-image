// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

// "function section" binary layout
//
//              |-------------------------------------------------------------------------------------------|
//              | item count (u32) | (4 bytes padding)                                                      |
//              |-------------------------------------------------------------------------------------------|
//   item 0 --> | code offset 0 (u32) | code length 0 (u32) | type index 0 (u32) | local list index 0 (u32) |  <-- table
//   item 1 --> | code offset 1       | code length 1       | type index 1       | local list index 1       |
//              | ...                                                                                       |
//              |-------------------------------------------------------------------------------------------|
// offset 0 --> | code 0                                                                                    | <-- data area
// offset 1 --> | code 1                                                                                    |
//              | ...                                                                                       |
//              |-------------------------------------------------------------------------------------------|

use crate::{
    entry::FunctionEntry,
    module_image::{ModuleSectionId, SectionEntry},
    tableaccess::{read_section_with_table_and_data_area, write_section_with_table_and_data_area},
};

#[derive(Debug, PartialEq)]
pub struct FunctionSection<'a> {
    pub items: &'a [FunctionItem],
    pub codes_data: &'a [u8],
}

#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct FunctionItem {
    pub code_offset: u32,               // the offset of the code in data area
    pub code_length: u32,               // the length (in bytes) of the code in data area
    pub type_index: u32,                // the index of the type (of function)
    pub local_variable_list_index: u32, // the index of the 'local variable list'
}

impl FunctionItem {
    pub fn new(
        code_offset: u32,
        code_length: u32,
        type_index: u32,
        local_variable_list_index: u32,
    ) -> Self {
        Self {
            code_offset,
            code_length,
            type_index,
            local_variable_list_index,
        }
    }
}

impl<'a> SectionEntry<'a> for FunctionSection<'a> {
    fn read(section_data: &'a [u8]) -> Self {
        let (items, codes_data) =
            read_section_with_table_and_data_area::<FunctionItem>(section_data);
        FunctionSection { items, codes_data }
    }

    fn write(&'a self, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        write_section_with_table_and_data_area(self.items, self.codes_data, writer)
    }

    fn id(&'a self) -> ModuleSectionId {
        ModuleSectionId::Function
    }
}

impl<'a> FunctionSection<'a> {
    pub fn get_item_type_index_and_local_variable_list_index_and_code(
        &'a self,
        idx: usize,
    ) -> (usize, usize, &'a [u8]) {
        let items = self.items;
        let codes_data = self.codes_data;

        let item = &items[idx];
        let code_data =
            &codes_data[item.code_offset as usize..(item.code_offset + item.code_length) as usize];

        (
            item.type_index as usize,
            item.local_variable_list_index as usize,
            code_data,
        )
    }

    //     // for inspect
    //     pub fn get_function_entry(&self, idx: usize) -> FunctionEntry {
    //         let item = &self.items[idx];
    //         let code = self.codes_data
    //             [item.code_offset as usize..(item.code_offset + item.code_length) as usize]
    //             .to_vec();
    //
    //         FunctionEntry {
    //             type_index: item.type_index as usize,
    //             local_variable_list_index: item.local_variable_list_index as usize,
    //             code,
    //         }
    //     }

    pub fn convert_to_entries(&self) -> Vec<FunctionEntry> {
        let items = self.items;
        let codes_data = self.codes_data;

        items
            .iter()
            .map(|item| {
                let code = codes_data
                    [item.code_offset as usize..(item.code_offset + item.code_length) as usize]
                    .to_vec();

                FunctionEntry::new(
                    item.type_index as usize,
                    item.local_variable_list_index as usize,
                    code,
                )
            })
            .collect()
    }

    pub fn convert_from_entries(entries: &[FunctionEntry]) -> (Vec<FunctionItem>, Vec<u8>) {
        let mut next_offset: u32 = 0;

        let items = entries
            .iter()
            .map(|entry| {
                let code_offset = next_offset;
                let code_length = entry.code.len() as u32;
                next_offset += code_length; // for next offset
                FunctionItem::new(
                    code_offset,
                    code_length,
                    entry.type_index as u32,
                    entry.local_variable_list_index as u32,
                )
            })
            .collect::<Vec<FunctionItem>>();

        let codes_data = entries
            .iter()
            .flat_map(|entry| entry.code.clone())
            .collect::<Vec<u8>>();

        (items, codes_data)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        common_sections::function_section::{FunctionItem, FunctionSection},
        entry::FunctionEntry,
        module_image::SectionEntry,
    };

    #[test]
    fn test_read_section() {
        let mut section_data = vec![
            2u8, 0, 0, 0, // item count
            0, 0, 0, 0, // 4 bytes padding
            //
            3, 0, 0, 0, // code offset (item 0)
            5, 0, 0, 0, // code length
            7, 0, 0, 0, // function type index
            11, 0, 0, 0, // local variable list index
            //
            13, 0, 0, 0, // code offset (item 1)
            17, 0, 0, 0, // code length
            19, 0, 0, 0, // function type index
            23, 0, 0, 0, // local variable list index
        ];

        section_data.extend_from_slice(b"hello0123456789a");

        let section = FunctionSection::read(&section_data);

        assert_eq!(section.items.len(), 2);
        assert_eq!(section.items[0], FunctionItem::new(3, 5, 7, 11));
        assert_eq!(section.items[1], FunctionItem::new(13, 17, 19, 23));
        assert_eq!(section.codes_data, b"hello0123456789a")
    }

    #[test]
    fn test_write_section() {
        let items = vec![
            FunctionItem::new(3, 5, 7, 11),
            FunctionItem::new(13, 17, 19, 23),
        ];

        let section = FunctionSection {
            items: &items,
            codes_data: b"hello0123456789a",
        };

        let mut section_data: Vec<u8> = vec![];
        section.write(&mut section_data).unwrap();

        let mut expect_data = vec![
            2u8, 0, 0, 0, // item count
            0, 0, 0, 0, // 4 bytes padding
            //
            3, 0, 0, 0, // code offset (item 0)
            5, 0, 0, 0, // code length
            7, 0, 0, 0, // function type index
            11, 0, 0, 0, // local variable list index
            //
            13, 0, 0, 0, // code offset (item 1)
            17, 0, 0, 0, // code length
            19, 0, 0, 0, // function type index
            23, 0, 0, 0, // local variable list index
        ];

        expect_data.extend_from_slice(b"hello0123456789a");

        assert_eq!(section_data, expect_data);
    }

    #[test]
    fn test_convert() {
        let entries = vec![
            FunctionEntry {
                type_index: 7,
                local_variable_list_index: 9,
                code: b"bar".to_vec(),
            },
            FunctionEntry {
                type_index: 11,
                local_variable_list_index: 13,
                code: b"world".to_vec(),
            },
        ];

        let (items, codes_data) = FunctionSection::convert_from_entries(&entries);
        let section = FunctionSection {
            items: &items,
            codes_data: &codes_data,
        };

        assert_eq!(
            section.get_item_type_index_and_local_variable_list_index_and_code(0),
            (7, 9, b"bar".to_vec().as_ref())
        );

        assert_eq!(
            section.get_item_type_index_and_local_variable_list_index_and_code(1),
            (11, 13, b"world".to_vec().as_ref())
        );

        let entries_restore = section.convert_to_entries();
        assert_eq!(entries, entries_restore);
    }
}
