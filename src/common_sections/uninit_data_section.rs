// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

// "Uninit Data Section" binary layout:
//
//              |---------------------------------------------------------------------|
//              | item count (u32) | extra header length (u32)                        |
//              |---------------------------------------------------------------------|
//  item 0 -->  | data offset 0 (u32) | data length 0 (u32) | memory data type 0 (u8) |
//              | pad (1 byte) | data align 0 (u16)                                   | <-- table
//  item 1 -->  | data offset 1       | data length 1       | memory data type 1      |
//              | pad          | data align 1                                         |
//              | ...                                                                 |
//              |---------------------------------------------------------------------|

use anc_isa::MemoryDataType;

use crate::{
    datatableaccess::{read_section_with_one_table, write_section_with_one_table},
    entry::UninitDataEntry,
    module_image::{ModuleSectionId, SectionEntry, DATA_ITEM_ALIGN_BYTES},
};

#[derive(Debug, PartialEq, Default)]
pub struct UninitDataSection<'a> {
    pub items: &'a [DataItem], // Array of data items in the section
}

#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct DataItem {
    pub data_offset: u32, // Offset of the data item in the section's "data area"
    pub data_length: u32, // Length (in bytes) of the data item in the section's "data area"

    // The data type field is not required at runtime but is useful for debugging.
    pub memory_data_type: MemoryDataType,

    _padding0: u8, // Padding for alignment

    // Alignment of the data item itself.
    //
    // This field is not required for runtime data loading and storing because the `data_offset`
    // already implies the alignment at compilation time. The `data_offset` is aligned to
    // `DATA_ITEM_ALIGN_BYTES` (8 bytes). However, this field is necessary for cases where data
    // is copied into other memory (e.g., copying a struct from the data section into the heap),
    // as the alignment is needed.
    //
    // The value of this field depends on the data type and should never be `0`.
    //
    // | Type    | Size | Alignment |
    // |---------|------|-----------|
    // | i32     | 4    | 4         |
    // | i64     | 8    | 8         |
    // | f32     | 4    | 4         |
    // | f64     | 8    | 8         |
    // | bytes   | -    | -         |
    //
    // If the content of the data is a "struct" or other structured object, the data type "byte"
    // should be used, and the alignment should be specified. It is usually equal to the maximum
    // alignment of the fields. For instance, it is 8 if a struct contains an `i64` field.
    pub data_align: u16,
}

impl DataItem {
    pub fn new(
        data_offset: u32,
        data_length: u32,
        data_type: MemoryDataType,
        data_align: u16,
    ) -> Self {
        DataItem {
            data_offset,
            data_length,
            memory_data_type: data_type,
            _padding0: 0,
            data_align,
        }
    }
}

impl<'a> SectionEntry<'a> for UninitDataSection<'a> {
    fn id(&'a self) -> ModuleSectionId {
        ModuleSectionId::UninitData
    }

    fn read(section_data: &'a [u8]) -> Self
    where
        Self: Sized,
    {
        let items = read_section_with_one_table::<DataItem>(section_data);
        UninitDataSection { items }
    }

    fn write(&'a self, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        write_section_with_one_table(self.items, writer)
    }
}

impl UninitDataSection<'_> {
    pub fn convert_to_entries(&self) -> Vec<UninitDataEntry> {
        self.items
            .iter()
            .map(|item| UninitDataEntry {
                memory_data_type: item.memory_data_type,
                length: item.data_length,
                align: item.data_align,
            })
            .collect()
    }

    pub fn convert_from_entries(entries: &[UninitDataEntry]) -> Vec<DataItem> {
        let mut next_offset: u32 = 0;

        // Calculate the position `(padding, data_offset, data_length)` for each entry
        let positions = entries
            .iter()
            .map(|entry| {
                // The alignment of the record should be a multiple of `DATA_ITEM_ALIGN_BYTES` (8 bytes)
                let entry_align = entry.align as u32;
                let head_align = DATA_ITEM_ALIGN_BYTES as u32;
                let actual_align = (entry_align / head_align
                    + if entry_align % head_align != 0 { 1 } else { 0 })
                    * head_align;

                let remainder = next_offset % actual_align; // Remainder
                let head_padding = if remainder != 0 {
                    actual_align - remainder
                } else {
                    0
                };

                let data_offset = next_offset + head_padding; // Data offset after aligning
                let data_length = entry.length;
                next_offset = data_offset + data_length;
                (head_padding, data_offset, data_length)
            })
            .collect::<Vec<(u32, u32, u32)>>();

        let items = entries
            .iter()
            .zip(&positions)
            .map(|(entry, (_padding, data_offset, data_length))| {
                DataItem::new(
                    *data_offset,
                    *data_length,
                    entry.memory_data_type,
                    entry.align,
                )
            })
            .collect::<Vec<DataItem>>();

        items
    }
}

#[cfg(test)]
mod tests {
    use anc_isa::MemoryDataType;

    use crate::{
        common_sections::uninit_data_section::{DataItem, UninitDataSection},
        entry::UninitDataEntry,
        module_image::SectionEntry,
    };

    #[test]
    fn test_write_section() {
        let data_entry0 = UninitDataEntry::from_i32();
        let data_entry1 = UninitDataEntry::from_i64();
        let data_entry2 = UninitDataEntry::from_bytes(5, 1);
        let data_entry3 = UninitDataEntry::from_f32();
        let data_entry4 = UninitDataEntry::from_f64();
        let data_entry5 = UninitDataEntry::from_bytes(3, 8);
        let data_entry6 = UninitDataEntry::from_i64();
        let data_entry7 = UninitDataEntry::from_i32();

        let entries = vec![
            data_entry0,
            data_entry1,
            data_entry2,
            data_entry3,
            data_entry4,
            data_entry5,
            data_entry6,
            data_entry7,
        ];

        let items = UninitDataSection::convert_from_entries(&entries);
        let section = UninitDataSection { items: &items };

        let mut section_data: Vec<u8> = vec![];
        section.write(&mut section_data).unwrap();

        let expect_data = vec![
            8u8, 0, 0, 0, // item count
            0, 0, 0, 0, // padding
            //
            0, 0, 0, 0, // offset 0
            4, 0, 0, 0, // length
            0, // type
            0, // padding
            4, 0, // align
            //
            8, 0, 0, 0, // offset 1
            8, 0, 0, 0, // length
            1, // type
            0, // padding
            8, 0, // align
            //
            16, 0, 0, 0, // offset 2
            5, 0, 0, 0, // length
            4, // type
            0, // padding
            1, 0, // align
            //
            24, 0, 0, 0, // offset 3
            4, 0, 0, 0, // length
            2, // type
            0, // padding
            4, 0, // align
            //
            32, 0, 0, 0, // offset 4
            8, 0, 0, 0, // length
            3, // type
            0, // padding
            8, 0, // align
            //
            40, 0, 0, 0, // offset 5
            3, 0, 0, 0, // length
            4, // type
            0, // padding
            8, 0, // align
            //
            48, 0, 0, 0, // offset 6
            8, 0, 0, 0, // length
            1, // type
            0, // padding
            8, 0, // align
            //
            56, 0, 0, 0, // offset 7
            4, 0, 0, 0, // length
            0, // type
            0, // padding
            4, 0, // align
        ];

        assert_eq!(section_data, expect_data);
    }

    #[test]
    fn test_read_section() {
        let section_data = vec![
            8u8, 0, 0, 0, // item count
            0, 0, 0, 0, // padding
            //
            0, 0, 0, 0, // offset 0
            4, 0, 0, 0, // length
            0, // type
            0, // padding
            4, 0, // align
            //
            8, 0, 0, 0, // offset 1
            8, 0, 0, 0, // length
            1, // type
            0, // padding
            8, 0, // align
            //
            16, 0, 0, 0, // offset 2
            5, 0, 0, 0, // length
            4, // type
            0, // padding
            1, 0, // align
            //
            24, 0, 0, 0, // offset 3
            4, 0, 0, 0, // length
            2, // type
            0, // padding
            4, 0, // align
            //
            32, 0, 0, 0, // offset 4
            8, 0, 0, 0, // length
            3, // type
            0, // padding
            8, 0, // align
            //
            40, 0, 0, 0, // offset 5
            3, 0, 0, 0, // length
            4, // type
            0, // padding
            8, 0, // align
            //
            48, 0, 0, 0, // offset 6
            8, 0, 0, 0, // length
            1, // type
            0, // padding
            8, 0, // align
            //
            56, 0, 0, 0, // offset 7
            4, 0, 0, 0, // length
            0, // type
            0, // padding
            4, 0, // align
        ];

        let section = UninitDataSection::read(&section_data);
        assert_eq!(
            section.items,
            &[
                DataItem::new(0, 4, MemoryDataType::I32, 4),
                DataItem::new(8, 8, MemoryDataType::I64, 8),
                DataItem::new(16, 5, MemoryDataType::Bytes, 1),
                DataItem::new(24, 4, MemoryDataType::F32, 4),
                DataItem::new(32, 8, MemoryDataType::F64, 8),
                DataItem::new(40, 3, MemoryDataType::Bytes, 8),
                DataItem::new(48, 8, MemoryDataType::I64, 8),
                DataItem::new(56, 4, MemoryDataType::I32, 4),
            ]
        );
    }

    #[test]
    fn test_convert() {
        let data_entry0 = UninitDataEntry::from_i32();
        let data_entry1 = UninitDataEntry::from_i64();
        let data_entry2 = UninitDataEntry::from_bytes(5, 1);
        let data_entry3 = UninitDataEntry::from_f32();
        let data_entry4 = UninitDataEntry::from_f64();
        let data_entry5 = UninitDataEntry::from_bytes(3, 8);
        let data_entry6 = UninitDataEntry::from_i64();
        let data_entry7 = UninitDataEntry::from_i32();

        let entries = vec![
            data_entry0,
            data_entry1,
            data_entry2,
            data_entry3,
            data_entry4,
            data_entry5,
            data_entry6,
            data_entry7,
        ];

        let items = UninitDataSection::convert_from_entries(&entries);
        let section = UninitDataSection { items: &items };

        let entries_restore = section.convert_to_entries();
        assert_eq!(entries_restore, entries);
    }
}
