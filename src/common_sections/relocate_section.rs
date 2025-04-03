// Copyright (c) 2025 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions.
// For more details, see the LICENSE, LICENSE.additional, and CONTRIBUTING files.

// "Relocate Section" binary layout:
//
//              |-----------------------------------------------|
//              | item count (u32) | extra header length (u32)  |
//              |-----------------------------------------------|
//  item 0 -->  | list offset 0 (u32) | list item count 0 (u32) | <-- table
//  item 1 -->  | list offset 1       | list item count 1       |
//              | ...                                           |
//              |-----------------------------------------------|
// offset 0 --> | list data 0                                   | <-- data area
// offset 1 --> | list data 1                                   |
//              | ...                                           |
//              |-----------------------------------------------|
//
// Each "list data" is a table with the following layout:
//
//                |--------|
// list data 0 -> | item 0 |
//                | item 1 |
//                | ...    |
//                |--------|
// list data 1 -> | item 0 |
//                | item 1 |
//                | ...    |
//                |--------|
//
// The details of each list data:
//
//             list data 0
//            |--------------------------------------------------------|
// item 0 --> | stub offset 0 (u32) | stub type 0 (u8) | pad (3 bytes) |
// item 1 --> | stub offset 1       | stub type 1      |               |
//            | ...                                                    |
//            |--------------------------------------------------------|

use crate::{
    datatableaccess::{
        read_section_with_table_and_data_area, write_section_with_table_and_data_area,
    },
    entry::{RelocateEntry, RelocateListEntry},
    module_image::{ModuleSectionId, RelocateType, SectionEntry},
};

#[derive(Debug, PartialEq, Default)]
pub struct RelocateSection<'a> {
    pub lists: &'a [RelocateList],
    pub list_data: &'a [u8],
}

// A list per function
#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct RelocateList {
    pub list_offset: u32,
    pub list_item_count: u32,
}

#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct RelocateItem {
    // Offset within a function where relocation is required.
    pub offset_in_function: u32,

    // Type of relocation (e.g., type index, function index, etc.).
    pub relocate_type: RelocateType,

    // Padding to ensure proper alignment (3 bytes).
    _padding0: [u8; 3],
}

impl RelocateItem {
    pub fn new(offset_in_function: u32, relocate_type: RelocateType) -> Self {
        Self {
            offset_in_function,
            relocate_type,
            _padding0: [0_u8; 3],
        }
    }
}

impl RelocateList {
    pub fn new(list_offset: u32, list_item_count: u32) -> Self {
        Self {
            list_offset,
            list_item_count,
        }
    }
}

impl<'a> SectionEntry<'a> for RelocateSection<'a> {
    fn id(&'a self) -> ModuleSectionId {
        ModuleSectionId::Relocate
    }

    fn read(section_data: &'a [u8]) -> Self
    where
        Self: Sized,
    {
        let (lists, datas) = read_section_with_table_and_data_area::<RelocateList>(section_data);
        RelocateSection {
            lists,
            list_data: datas,
        }
    }

    fn write(&'a self, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        write_section_with_table_and_data_area(self.lists, self.list_data, writer)
    }
}

impl<'a> RelocateSection<'a> {
    // Retrieves the relocation list for a specific function by index.
    pub fn get_relocate_list(&'a self, idx: usize) -> &'a [RelocateItem] {
        let list = &self.lists[idx];

        let list_offset = list.list_offset as usize;
        let item_count = list.list_item_count as usize;
        let items_data =
            &self.list_data[list_offset..(list_offset + item_count * size_of::<RelocateItem>())];
        let items_ptr = items_data.as_ptr() as *const RelocateItem;
        let items = std::ptr::slice_from_raw_parts(items_ptr, item_count);
        unsafe { &*items }
    }

    // Converts the section into a vector of `RelocateListEntry` objects for easier manipulation.
    pub fn convert_to_entries(&self) -> Vec<RelocateListEntry> {
        let lists = &self.lists;
        let list_data = &self.list_data;

        lists
            .iter()
            .map(|list| {
                let list_offset = list.list_offset as usize;
                let item_count = list.list_item_count as usize;
                let items_data =
                    &list_data[list_offset..(list_offset + item_count * size_of::<RelocateItem>())];
                let items_ptr = items_data.as_ptr() as *const RelocateItem;
                let items = std::ptr::slice_from_raw_parts(items_ptr, item_count);
                let items_ref = unsafe { &*items };

                let relocate_entries = items_ref
                    .iter()
                    .map(|item| RelocateEntry {
                        offset_in_function: item.offset_in_function as usize,
                        relocate_type: item.relocate_type,
                    })
                    .collect();

                RelocateListEntry { relocate_entries }
            })
            .collect()
    }

    // Converts a vector of `RelocateListEntry` objects back into the binary layout of the section.
    pub fn convert_from_entries(entires: &[RelocateListEntry]) -> (Vec<RelocateList>, Vec<u8>) {
        const RELOCATE_ITEM_LENGTH_IN_BYTES: usize = size_of::<RelocateItem>();

        let mut list_offset_next: u32 = 0;

        let items_list = entires
            .iter()
            .map(|list_entry| {
                // A function contains a relocate item list.
                // A list contains several relocate entries.
                list_entry
                    .relocate_entries
                    .iter()
                    .map(|var_entry| {
                        RelocateItem::new(
                            var_entry.offset_in_function as u32,
                            var_entry.relocate_type,
                        )
                    })
                    .collect::<Vec<RelocateItem>>()
            })
            .collect::<Vec<_>>();

        // Make lists
        let lists = items_list
            .iter()
            .map(|list| {
                let list_offset = list_offset_next;
                let list_item_count = list.len() as u32;
                list_offset_next += list_item_count * RELOCATE_ITEM_LENGTH_IN_BYTES as u32;

                RelocateList {
                    list_offset,
                    list_item_count,
                }
            })
            .collect::<Vec<_>>();

        // Make data
        let list_data = items_list
            .iter()
            .flat_map(|list| {
                let list_item_count = list.len();
                let total_length_in_bytes = list_item_count * RELOCATE_ITEM_LENGTH_IN_BYTES;

                let mut buf: Vec<u8> = Vec::with_capacity(total_length_in_bytes);
                let dst = buf.as_mut_ptr(); // as *mut u8;
                let src = list.as_ptr() as *const u8;

                unsafe {
                    std::ptr::copy(src, dst, total_length_in_bytes);
                    buf.set_len(total_length_in_bytes);
                }

                buf
            })
            .collect::<Vec<u8>>();

        (lists, list_data)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        common_sections::relocate_section::{RelocateItem, RelocateList},
        entry::{RelocateEntry, RelocateListEntry},
        module_image::{RelocateType, SectionEntry},
    };

    use super::RelocateSection;

    #[test]
    fn test_write_section() {
        let entries = vec![
            RelocateListEntry::new(vec![
                RelocateEntry::new(11, RelocateType::TypeIndex),
                RelocateEntry::new(13, RelocateType::LocalVariableListIndex),
                RelocateEntry::new(17, RelocateType::FunctionPublicIndex),
                RelocateEntry::new(19, RelocateType::DataPublicIndex),
            ]),
            RelocateListEntry::new(vec![
                RelocateEntry::new(23, RelocateType::ExternalFunctionIndex),
                RelocateEntry::new(29, RelocateType::FunctionPublicIndex),
            ]),
            RelocateListEntry::new(vec![]),
            RelocateListEntry::new(vec![
                RelocateEntry::new(31, RelocateType::DataPublicIndex),
                RelocateEntry::new(37, RelocateType::FunctionPublicIndex),
            ]),
            RelocateListEntry::new(vec![]),
            RelocateListEntry::new(vec![]),
            RelocateListEntry::new(vec![
                RelocateEntry::new(41, RelocateType::TypeIndex),
                RelocateEntry::new(43, RelocateType::LocalVariableListIndex),
                RelocateEntry::new(47, RelocateType::DataPublicIndex),
            ]),
        ];

        let (lists, list_data) = RelocateSection::convert_from_entries(&entries);

        let section = RelocateSection {
            lists: &lists,
            list_data: &list_data,
        };

        let mut section_data: Vec<u8> = vec![];
        section.write(&mut section_data).unwrap();

        assert_eq!(
            section_data,
            vec![
                //
                // Header
                //
                7, 0, 0, 0, // Item count
                0, 0, 0, 0, // Extra section header len (i32)
                //
                // Table
                //
                0, 0, 0, 0, // Offset 0
                4, 0, 0, 0, // Count
                //
                32, 0, 0, 0, // Offset 1 = 4 (count) * 8 (bytes/record)
                2, 0, 0, 0, // Count
                //
                48, 0, 0, 0, // Offset 2
                0, 0, 0, 0, // Count
                //
                48, 0, 0, 0, // Offset 3
                2, 0, 0, 0, // Count
                //
                64, 0, 0, 0, // Offset 4
                0, 0, 0, 0, // Count
                //
                64, 0, 0, 0, // Offset 5
                0, 0, 0, 0, // Count
                //
                64, 0, 0, 0, // Offset 6
                3, 0, 0, 0, // Count
                //
                // Data, list 0
                //
                11, 0, 0, 0, // Relocate offset
                0, // Relocate type
                0, 0, 0, // Padding
                //
                13, 0, 0, 0, // Relocate offset
                1, // Relocate type
                0, 0, 0, // Padding
                //
                17, 0, 0, 0, // Relocate offset
                2, // Relocate type
                0, 0, 0, // Padding
                //
                19, 0, 0, 0, // Relocate offset
                4, // Relocate type
                0, 0, 0, // Padding
                //
                // Data, list 1
                //
                23, 0, 0, 0, // Relocate offset
                3, // Relocate type
                0, 0, 0, // Padding
                //
                29, 0, 0, 0, // Relocate offset
                2, // Relocate type
                0, 0, 0, // Padding
                //
                // Data, list 3
                //
                31, 0, 0, 0, // Relocate offset
                4, // Relocate type
                0, 0, 0, // Padding
                //
                37, 0, 0, 0, // Relocate offset
                2, // Relocate type
                0, 0, 0, // Padding
                //
                // Data, list 6
                //
                41, 0, 0, 0, // Relocate offset
                0, // Relocate type
                0, 0, 0, // Padding
                //
                43, 0, 0, 0, // Relocate offset
                1, // Relocate type
                0, 0, 0, // Padding
                //
                47, 0, 0, 0, // Relocate offset
                4, // Relocate type
                0, 0, 0 // Padding
            ]
        );
    }

    #[test]
    fn test_read_section() {
        let section_data = vec![
            //
            // Header
            //
            7, 0, 0, 0, // Item count
            0, 0, 0, 0, // Extra section header len (i32)
            //
            // Table
            //
            0, 0, 0, 0, // Offset 0
            4, 0, 0, 0, // Count
            //
            32, 0, 0, 0, // Offset 1 = 4 (count) * 8 (bytes/record)
            2, 0, 0, 0, // Count
            //
            48, 0, 0, 0, // Offset 2
            0, 0, 0, 0, // Count
            //
            48, 0, 0, 0, // Offset 3
            2, 0, 0, 0, // Count
            //
            64, 0, 0, 0, // Offset 4
            0, 0, 0, 0, // Count
            //
            64, 0, 0, 0, // Offset 5
            0, 0, 0, 0, // Count
            //
            64, 0, 0, 0, // Offset 6
            3, 0, 0, 0, // Count
            //
            // Data, list 0
            //
            11, 0, 0, 0, // Relocate offset
            0, // Relocate type
            0, 0, 0, // Padding
            //
            13, 0, 0, 0, // Relocate offset
            1, // Relocate type
            0, 0, 0, // Padding
            //
            17, 0, 0, 0, // Relocate offset
            2, // Relocate type
            0, 0, 0, // Padding
            //
            19, 0, 0, 0, // Relocate offset
            4, // Relocate type
            0, 0, 0, // Padding
            //
            // Data, list 1
            //
            23, 0, 0, 0, // Relocate offset
            3, // Relocate type
            0, 0, 0, // Padding
            //
            29, 0, 0, 0, // Relocate offset
            2, // Relocate type
            0, 0, 0, // Padding
            //
            // Data, list 3
            //
            31, 0, 0, 0, // Relocate offset
            4, // Relocate type
            0, 0, 0, // Padding
            //
            37, 0, 0, 0, // Relocate offset
            2, // Relocate type
            0, 0, 0, // Padding
            //
            // Data, list 6
            //
            41, 0, 0, 0, // Relocate offset
            0, // Relocate type
            0, 0, 0, // Padding
            //
            43, 0, 0, 0, // Relocate offset
            1, // Relocate type
            0, 0, 0, // Padding
            //
            47, 0, 0, 0, // Relocate offset
            4, // Relocate type
            0, 0, 0, // Padding
        ];

        let section = RelocateSection::read(&section_data);

        assert_eq!(section.lists.len(), 7);

        // Check lists

        assert_eq!(
            section.lists[0],
            RelocateList {
                list_offset: 0,
                list_item_count: 4,
            }
        );

        assert_eq!(
            section.lists[1],
            RelocateList {
                list_offset: 32, // =4*8
                list_item_count: 2,
            }
        );

        assert_eq!(
            section.lists[2],
            RelocateList {
                list_offset: 48, // 32 + (2*8)
                list_item_count: 0,
            }
        );

        assert_eq!(
            section.lists[3],
            RelocateList {
                list_offset: 48,
                list_item_count: 2,
            }
        );

        assert_eq!(
            section.lists[4],
            RelocateList {
                list_offset: 64, // = 48 + (2*8)
                list_item_count: 0,
            }
        );

        assert_eq!(
            section.lists[5],
            RelocateList {
                list_offset: 64,
                list_item_count: 0,
            }
        );

        assert_eq!(
            section.lists[6],
            RelocateList {
                list_offset: 64,
                list_item_count: 3,
            }
        );

        // Check var items

        let list0 = section.get_relocate_list(0);
        assert_eq!(
            list0,
            &[
                RelocateItem::new(11, RelocateType::TypeIndex),
                RelocateItem::new(13, RelocateType::LocalVariableListIndex),
                RelocateItem::new(17, RelocateType::FunctionPublicIndex),
                RelocateItem::new(19, RelocateType::DataPublicIndex),
            ]
        );

        let list1 = section.get_relocate_list(1);
        assert_eq!(
            list1,
            &[
                RelocateItem::new(23, RelocateType::ExternalFunctionIndex),
                RelocateItem::new(29, RelocateType::FunctionPublicIndex),
            ]
        );

        let list2 = section.get_relocate_list(2);
        assert_eq!(list2.len(), 0);

        let list3 = section.get_relocate_list(3);
        assert_eq!(
            list3,
            &[
                RelocateItem::new(31, RelocateType::DataPublicIndex),
                RelocateItem::new(37, RelocateType::FunctionPublicIndex),
            ]
        );

        let list4 = section.get_relocate_list(4);
        assert_eq!(list4.len(), 0);

        let list5 = section.get_relocate_list(5);
        assert_eq!(list5.len(), 0);

        let list6 = section.get_relocate_list(6);
        assert_eq!(
            list6,
            &[
                RelocateItem::new(41, RelocateType::TypeIndex),
                RelocateItem::new(43, RelocateType::LocalVariableListIndex),
                RelocateItem::new(47, RelocateType::DataPublicIndex),
            ]
        );
    }

    #[test]
    fn test_convert() {
        let entries = vec![
            RelocateListEntry::new(vec![
                RelocateEntry::new(11, RelocateType::TypeIndex),
                RelocateEntry::new(13, RelocateType::LocalVariableListIndex),
                RelocateEntry::new(17, RelocateType::FunctionPublicIndex),
                RelocateEntry::new(19, RelocateType::DataPublicIndex),
            ]),
            RelocateListEntry::new(vec![
                RelocateEntry::new(23, RelocateType::ExternalFunctionIndex),
                RelocateEntry::new(29, RelocateType::FunctionPublicIndex),
            ]),
            RelocateListEntry::new(vec![]),
            RelocateListEntry::new(vec![
                RelocateEntry::new(31, RelocateType::DataPublicIndex),
                RelocateEntry::new(37, RelocateType::FunctionPublicIndex),
            ]),
            RelocateListEntry::new(vec![]),
            RelocateListEntry::new(vec![]),
            RelocateListEntry::new(vec![
                RelocateEntry::new(41, RelocateType::TypeIndex),
                RelocateEntry::new(43, RelocateType::LocalVariableListIndex),
                RelocateEntry::new(47, RelocateType::DataPublicIndex),
            ]),
        ];

        let (lists, list_data) = RelocateSection::convert_from_entries(&entries);

        let section = RelocateSection {
            lists: &lists,
            list_data: &list_data,
        };

        let entries_restore = section.convert_to_entries();
        assert_eq!(entries_restore, entries);
    }
}
