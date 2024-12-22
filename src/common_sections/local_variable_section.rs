// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

// "local variable section" binary layout
//
//              |-----------------------------------------------------------------------------|
//              | item count (u32) | (4 bytes padding)                                        |
//              |-----------------------------------------------------------------------------|
//  item 0 -->  | list offset 0 (u32) | list item count 0 (u32) | list allocate bytes 0 (u32) | <-- table
//  item 1 -->  | list offset 1       | list item count 1       | list allocate bytes 1       |
//              | ...                                                                         |
//              |-----------------------------------------------------------------------------|
// offset 0 --> | list data 0                                                                 | <-- data area
// offset 1 --> | list data 1                                                                 |
//              | ...                                                                         |
//              |-----------------------------------------------------------------------------|
//
//
// the "list" is also a table, the layout of "list":
//
//          |--------|     |----------------------------------------------------------------------------------------------------------|
// list     | item 0 | --> | var offset 0 (u32) | var actual length 0 (u32) | mem data type 0 (u8) | pad (1 byte) | var align 0 (u16) |
// data0 -> | item 1 | --> | var offset 1       | var actual length 1       | mem data type 1      |              | var align 1       |
//          | ...    |     | ...                                                                                                      |
//          |--------|     |----------------------------------------------------------------------------------------------------------|
//
// note:
// - all variables in the 'local variable area' MUST be 8-byte aligned,
//   and their size should be padded to a multiple of 8.
//   for example, an i32 will be padded to 8 bytes, and a struct with 12 bytes will
//   be padded to 16 (= 8 * 2) bytes.
//   this is because the current VM implementation allocates 'local variables area' on the stack frame,
//   and the stack address is 8-byte aligned.
// - the local variable list also includes the functions arguments. the compiler will
//   put arguments to the beginning of the list as local variables automatically.
// - both function and block can contain a local variables list.

use std::mem::size_of;

use anc_isa::{MemoryDataType, OPERAND_SIZE_IN_BYTES};

use crate::{
    entry::{LocalVariableEntry, LocalVariableListEntry},
    module_image::{ModuleSectionId, SectionEntry},
    tableaccess::{read_section_with_table_and_data_area, write_section_with_table_and_data_area},
};

#[derive(Debug, PartialEq)]
pub struct LocalVariableSection<'a> {
    pub lists: &'a [LocalVariableList],
    pub list_data: &'a [u8],
}

// a list per function
#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct LocalVariableList {
    pub list_offset: u32,
    pub list_item_count: u32,

    // note that all variables in the 'local variable area' MUST be 8-byte aligned,
    // and their size are padded to a multiple of 8.
    // so the value of this filed will be always the multiple of 8.
    pub list_allocate_bytes: u32,
}

#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct LocalVariableItem {
    pub var_offset: u32, // the offset of a data item in the "local variables area"

    // 'var_actual_length' is the actual length (in bytes) of a variable/data,
    // not the length of the item in the local variable area.
    // also note that all variable is allocated with the length (in bytes) of multiple of 8.
    //
    // e.g.
    // - the actual length of 'i32' is 4 bytes, but in the local variable area, 'i32' occurs 8 byets.
    // - the actual length of 'i64' is 8 bytes, and occurs 8 bytes in the local variable area.
    // - the actual length of a struct is `size_of(struct)`, note that the size contains the padding
    //   either between fields or after the last field.
    //   e.g. the actual length of 'struct {u8, u16}' is '1 + 1 padding + 2' = 4 bytes,
    //   which includes 1 byte between the two fields.
    //   and in the local variable area, this struct occurs 8 bytes, that is, there are
    //   extra 4 bytes added to the end.
    pub var_actual_length: u32,

    // the memory_data_type field is not necessary at runtime, though it is helpful for debugging.
    pub memory_data_type: MemoryDataType,

    _padding0: u8,

    // the var_align field is not necessary for local variables loading and storing,
    // the local variable is always 8-byte aligned in the local variable area currently,
    // but it is needed for copying data into other memory, such as
    // copying a struct from local variables area to heap.
    //
    // if the content of data is a byte array (includes string), the value should be 1,
    // if the content of data is a struct, the value should be the max one of the length of its fields.
    // currently the MAX value of align is 8, MIN value is 1.
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
    pub fn new(list_offset: u32, list_item_count: u32, list_allocate_bytes: u32) -> Self {
        Self {
            list_offset,
            list_item_count,
            list_allocate_bytes,
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
    pub fn get_local_variable_list(&'a self, idx: usize) -> &'a [LocalVariableItem] {
        let list = &self.lists[idx];

        let list_offset = list.list_offset as usize;
        let item_count = list.list_item_count as usize;
        let items_data =
            &self.list_data[list_offset..(list_offset + item_count * size_of::<LocalVariableItem>())];
        let items_ptr = items_data.as_ptr() as *const LocalVariableItem;
        let items = std::ptr::slice_from_raw_parts(items_ptr, item_count);
        unsafe { &*items }
    }

//     // for inspect
//     pub fn get_local_variable_list_entry(&self, idx: usize) -> LocalVariableListEntry {
//         let items = self.get_local_variable_list(idx);
//
//         let local_variable_entries = items
//             .iter()
//             .map(|item| LocalVariableEntry {
//                 memory_data_type: item.memory_data_type,
//                 length: item.var_actual_length,
//                 align: item.var_align,
//             })
//             .collect::<Vec<_>>();
//
//         LocalVariableListEntry {
//             local_variable_entries,
//         }
//     }

    pub fn convert_to_entries(&self) -> Vec<LocalVariableListEntry> {
        let lists = &self.lists;
        let list_data = &self.list_data;

        lists
            .iter()
            .map(|list| {
                let list_offset = list.list_offset as usize;
                let item_count = list.list_item_count as usize;
                let items_data =
                    &list_data[list_offset..(list_offset + item_count * size_of::<LocalVariableItem>())];
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

    pub fn convert_from_entries(
        entiress: &[LocalVariableListEntry],
    ) -> (Vec<LocalVariableList>, Vec<u8>) {
        const LOCAL_VARIABLE_ITEM_LENGTH_IN_BYTES:usize = size_of::<LocalVariableItem>();

        // generate a list of (list, list_allocate_bytes)
        let list_with_allocate_bytes = entiress
            .iter()
            .map(|list_entry| {
                // a function contains a variable list
                // a list contains several entries

                // the offset in the list
                let mut next_offset: u32 = 0;

                let items = list_entry
                    .local_variable_entries
                    .iter()
                    .map(|var_entry| {
                        let item = LocalVariableItem::new(
                            next_offset,
                            var_entry.length,
                            var_entry.memory_data_type,
                            var_entry.align,
                        );

                        // pad the length of variable/data to the multiple of 8
                        let padding = {
                            let remainder = var_entry.length % OPERAND_SIZE_IN_BYTES as u32; // remainder
                            if remainder != 0 {
                                OPERAND_SIZE_IN_BYTES as u32 - remainder
                            } else {
                                0
                            }
                        };

                        let var_allocated_in_bytes = var_entry.length + padding;
                        next_offset += var_allocated_in_bytes;
                        item
                    })
                    .collect::<Vec<LocalVariableItem>>();

                // here 'next_offset' is the list_allocate_bytes
                (items, next_offset)
            })
            .collect::<Vec<(Vec<LocalVariableItem>, u32)>>();

        // make lists
        let mut next_offset: u32 = 0;
        let lists = list_with_allocate_bytes
            .iter()
            .map(|(list, list_allocate_bytes)| {
                let list_offset = next_offset;
                let list_item_count = list.len() as u32;
                next_offset += list_item_count * LOCAL_VARIABLE_ITEM_LENGTH_IN_BYTES as u32;

                LocalVariableList {
                    list_offset,
                    list_item_count,
                    list_allocate_bytes: *list_allocate_bytes,
                }
            })
            .collect::<Vec<LocalVariableList>>();

        // make data
        let list_data = list_with_allocate_bytes
            .iter()
            .flat_map(|(list, _list_allocate_bytes)| {
                let list_item_count = list.len();
                let total_length_in_bytes = list_item_count * LOCAL_VARIABLE_ITEM_LENGTH_IN_BYTES;

                // let mut buf: Vec<u8> = vec![0u8; total_length_in_bytes];
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

        let mut section_data: Vec<u8> = Vec::new();
        section.write(&mut section_data).unwrap();

        assert_eq!(
            section_data,
            vec![
                //
                // header
                //
                7u8, 0, 0, 0, // item count
                0, 0, 0, 0, // 4 bytes padding
                //
                // table
                //
                0, 0, 0, 0, // offset
                4, 0, 0, 0, // count
                32, 0, 0, 0, // alloc bytes
                //
                48, 0, 0, 0, // offset = 4 (count) * 12 (bytes/record)
                6, 0, 0, 0, // count
                56, 0, 0, 0, // alloc bytes
                //
                120, 0, 0, 0, // offset = 48 + (6 * 12)
                0, 0, 0, 0, // count
                0, 0, 0, 0, // alloc bytes
                //
                120, 0, 0, 0, // offset = 120 + 0
                1, 0, 0, 0, // count
                8, 0, 0, 0, // alloc bytes
                //
                132, 0, 0, 0, // offset = 120 + (1 * 12)
                0, 0, 0, 0, // count
                0, 0, 0, 0, // alloc bytes
                //
                132, 0, 0, 0, // offset = 132 + 0
                0, 0, 0, 0, // count
                0, 0, 0, 0, // alloc bytes
                //
                132, 0, 0, 0, // offset = 132 + 0
                1, 0, 0, 0, // count
                8, 0, 0, 0, // alloc bytes
                //
                // data
                //
                // list 0
                0, 0, 0, 0, // var offset (i32)
                4, 0, 0, 0, // var len
                0, // data type
                0, // padding
                4, 0, // align
                //
                8, 0, 0, 0, // var offset (i64)
                8, 0, 0, 0, // var len
                1, // data type
                0, // padding
                8, 0, // align
                //
                16, 0, 0, 0, // var offset (f32)
                4, 0, 0, 0, // var len
                2, // data type
                0, // padding
                4, 0, // align
                //
                24, 0, 0, 0, // var offset (f64)
                8, 0, 0, 0, // var len
                3, // data type
                0, // padding
                8, 0, // align
                //
                // list 1
                0, 0, 0, 0, // var offset (i32)
                4, 0, 0, 0, // var len
                0, // data type
                0, // padding
                4, 0, // align
                //
                8, 0, 0, 0, // var offset (byte len 1 align 2)
                1, 0, 0, 0, // var len
                4, // data type
                0, // padding
                2, 0, // align
                //
                16, 0, 0, 0, // var offset (i32)
                4, 0, 0, 0, // var len
                0, // data type
                0, // padding
                4, 0, // align
                //
                24, 0, 0, 0, // var offset (byte len 6 align 12)
                6, 0, 0, 0, // var len
                4, // data type
                0, // padding
                12, 0, // align
                //
                32, 0, 0, 0, // var offset (byte len 12 align 16)
                12, 0, 0, 0, // var len
                4, // data type
                0, // padding
                16, 0, // align
                //
                48, 0, 0, 0, // var offset (i32)
                4, 0, 0, 0, // var len
                0, // data type
                0, // padding
                4, 0, // align
                //
                // list 3
                0, 0, 0, 0, // var offset (byte len 1 align 4)
                1, 0, 0, 0, // var len
                4, // data type
                0, // padding
                4, 0, // align
                //
                // list 6
                0, 0, 0, 0, // var offset (i32)
                4, 0, 0, 0, // var len
                0, // data type
                0, // padding
                4, 0 // align
            ]
        );
    }

    #[test]
    fn test_read_section() {
        let section_data = vec![
            //
            // header
            //
            7u8, 0, 0, 0, // item count
            0, 0, 0, 0, // 4 bytes padding
            //
            // table
            //
            0, 0, 0, 0, // offset
            4, 0, 0, 0, // count
            32, 0, 0, 0, // alloc bytes
            //
            48, 0, 0, 0, // offset = 4 (count) * 12 (bytes/record)
            6, 0, 0, 0, // count
            56, 0, 0, 0, // alloc bytes
            //
            120, 0, 0, 0, // offset = 48 + (6 * 12)
            0, 0, 0, 0, // count
            0, 0, 0, 0, // alloc bytes
            //
            120, 0, 0, 0, // offset = 120 + 0
            1, 0, 0, 0, // count
            8, 0, 0, 0, // alloc bytes
            //
            132, 0, 0, 0, // offset = 120 + (1 * 12)
            0, 0, 0, 0, // count
            0, 0, 0, 0, // alloc bytes
            //
            132, 0, 0, 0, // offset = 132 + 0
            0, 0, 0, 0, // count
            0, 0, 0, 0, // alloc bytes
            //
            132, 0, 0, 0, // offset = 132 + 0
            1, 0, 0, 0, // count
            8, 0, 0, 0, // alloc bytes
            //
            // data
            //
            // list 0
            0, 0, 0, 0, // var offset (i32)
            4, 0, 0, 0, // var len
            0, // data type
            0, // padding
            4, 0, // align
            //
            8, 0, 0, 0, // var offset (i64)
            8, 0, 0, 0, // var len
            1, // data type
            0, // padding
            8, 0, // align
            //
            16, 0, 0, 0, // var offset (f32)
            4, 0, 0, 0, // var len
            2, // data type
            0, // padding
            4, 0, // align
            //
            24, 0, 0, 0, // var offset (f64)
            8, 0, 0, 0, // var len
            3, // data type
            0, // padding
            8, 0, // align
            //
            // list 1
            0, 0, 0, 0, // var offset (i32)
            4, 0, 0, 0, // var len
            0, // data type
            0, // padding
            4, 0, // align
            //
            8, 0, 0, 0, // var offset (byte len 1 align 2)
            1, 0, 0, 0, // var len
            4, // data type
            0, // padding
            2, 0, // align
            //
            16, 0, 0, 0, // var offset (i32)
            4, 0, 0, 0, // var len
            0, // data type
            0, // padding
            4, 0, // align
            //
            24, 0, 0, 0, // var offset (byte len 6 align 12)
            6, 0, 0, 0, // var len
            4, // data type
            0, // padding
            12, 0, // align
            //
            32, 0, 0, 0, // var offset (byte len 12 align 16)
            12, 0, 0, 0, // var len
            4, // data type
            0, // padding
            16, 0, // align
            //
            48, 0, 0, 0, // var offset (i32)
            4, 0, 0, 0, // var len
            0, // data type
            0, // padding
            4, 0, // align
            //
            // list 3
            0, 0, 0, 0, // var offset (byte len 1 align 4)
            1, 0, 0, 0, // var len
            4, // data type
            0, // padding
            4, 0, // align
            //
            // list 6
            0, 0, 0, 0, // var offset (i32)
            4, 0, 0, 0, // var len
            0, // data type
            0, // padding
            4, 0, // align
        ];

        let section = LocalVariableSection::read(&section_data);

        assert_eq!(section.lists.len(), 7);

        // check lists

        assert_eq!(
            section.lists[0],
            LocalVariableList {
                list_offset: 0,
                list_item_count: 4,
                list_allocate_bytes: 32
            }
        );

        assert_eq!(
            section.lists[1],
            LocalVariableList {
                list_offset: 48,
                list_item_count: 6,
                list_allocate_bytes: 56
            }
        );

        assert_eq!(
            section.lists[2],
            LocalVariableList {
                list_offset: 120,
                list_item_count: 0,
                list_allocate_bytes: 0
            }
        );

        assert_eq!(
            section.lists[3],
            LocalVariableList {
                list_offset: 120,
                list_item_count: 1,
                list_allocate_bytes: 8
            }
        );

        assert_eq!(
            section.lists[4],
            LocalVariableList {
                list_offset: 132,
                list_item_count: 0,
                list_allocate_bytes: 0
            }
        );

        assert_eq!(
            section.lists[5],
            LocalVariableList {
                list_offset: 132,
                list_item_count: 0,
                list_allocate_bytes: 0
            }
        );

        assert_eq!(
            section.lists[6],
            LocalVariableList {
                list_offset: 132,
                list_item_count: 1,
                list_allocate_bytes: 8
            }
        );

        // check var items

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
        // todo
    }
}
