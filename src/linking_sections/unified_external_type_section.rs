// Copyright (c) 2025 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions.
// For more details, see the LICENSE, LICENSE.additional, and CONTRIBUTING files.

// "Unified External Type Section" binary layout:
//
//                     |------------------------------------------------|
//                     | item count (u32) | extra header length (u32)   |
//                     |------------------------------------------------|
//          item 0 --> | params count 0 (u16) | results count 0 (u16)   |
//                     | params offset 0 (u32) | results offset 0 (u32) | <-- table
//          item 1 --> | params count 1       | results count 1         |
//                     | params offset 1       | results offset 1       |
//                     | ...                                            |
//                     |------------------------------------------------|
//  param offset 0 --> | parameter data type list 0                     | <-- data
// result offset 0 --> | result data type list 0                        |
//  param offset 1 --> | parameter data type list 1                     |
// result offset 1 --> | result data type list 1                        |
//                     | ...                                            |
//                     |------------------------------------------------|
//
// The binary layout of this section is identical to `TypeSection`.

use std::ptr::slice_from_raw_parts;

use anc_isa::OperandDataType;

use crate::{
    datatableaccess::{
        read_section_with_table_and_data_area, write_section_with_table_and_data_area,
    },
    entry::TypeEntry,
    module_image::{ModuleSectionId, SectionEntry},
};

#[derive(Debug, PartialEq, Default)]
pub struct UnifiedExternalTypeSection<'a> {
    pub items: &'a [TypeItem], // Array of type items in the section.
    pub types_data: &'a [u8],  // Raw data for parameter and result types.
}

#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct TypeItem {
    // Number of parameters. Since each data type is 1 byte, this also represents
    // the length (in bytes) of the "parameter type list" in the data area.
    pub params_count: u16,

    // Number of results. Similarly, this represents the length (in bytes) of the
    // "result type list" in the data area.
    pub results_count: u16,

    // Offset of the "parameter type list" in the data area.
    pub params_offset: u32,

    // Offset of the "result type list" in the data area.
    pub results_offset: u32,
}

impl TypeItem {
    pub fn new(
        params_count: u16,
        results_count: u16,
        params_offset: u32,
        results_offset: u32,
    ) -> Self {
        Self {
            params_count,
            results_count,
            params_offset,
            results_offset,
        }
    }
}

impl<'a> SectionEntry<'a> for UnifiedExternalTypeSection<'a> {
    fn read(section_data: &'a [u8]) -> Self {
        // Reads the section data and splits it into items and types_data.
        let (items, types_data) = read_section_with_table_and_data_area::<TypeItem>(section_data);
        UnifiedExternalTypeSection { items, types_data }
    }

    fn write(&'a self, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        // Writes the section data, including the table and data area.
        write_section_with_table_and_data_area(self.items, self.types_data, writer)
    }

    fn id(&'a self) -> ModuleSectionId {
        ModuleSectionId::UnifiedExternalType
    }
}

impl<'a> UnifiedExternalTypeSection<'a> {
    pub fn get_item_params_and_results(
        &'a self,
        idx: usize,
    ) -> (&'a [OperandDataType], &'a [OperandDataType]) {
        // Retrieves the parameter and result types for a specific item by index.
        let items = self.items;
        let types_data = self.types_data;

        let item = &items[idx];

        let params_data = &types_data[(item.params_offset as usize)
            ..(item.params_offset as usize + item.params_count as usize)];
        let results_data = &types_data[(item.results_offset as usize)
            ..(item.results_offset as usize + item.results_count as usize)];

        let params_slice = unsafe {
            &*slice_from_raw_parts(
                params_data.as_ptr() as *const OperandDataType,
                item.params_count as usize,
            )
        };

        let results_slice = unsafe {
            &*slice_from_raw_parts(
                results_data.as_ptr() as *const OperandDataType,
                item.results_count as usize,
            )
        };

        (params_slice, results_slice)
    }

    // Converts a specific item into a `TypeEntry` for inspection.
    pub fn get_type_entry(&self, idx: usize) -> TypeEntry {
        let (params, results) = self.get_item_params_and_results(idx);
        TypeEntry {
            params: params.to_vec(),
            results: results.to_vec(),
        }
    }

    // Converts all items in the section into a vector of `TypeEntry`.
    pub fn convert_to_entries(&self) -> Vec<TypeEntry> {
        let items = &self.items;
        let types_data = &self.types_data;

        items
            .iter()
            .map(|item| {
                let params_data = &types_data[(item.params_offset as usize)
                    ..(item.params_offset as usize + item.params_count as usize)];
                let results_data = &types_data[(item.results_offset as usize)
                    ..(item.results_offset as usize + item.results_count as usize)];

                let params_slice = unsafe {
                    &*slice_from_raw_parts(
                        params_data.as_ptr() as *const OperandDataType,
                        item.params_count as usize,
                    )
                };

                let results_slice = unsafe {
                    &*slice_from_raw_parts(
                        results_data.as_ptr() as *const OperandDataType,
                        item.results_count as usize,
                    )
                };

                TypeEntry {
                    params: params_slice.to_vec(),
                    results: results_slice.to_vec(),
                }
            })
            .collect()
    }

    // Converts a vector of `TypeEntry` into the section's binary representation.
    pub fn convert_from_entries(entries: &[TypeEntry]) -> (Vec<TypeItem>, Vec<u8>) {
        let mut next_offset: u32 = 0;

        let items = entries
            .iter()
            .map(|entry| {
                let params_count = entry.params.len() as u16;
                let params_offset = next_offset;
                let results_count = entry.results.len() as u16;
                let results_offset = params_offset + params_count as u32;

                // Since each data type is 1 byte, `results_count` also represents
                // the length (in bytes) of the result list.
                next_offset = results_offset + results_count as u32; // Update for next offset.

                TypeItem {
                    params_count,
                    results_count,
                    params_offset,
                    results_offset,
                }
            })
            .collect::<Vec<TypeItem>>();

        let types_data = entries
            .iter()
            .flat_map(|entry| {
                let mut bytes: Vec<u8> = vec![];
                let params_bytes =
                    slice_from_raw_parts(entry.params.as_ptr() as *const u8, entry.params.len());
                let results_bytes =
                    slice_from_raw_parts(entry.results.as_ptr() as *const u8, entry.results.len());
                bytes.extend_from_slice(unsafe { &*params_bytes });
                bytes.extend_from_slice(unsafe { &*results_bytes });
                bytes
            })
            .collect::<Vec<u8>>();

        (items, types_data)
    }
}

#[cfg(test)]
mod tests {
    use anc_isa::OperandDataType;

    use crate::{
        common_sections::type_section::{TypeItem, TypeSection},
        entry::TypeEntry,
        module_image::SectionEntry,
    };

    #[test]
    fn test_read_section() {
        let section_data = vec![
            3u8, 0, 0, 0, // item count
            0, 0, 0, 0, // extra section header len (i32)
            //
            2, 0, // param count
            3, 0, // result count
            0, 0, 0, 0, // param offset (item 0)
            2, 0, 0, 0, // result offset
            //
            1, 0, // param count
            0, 0, // result count
            5, 0, 0, 0, // param offset (item 1)
            6, 0, 0, 0, // result offset
            //
            4, 0, // param count
            1, 0, // result count
            6, 0, 0, 0, // param offset (item 2)
            10, 0, 0, 0, // result offset
            //
            1u8, 2, // param types 0
            3, 2, 1, // result types 0
            4, // param types 1
            // result types 1
            4, 3, 2, 1, // param types 2
            1, // result types 2
        ];

        let section = TypeSection::read(&section_data);

        assert_eq!(section.items.len(), 3);
        assert_eq!(
            section.items[0],
            TypeItem {
                params_count: 2,
                results_count: 3,
                params_offset: 0,
                results_offset: 2,
            }
        );
        assert_eq!(
            section.items[1],
            TypeItem {
                params_count: 1,
                results_count: 0,
                params_offset: 5,
                results_offset: 6,
            }
        );
        assert_eq!(
            section.items[2],
            TypeItem {
                params_count: 4,
                results_count: 1,
                params_offset: 6,
                results_offset: 10,
            }
        );
    }

    #[test]
    fn test_write_section() {
        let items = vec![
            TypeItem {
                params_count: 2,
                results_count: 3,
                params_offset: 0,
                results_offset: 2,
            },
            TypeItem {
                params_count: 1,
                results_count: 0,
                params_offset: 5,
                results_offset: 6,
            },
            TypeItem {
                params_count: 4,
                results_count: 1,
                params_offset: 6,
                results_offset: 10,
            },
        ];

        let section = TypeSection {
            items: &items,
            types_data: &[
                1u8, 2, // param types 0
                3, 2, 1, // result types 0
                4, // param types 1
                // result types 1
                4, 3, 2, 1, // param types 2
                1, // result types 2
            ],
        };

        let mut section_data: Vec<u8> = vec![];
        section.write(&mut section_data).unwrap();

        assert_eq!(
            section_data,
            vec![
                3u8, 0, 0, 0, // item count
                0, 0, 0, 0, // extra section header len (i32)
                //
                2, 0, // param count
                3, 0, // result count
                0, 0, 0, 0, // param offset (item 0)
                2, 0, 0, 0, // result offset
                //
                1, 0, // param count
                0, 0, // result count
                5, 0, 0, 0, // param offset (item 1)
                6, 0, 0, 0, // result offset
                //
                4, 0, // param count
                1, 0, // result count
                6, 0, 0, 0, // param offset (item 2)
                10, 0, 0, 0, // result offset
                //
                1u8, 2, // param types 0
                3, 2, 1, // result types 0
                4, // param types 1
                // result types 1
                4, 3, 2, 1, // param types 2
                1, // result types 2
                //
                0, // padding for 4-byte align
            ]
        );
    }

    #[test]
    fn test_convert() {
        let entries = vec![
            TypeEntry {
                params: vec![OperandDataType::I32, OperandDataType::I64],
                results: vec![OperandDataType::I32],
            },
            TypeEntry {
                params: vec![OperandDataType::I64],
                results: vec![OperandDataType::I64, OperandDataType::I32],
            },
            TypeEntry {
                params: vec![],
                results: vec![OperandDataType::F32],
            },
            TypeEntry {
                params: vec![],
                results: vec![],
            },
        ];

        let (items, types_data) = TypeSection::convert_from_entries(&entries);
        let section = TypeSection {
            items: &items,
            types_data: &types_data,
        };

        assert_eq!(
            section.get_item_params_and_results(0),
            (
                vec![OperandDataType::I32, OperandDataType::I64].as_ref(),
                vec![OperandDataType::I32].as_ref()
            )
        );

        assert_eq!(
            section.get_item_params_and_results(1),
            (
                vec![OperandDataType::I64].as_ref(),
                vec![OperandDataType::I64, OperandDataType::I32].as_ref()
            )
        );

        assert_eq!(
            section.get_item_params_and_results(2),
            ([].as_ref(), vec![OperandDataType::F32].as_ref())
        );

        assert_eq!(
            section.get_item_params_and_results(3),
            ([].as_ref(), [].as_ref())
        );

        let entries_restore = section.convert_to_entries();
        assert_eq!(entries_restore, entries);
    }
}
