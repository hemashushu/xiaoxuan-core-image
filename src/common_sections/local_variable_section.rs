// Copyright (c) 2025 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions.
// For more details, see the LICENSE, LICENSE.additional, and CONTRIBUTING files.

// "Local Variable Section" binary layout:
//
//              |-----------------------------------------------|
//              | item count (u32) | extra header length (u32)  |
//              |-----------------------------------------------|
//  item 0 -->  | list offset 0 (u32) | list item count 0 (u32) |
//              | allocated bytes 0 (u32)                       | <-- table
//  item 1 -->  | list offset 1       | list item count 1       |
//              | allocated bytes 1                             |
//              | ...                                           |
//              |-----------------------------------------------|
// offset 0 --> | list data 0                                   | <-- data
// offset 1 --> | list data 1                                   |
//              | ...                                           |
//              |-----------------------------------------------|
//
// Each "list data" is also a table, the layout of "list data" is:
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
//             the details of list data 0
//            |------------------------------------------------------------|
// item 0 --> | var offset 0 (u32) | var actual length 0 (u32)             |
//            | memory data type 0 (u8) | pad (1 byte) | var align 0 (u16) |
// item 1 --> | var offset 1       | var actual length 1                   |
//            | memory data type 1      | pad          | var align 1       |
//            | ...                                                        |
//            |------------------------------------------------------------|

// Notes:
// - All variables in the 'local variable area' MUST be 8-byte aligned, and their size should be padded to a multiple of 8.
//   For example, an i32 will be padded to 8 bytes, and a struct with 12 bytes will be padded to 16 bytes.
//   This is because the current VM implementation allocates the 'local variable area' on the stack frame,
//   and the stack address is 8-byte aligned.
// - The local variable list also includes function arguments. The compiler automatically places arguments
//   at the beginning of the list as local variables.
// - Both functions and blocks can contain a local variable list.

use std::mem::size_of;

use anc_isa::{MemoryDataType, OPERAND_SIZE_IN_BYTES};

use crate::{
    datatableaccess::{
        read_section_with_table_and_data_area, write_section_with_table_and_data_area,
    },
    entry::{LocalVariableEntry, LocalVariableListEntry},
    module_image::{ModuleSectionId, SectionEntry},
};

#[derive(Debug, PartialEq)]
pub struct LocalVariableSection<'a> {
    pub lists: &'a [LocalVariableList],
    pub list_data: &'a [u8],
}

// A list per function
#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct LocalVariableList {
    pub list_offset: u32,
    pub list_item_count: u32,

    // The allocated bytes of local variables and arguments of a function or block.
    // This is the size of the 'local variable area' in the stack frame.
    //
    // Note that all variables in the 'local variable area' MUST be 8-byte aligned,
    // and their size is padded to a multiple of 8.
    // So the value of this field will always be a multiple of 8.
    pub allocated_bytes: u32,
}

#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct LocalVariableItem {
    pub var_offset: u32, // Offset of the variable in the "local variable area"

    // 'var_actual_length' is the actual size (in bytes) of the variable, not the padded size in the local variable area.
    // For example:
    // - An i32 has an actual length of 4 bytes but occupies 8 bytes in the local variable area.
    // - An i64 has an actual length of 8 bytes and occupies 8 bytes in the local variable area.
    // - A struct's actual length includes padding between fields or after the last field.
    //   For example, a struct `{u8, u16}` has an actual length of 4 bytes (1 byte + 1 padding + 2 bytes),
    //   but it occupies 8 bytes in the local variable area (4 bytes of extra padding added at the end).
    pub var_actual_length: u32,

    pub memory_data_type: MemoryDataType, // Type of the variable (e.g., i32, i64, etc.)
    _padding0: u8,                        // Padding for alignment

    // 'var_align' specifies the alignment of the variable. It is not required for loading/storing local variables
    // (since they are always 8-byte aligned in the local variable area), but it is needed when copying data
    // into other memory (e.g., copying a struct from the local variable area to the heap).
    // - For byte arrays (including strings), the value should be 1.
    // - For structs, the value should be the maximum alignment of its fields.
    // - The maximum alignment value is 8, and the minimum is 1.
    pub var_align: u16,
}

impl LocalVariableItem {
    pub fn new(
        var_offset: u32,
        var_actual_length: u32,
        data_type: MemoryDataType,
        var_align: u16,
    ) -> Self {
        Self {
            var_offset,
            var_actual_length,
            memory_data_type: data_type,
            _padding0: 0,
            var_align,
        }
    }
}

impl LocalVariableList {
    pub fn new(list_offset: u32, list_item_count: u32, allocated_bytes: u32) -> Self {
        Self {
            list_offset,
            list_item_count,
            allocated_bytes,
        }
    }
}

impl<'a> SectionEntry<'a> for LocalVariableSection<'a> {
    fn id(&'a self) -> ModuleSectionId {
        ModuleSectionId::LocalVariable
    }

    fn read(section_data: &'a [u8]) -> Self
    where
        Self: Sized,
    {
        let (lists, datas) =
            read_section_with_table_and_data_area::<LocalVariableList>(section_data);
        LocalVariableSection {
            lists,
            list_data: datas,
        }
    }

    fn write(&'a self, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        write_section_with_table_and_data_area(self.lists, self.list_data, writer)
    }
}

impl<'a> LocalVariableSection<'a> {
    /// Retrieves the local variable list at the specified index.
    pub fn get_local_variable_list(&'a self, idx: usize) -> &'a [LocalVariableItem] {
        let list = &self.lists[idx];

        let list_offset = list.list_offset as usize;
        let item_count = list.list_item_count as usize;
        let items_data = &self.list_data
            [list_offset..(list_offset + item_count * size_of::<LocalVariableItem>())];
        let items_ptr = items_data.as_ptr() as *const LocalVariableItem;
        let items = std::ptr::slice_from_raw_parts(items_ptr, item_count);
        unsafe { &*items }
    }

    /// Converts the section into a vector of `LocalVariableListEntry` objects.
    pub fn convert_to_entries(&self) -> Vec<LocalVariableListEntry> {
        let lists = &self.lists;
        let list_data = &self.list_data;

        lists
            .iter()
            .map(|list| {
                let list_offset = list.list_offset as usize;
                let item_count = list.list_item_count as usize;
                let items_data = &list_data
                    [list_offset..(list_offset + item_count * size_of::<LocalVariableItem>())];
                let items_ptr = items_data.as_ptr() as *const LocalVariableItem;
                let items = std::ptr::slice_from_raw_parts(items_ptr, item_count);
                let items_ref = unsafe { &*items };

                let local_variable_entries = items_ref
                    .iter()
                    .map(|item| LocalVariableEntry {
                        memory_data_type: item.memory_data_type,
                        length: item.var_actual_length,
                        align: item.var_align,
                    })
                    .collect();

                LocalVariableListEntry {
                    local_variable_entries,
                }
            })
            .collect()
    }

    /// Converts a vector of `LocalVariableListEntry` objects into the section's internal representation.
    pub fn convert_from_entries(
        entries: &[LocalVariableListEntry],
    ) -> (Vec<LocalVariableList>, Vec<u8>) {
        const LOCAL_VARIABLE_ITEM_LENGTH_IN_RECORD_IN_BYTES: usize = size_of::<LocalVariableItem>();

        // Generate a list of (list, variables_allocated_bytes)
        let items_list_with_variables_allocated_bytes = entries
            .iter()
            .map(|list_entry| {
                // A function contains a variable list
                // A list contains several entries

                // The offset in the list
                let mut var_offset_next: u32 = 0;

                let items = list_entry
                    .local_variable_entries
                    .iter()
                    .map(|var_entry| {
                        let item = LocalVariableItem::new(
                            var_offset_next,
                            var_entry.length,
                            var_entry.memory_data_type,
                            var_entry.align,
                        );

                        // Pad the length of variable/data to the multiple of 8
                        let padding = {
                            let remainder = var_entry.length % OPERAND_SIZE_IN_BYTES as u32; // Remainder
                            if remainder != 0 {
                                OPERAND_SIZE_IN_BYTES as u32 - remainder
                            } else {
                                0
                            }
                        };

                        let variables_allocated_bytes = var_entry.length + padding;
                        var_offset_next += variables_allocated_bytes;
                        item
                    })
                    .collect::<Vec<LocalVariableItem>>();

                // Now `var_offset_next` is the `variables_allocated_bytes * N`
                (items, var_offset_next)
            })
            .collect::<Vec<(Vec<LocalVariableItem>, u32)>>();

        // Make lists
        let mut list_offset_next: u32 = 0;
        let lists = items_list_with_variables_allocated_bytes
            .iter()
            .map(|(list, variables_allocated_bytes)| {
                let list_offset = list_offset_next;
                let list_item_count = list.len() as u32;
                list_offset_next += list_item_count * LOCAL_VARIABLE_ITEM_LENGTH_IN_RECORD_IN_BYTES as u32;

                LocalVariableList {
                    list_offset,
                    list_item_count,
                    allocated_bytes: *variables_allocated_bytes,
                }
            })
            .collect::<Vec<LocalVariableList>>();

        // Make data
        let list_data = items_list_with_variables_allocated_bytes
            .iter()
            .flat_map(|(list, _)| {
                let list_item_count = list.len();
                let total_length_in_bytes = list_item_count * LOCAL_VARIABLE_ITEM_LENGTH_IN_RECORD_IN_BYTES;

                let mut buf: Vec<u8> = Vec::with_capacity(total_length_in_bytes);
                let dst = buf.as_mut_ptr();
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
    use anc_isa::MemoryDataType;

    use crate::{
        common_sections::local_variable_section::{
            LocalVariableItem, LocalVariableList, LocalVariableSection,
        },
        entry::{LocalVariableEntry, LocalVariableListEntry},
        module_image::SectionEntry,
    };

    #[test]
    fn test_write_section() {
        let entries = vec![
            LocalVariableListEntry::new(vec![
                LocalVariableEntry::from_i32(),
                LocalVariableEntry::from_i64(),
                LocalVariableEntry::from_f32(),
                LocalVariableEntry::from_f64(),
            ]),
            LocalVariableListEntry::new(vec![
                LocalVariableEntry::from_i32(),
                LocalVariableEntry::from_bytes(1, 2),
                LocalVariableEntry::from_i32(),
                LocalVariableEntry::from_bytes(6, 12),
                LocalVariableEntry::from_bytes(12, 16),
                LocalVariableEntry::from_i32(),
            ]),
            LocalVariableListEntry::new(vec![]),
            LocalVariableListEntry::new(vec![LocalVariableEntry::from_bytes(1, 4)]),
            LocalVariableListEntry::new(vec![]),
            LocalVariableListEntry::new(vec![]),
            LocalVariableListEntry::new(vec![LocalVariableEntry::from_i32()]),
        ];

        let (lists, list_data) = LocalVariableSection::convert_from_entries(&entries);

        let section = LocalVariableSection {
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
                7u8, 0, 0, 0, // Item count
                0, 0, 0, 0, // Extra section header len (i32)
                //
                // Table
                //
                0, 0, 0, 0, // Offset
                4, 0, 0, 0, // Count
                32, 0, 0, 0, // Slot bytes
                //
                48, 0, 0, 0, // Offset = 4 (count) * 12 (bytes/record)
                6, 0, 0, 0, // Count
                56, 0, 0, 0, // Slot bytes
                //
                120, 0, 0, 0, // Offset = 48 + (6 * 12)
                0, 0, 0, 0, // Count
                0, 0, 0, 0, // Slot bytes
                //
                120, 0, 0, 0, // Offset = 120 + 0
                1, 0, 0, 0, // Count
                8, 0, 0, 0, // Slot bytes
                //
                132, 0, 0, 0, // Offset = 120 + (1 * 12)
                0, 0, 0, 0, // Count
                0, 0, 0, 0, // Slot bytes
                //
                132, 0, 0, 0, // Offset = 132 + 0
                0, 0, 0, 0, // Count
                0, 0, 0, 0, // Slot bytes
                //
                132, 0, 0, 0, // Offset = 132 + 0
                1, 0, 0, 0, // Count
                8, 0, 0, 0, // Slot bytes
                //
                // Data
                //
                // List 0
                0, 0, 0, 0, // Var offset (i32)
                4, 0, 0, 0, // Var len
                0, // Data type
                0, // Padding
                4, 0, // Align
                //
                8, 0, 0, 0, // Var offset (i64)
                8, 0, 0, 0, // Var len
                1, // Data type
                0, // Padding
                8, 0, // Align
                //
                16, 0, 0, 0, // Var offset (f32)
                4, 0, 0, 0, // Var len
                2, // Data type
                0, // Padding
                4, 0, // Align
                //
                24, 0, 0, 0, // Var offset (f64)
                8, 0, 0, 0, // Var len
                3, // Data type
                0, // Padding
                8, 0, // Align
                //
                // List 1
                0, 0, 0, 0, // Var offset (i32)
                4, 0, 0, 0, // Var len
                0, // Data type
                0, // Padding
                4, 0, // Align
                //
                8, 0, 0, 0, // Var offset (byte len 1 align 2)
                1, 0, 0, 0, // Var len
                4, // Data type
                0, // Padding
                2, 0, // Align
                //
                16, 0, 0, 0, // Var offset (i32)
                4, 0, 0, 0, // Var len
                0, // Data type
                0, // Padding
                4, 0, // Align
                //
                24, 0, 0, 0, // Var offset (byte len 6 align 12)
                6, 0, 0, 0, // Var len
                4, // Data type
                0, // Padding
                12, 0, // Align
                //
                32, 0, 0, 0, // Var offset (byte len 12 align 16)
                12, 0, 0, 0, // Var len
                4, // Data type
                0, // Padding
                16, 0, // Align
                //
                48, 0, 0, 0, // Var offset (i32)
                4, 0, 0, 0, // Var len
                0, // Data type
                0, // Padding
                4, 0, // Align
                //
                // List 3
                0, 0, 0, 0, // Var offset (byte len 1 align 4)
                1, 0, 0, 0, // Var len
                4, // Data type
                0, // Padding
                4, 0, // Align
                //
                // List 6
                0, 0, 0, 0, // Var offset (i32)
                4, 0, 0, 0, // Var len
                0, // Data type
                0, // Padding
                4, 0 // Align
            ]
        );
    }

    #[test]
    fn test_read_section() {
        let section_data = vec![
            //
            // Header
            //
            7u8, 0, 0, 0, // Item count
            0, 0, 0, 0, // Extra section header len (i32)
            //
            // Table
            //
            0, 0, 0, 0, // Offset
            4, 0, 0, 0, // Count
            32, 0, 0, 0, // Slot bytes
            //
            48, 0, 0, 0, // Offset = 4 (count) * 12 (bytes/record)
            6, 0, 0, 0, // Count
            56, 0, 0, 0, // Slot bytes
            //
            120, 0, 0, 0, // Offset = 48 + (6 * 12)
            0, 0, 0, 0, // Count
            0, 0, 0, 0, // Slot bytes
            //
            120, 0, 0, 0, // Offset = 120 + 0
            1, 0, 0, 0, // Count
            8, 0, 0, 0, // Slot bytes
            //
            132, 0, 0, 0, // Offset = 120 + (1 * 12)
            0, 0, 0, 0, // Count
            0, 0, 0, 0, // Slot bytes
            //
            132, 0, 0, 0, // Offset = 132 + 0
            0, 0, 0, 0, // Count
            0, 0, 0, 0, // Slot bytes
            //
            132, 0, 0, 0, // Offset = 132 + 0
            1, 0, 0, 0, // Count
            8, 0, 0, 0, // Slot bytes
            //
            // Data
            //
            // List 0
            0, 0, 0, 0, // Var offset (i32)
            4, 0, 0, 0, // Var len
            0, // Data type
            0, // Padding
            4, 0, // Align
            //
            8, 0, 0, 0, // Var offset (i64)
            8, 0, 0, 0, // Var len
            1, // Data type
            0, // Padding
            8, 0, // Align
            //
            16, 0, 0, 0, // Var offset (f32)
            4, 0, 0, 0, // Var len
            2, // Data type
            0, // Padding
            4, 0, // Align
            //
            24, 0, 0, 0, // Var offset (f64)
            8, 0, 0, 0, // Var len
            3, // Data type
            0, // Padding
            8, 0, // Align
            //
            // List 1
            0, 0, 0, 0, // Var offset (i32)
            4, 0, 0, 0, // Var len
            0, // Data type
            0, // Padding
            4, 0, // Align
            //
            8, 0, 0, 0, // Var offset (byte len 1 align 2)
            1, 0, 0, 0, // Var len
            4, // Data type
            0, // Padding
            2, 0, // Align
            //
            16, 0, 0, 0, // Var offset (i32)
            4, 0, 0, 0, // Var len
            0, // Data type
            0, // Padding
            4, 0, // Align
            //
            24, 0, 0, 0, // Var offset (byte len 6 align 12)
            6, 0, 0, 0, // Var len
            4, // Data type
            0, // Padding
            12, 0, // Align
            //
            32, 0, 0, 0, // Var offset (byte len 12 align 16)
            12, 0, 0, 0, // Var len
            4, // Data type
            0, // Padding
            16, 0, // Align
            //
            48, 0, 0, 0, // Var offset (i32)
            4, 0, 0, 0, // Var len
            0, // Data type
            0, // Padding
            4, 0, // Align
            //
            // List 3
            0, 0, 0, 0, // Var offset (byte len 1 align 4)
            1, 0, 0, 0, // Var len
            4, // Data type
            0, // Padding
            4, 0, // Align
            //
            // List 6
            0, 0, 0, 0, // Var offset (i32)
            4, 0, 0, 0, // Var len
            0, // Data type
            0, // Padding
            4, 0, // Align
        ];

        let section = LocalVariableSection::read(&section_data);

        assert_eq!(section.lists.len(), 7);

        // Check lists

        assert_eq!(
            section.lists[0],
            LocalVariableList {
                list_offset: 0,
                list_item_count: 4,
                allocated_bytes: 32
            }
        );

        assert_eq!(
            section.lists[1],
            LocalVariableList {
                list_offset: 48,
                list_item_count: 6,
                allocated_bytes: 56
            }
        );

        assert_eq!(
            section.lists[2],
            LocalVariableList {
                list_offset: 120,
                list_item_count: 0,
                allocated_bytes: 0
            }
        );

        assert_eq!(
            section.lists[3],
            LocalVariableList {
                list_offset: 120,
                list_item_count: 1,
                allocated_bytes: 8
            }
        );

        assert_eq!(
            section.lists[4],
            LocalVariableList {
                list_offset: 132,
                list_item_count: 0,
                allocated_bytes: 0
            }
        );

        assert_eq!(
            section.lists[5],
            LocalVariableList {
                list_offset: 132,
                list_item_count: 0,
                allocated_bytes: 0
            }
        );

        assert_eq!(
            section.lists[6],
            LocalVariableList {
                list_offset: 132,
                list_item_count: 1,
                allocated_bytes: 8
            }
        );

        // Check var items

        let list0 = section.get_local_variable_list(0);
        assert_eq!(
            list0,
            &[
                LocalVariableItem::new(0, 4, MemoryDataType::I32, 4),
                LocalVariableItem::new(8, 8, MemoryDataType::I64, 8),
                LocalVariableItem::new(16, 4, MemoryDataType::F32, 4),
                LocalVariableItem::new(24, 8, MemoryDataType::F64, 8),
            ]
        );

        let list1 = section.get_local_variable_list(1);
        assert_eq!(
            list1,
            &[
                LocalVariableItem::new(0, 4, MemoryDataType::I32, 4),
                LocalVariableItem::new(8, 1, MemoryDataType::Bytes, 2),
                LocalVariableItem::new(16, 4, MemoryDataType::I32, 4),
                LocalVariableItem::new(24, 6, MemoryDataType::Bytes, 12),
                LocalVariableItem::new(32, 12, MemoryDataType::Bytes, 16),
                LocalVariableItem::new(48, 4, MemoryDataType::I32, 4),
            ]
        );

        let list2 = section.get_local_variable_list(2);
        assert_eq!(list2.len(), 0);

        let list3 = section.get_local_variable_list(3);
        assert_eq!(
            list3,
            &[LocalVariableItem::new(0, 1, MemoryDataType::Bytes, 4),]
        );

        let list4 = section.get_local_variable_list(4);
        assert_eq!(list4.len(), 0);

        let list5 = section.get_local_variable_list(5);
        assert_eq!(list5.len(), 0);

        let list6 = section.get_local_variable_list(6);
        assert_eq!(
            list6,
            &[LocalVariableItem::new(0, 4, MemoryDataType::I32, 4),]
        );
    }

    #[test]
    fn test_convert() {
        let entries = vec![
            LocalVariableListEntry::new(vec![
                LocalVariableEntry::from_i32(),
                LocalVariableEntry::from_i64(),
                LocalVariableEntry::from_f32(),
                LocalVariableEntry::from_f64(),
            ]),
            LocalVariableListEntry::new(vec![
                LocalVariableEntry::from_i32(),
                LocalVariableEntry::from_bytes(1, 2),
                LocalVariableEntry::from_i32(),
                LocalVariableEntry::from_bytes(6, 12),
                LocalVariableEntry::from_bytes(12, 16),
                LocalVariableEntry::from_i32(),
            ]),
            LocalVariableListEntry::new(vec![]),
            LocalVariableListEntry::new(vec![LocalVariableEntry::from_bytes(1, 4)]),
            LocalVariableListEntry::new(vec![]),
            LocalVariableListEntry::new(vec![]),
            LocalVariableListEntry::new(vec![LocalVariableEntry::from_i32()]),
        ];

        let (lists, list_data) = LocalVariableSection::convert_from_entries(&entries);

        let section = LocalVariableSection {
            lists: &lists,
            list_data: &list_data,
        };

        let entries_restore = section.convert_to_entries();
        assert_eq!(entries_restore, entries);
    }
}
