// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

// "Read-write Data Section" binary layout:
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
// offset 0 --> | data 0                                                              | <-- data
// offset 1 -->-| data 1                                                              |
//              |---------------------------------------------------------------------|

use anc_isa::MemoryDataType;

use crate::{
    datatableaccess::{
        read_section_with_table_and_data_area, write_section_with_table_and_data_area,
    },
    entry::ReadWriteDataEntry,
    module_image::{ModuleSectionId, SectionEntry, DATA_ITEM_ALIGN_BYTES},
};

#[derive(Debug, PartialEq, Default)]
pub struct ReadWriteDataSection<'a> {
    pub items: &'a [DataItem], // Array of data items in the section
    pub datas_data: &'a [u8],  // Raw data area of the section
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
    // This field is not required for data loading and storing because the `data_offset`
    // already implies the alignment at compilation time. (The `data_offset` is aligned to
    // `DATA_ITEM_ALIGN_BYTES`, which is 8 bytes).
    // However, this field is necessary for cases where data
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

impl<'a> SectionEntry<'a> for ReadWriteDataSection<'a> {
    fn id(&'a self) -> ModuleSectionId {
        ModuleSectionId::ReadWriteData
    }

    fn read(section_data: &'a [u8]) -> Self
    where
        Self: Sized,
    {
        let (items, datas) = read_section_with_table_and_data_area::<DataItem>(section_data);
        ReadWriteDataSection {
            items,
            datas_data: datas,
        }
    }

    fn write(&'a self, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        write_section_with_table_and_data_area(self.items, self.datas_data, writer)
    }
}

impl ReadWriteDataSection<'_> {
    pub fn convert_to_entries(&self) -> Vec<ReadWriteDataEntry> {
        let items = self.items;
        let datas_data = self.datas_data;
        items
            .iter()
            .map(|item| {
                let data = &datas_data
                    [item.data_offset as usize..(item.data_offset + item.data_length) as usize];

                ReadWriteDataEntry {
                    memory_data_type: item.memory_data_type,
                    data: data.to_vec(),
                    length: item.data_length,
                    align: item.data_align,
                }
            })
            .collect()
    }

    pub fn convert_from_entries(entries: &[ReadWriteDataEntry]) -> (Vec<DataItem>, Vec<u8>) {
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

        let datas_data = entries
            .iter()
            .zip(&positions)
            .flat_map(|(entry, (padding, _data_offset, _data_length))| {
                let mut data = vec![0u8; *padding as usize];
                data.extend(entry.data.iter());
                data
            })
            .collect::<Vec<u8>>();

        (items, datas_data)
    }
}

#[cfg(test)]
mod tests {
    use anc_isa::MemoryDataType;

    use crate::{
        common_sections::read_write_data_section::DataItem, entry::ReadWriteDataEntry,
        module_image::SectionEntry,
    };

    use super::ReadWriteDataSection;

    #[test]
    fn test_write_section() {
        let data_entry0 = ReadWriteDataEntry::from_i32(11);
        let data_entry1 = ReadWriteDataEntry::from_i64(13);
        let data_entry2 = ReadWriteDataEntry::from_bytes(b"hello".to_vec(), 1);
        let data_entry3 = ReadWriteDataEntry::from_f32(std::f32::consts::PI);
        let data_entry4 = ReadWriteDataEntry::from_f64(std::f64::consts::E);
        let data_entry5 = ReadWriteDataEntry::from_bytes(b"foo".to_vec(), 8);
        let data_entry6 = ReadWriteDataEntry::from_i64(17);
        let data_entry7 = ReadWriteDataEntry::from_i32(19);

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

        let (items, datas) = ReadWriteDataSection::convert_from_entries(&entries);
        let section = ReadWriteDataSection {
            items: &items,
            datas_data: &datas,
        };

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
            //
            // datas
            //
            11, 0, 0, 0, // data 0
            0, 0, 0, 0, // padding
            13, 0, 0, 0, 0, 0, 0, 0, // data 1
            104, 101, 108, 108, 111, // data 2, "hello"
            0, 0, 0, // padding
            // Float (IEEE754 Single precision 32-bit)
            // 0x4048F5C3 = 0 1000000 0  1001000 11110101 11000011
            //              ^ ^--------  ^------------------------
            //         sign | | exponent | 31400....
            //
            // https://www.binaryconvert.com/result_float.html?decimal=051046049052
            //
            219, 15, 73, 64, // data 3
            0, 0, 0, 0, // padding
            // Double (IEEE754 Double precision 64-bit)
            // 0x41B1E1A300000000 =
            // 0 1000001 1011 0001 11100001 10100011 00000000 00000000 00000000 00000000
            // ^ ^----------- ^------------------...
            // | | Exponent   | Mantissa
            // |
            // | sign
            //
            // https://www.binaryconvert.com/result_double.html?decimal=051048048048048048048048048
            105, 87, 20, 139, 10, 191, 5, 64, // data 4
            102, 111, 111, // data 5, "bar"
            0, 0, 0, 0, 0, // padding
            17, 0, 0, 0, 0, 0, 0, 0, // data 6
            19, 0, 0, 0, // data 7
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
            //
            // datas
            //
            11, 0, 0, 0, // data 0
            0, 0, 0, 0, // padding
            13, 0, 0, 0, 0, 0, 0, 0, // data 1
            104, 101, 108, 108, 111, // data 2, "hello"
            0, 0, 0, // padding
            // Float (IEEE754 Single precision 32-bit)
            // 0x4048F5C3 = 0 1000000 0  1001000 11110101 11000011
            //              ^ ^--------  ^------------------------
            //         sign | | exponent | 31400....
            //
            // https://www.binaryconvert.com/result_float.html?decimal=051046049052
            //
            195, 245, 72, 64, // data 3
            0, 0, 0, 0, // padding
            // Double (IEEE754 Double precision 64-bit)
            // 0x41B1E1A300000000 =
            // 0 1000001 1011 0001 11100001 10100011 00000000 00000000 00000000 00000000
            // ^ ^----------- ^------------------...
            // | | Exponent   | Mantissa
            // |
            // | sign
            //
            // https://www.binaryconvert.com/result_double.html?decimal=051048048048048048048048048
            0, 0, 0, 0, 163, 225, 177, 65, // data 4
            102, 111, 111, // data 5, "bar"
            0, 0, 0, 0, 0, // padding
            17, 0, 0, 0, 0, 0, 0, 0, // data 6
            19, 0, 0, 0, // data 7
        ];

        let section = ReadWriteDataSection::read(&section_data);

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

        // the data area is too long, only check partly here.
        assert_eq!(
            &section.datas_data[0..16],
            &[
                11u8, 0, 0, 0, // data 0
                0, 0, 0, 0, // padding
                13, 0, 0, 0, 0, 0, 0, 0, // data 1
            ]
        )
    }

    #[test]
    fn test_convert() {
        let data_entry0 = ReadWriteDataEntry::from_i32(11);
        let data_entry1 = ReadWriteDataEntry::from_i64(13);
        let data_entry2 = ReadWriteDataEntry::from_bytes(b"hello".to_vec(), 1);
        let data_entry3 = ReadWriteDataEntry::from_f32(std::f32::consts::PI);
        let data_entry4 = ReadWriteDataEntry::from_f64(std::f64::consts::E);
        let data_entry5 = ReadWriteDataEntry::from_bytes(b"foo".to_vec(), 8);
        let data_entry6 = ReadWriteDataEntry::from_i64(17);
        let data_entry7 = ReadWriteDataEntry::from_i32(19);

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

        let (items, datas) = ReadWriteDataSection::convert_from_entries(&entries);
        let section = ReadWriteDataSection {
            items: &items,
            datas_data: &datas,
        };

        let entries_restore = section.convert_to_entries();
        assert_eq!(entries_restore, entries);
    }
}
