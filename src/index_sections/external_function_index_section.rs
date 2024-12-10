// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

//! this section is used to map the:
//! `(module[current_module_index]).extcall(external_function_index)`
//! to
//! `unified_external_function_index`
//!
//! where
//! - `current_module_index` == `index_of_range_item`
//! - `items[range.offset + external_function_index]` is the entry of `external_function_index`

// "external function index section" binary layout
//
//         |--------------------------------------|
//         | item count (u32) | (4 bytes padding) |
//         |--------------------------------------|
// range 0 | offset 0 (u32) | count 0 (u32)       | <-- table 0
// range 1 | offset 1       | count 1             |
//         | ...                                  |
//         |--------------------------------------|
//
//         |---------------------------------------|
//         | unified external function idx 0 (u32) | <-- table 1
//         | unified external function idx 1       |
//         | ...                                   |
//         |---------------------------------------|

use crate::{
    entry::ExternalFunctionIndexListEntry,
    module_image::{ModuleSectionId, RangeItem, SectionEntry},
    tableaccess::{load_section_with_two_tables, save_section_with_two_tables},
};

#[derive(Debug, PartialEq, Default)]
pub struct ExternalFunctionIndexSection<'a> {
    pub ranges: &'a [RangeItem],
    pub items: &'a [ExternalFunctionIndexItem],
}

/// the index of this item is `external_function_index`
/// mapping external_function_index -> unified_external_function_index
#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct ExternalFunctionIndexItem {
    pub unified_external_function_index: u32,
}

impl ExternalFunctionIndexItem {
    pub fn new(
        unified_external_function_index: u32,
    ) -> Self {
        Self {
            unified_external_function_index,
        }
    }
}

impl<'a> SectionEntry<'a> for ExternalFunctionIndexSection<'a> {
    fn load(section_data: &'a [u8]) -> Self {
        let (ranges, items) =
            load_section_with_two_tables::<RangeItem, ExternalFunctionIndexItem>(section_data);

        ExternalFunctionIndexSection { ranges, items }
    }

    fn save(&'a self, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        save_section_with_two_tables(self.ranges, self.items, writer)
    }

    fn id(&'a self) -> ModuleSectionId {
        ModuleSectionId::ExternalFunctionIndex
    }
}

impl<'a> ExternalFunctionIndexSection<'a> {
    pub fn get_item_unified_external_function_index(
        &self,
        module_index: usize,
        external_function_index: usize,
    ) -> usize {
        let range = &self.ranges[module_index];

        // bounds check
        #[cfg(feature = "bounds_check")]
        {
            if external_function_index > range.count as usize {
                panic!("Out of bounds of the external function index, module index:{}, total external functions: {}, request external function index: {}",
                    module_index,
                    range.count,
                    external_function_index);
            }
        }

        let item_index = range.offset as usize + external_function_index;
        let item = &self.items[item_index];

        item.unified_external_function_index as usize
    }

    pub fn convert_from_entries(
        sorted_external_function_index_module_entries: &[ExternalFunctionIndexListEntry],
    ) -> (Vec<RangeItem>, Vec<ExternalFunctionIndexItem>) {
        let mut range_start_offset: u32 = 0;
        let range_items = sorted_external_function_index_module_entries
            .iter()
            .map(|index_module_entry| {
                let count = index_module_entry.index_entries.len() as u32;
                let range_item = RangeItem::new(range_start_offset, count);
                range_start_offset += count;
                range_item
            })
            .collect::<Vec<_>>();

        let external_function_index_items = sorted_external_function_index_module_entries
            .iter()
            .flat_map(|index_module_entry| {
                index_module_entry.index_entries.iter().map(|entry| {
                    ExternalFunctionIndexItem::new(
                        // entry.external_function_index as u32,
                        entry.unified_external_function_index as u32,
                        // entry.type_index as u32,
                    )
                })
            })
            .collect::<Vec<_>>();

        (range_items, external_function_index_items)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        index_sections::external_function_index_section::{
            ExternalFunctionIndexItem, ExternalFunctionIndexSection,
        },
        module_image::{RangeItem, SectionEntry},
    };

    #[test]
    fn test_load_section() {
        let section_data = vec![
            2u8, 0, 0, 0, // item count (little endian)
            0, 0, 0, 0, // 4 bytes padding
            //
            0, 0, 0, 0, // offset 0 (item 0)
            2, 0, 0, 0, // count 0
            2, 0, 0, 0, // offset 1 (item 1)
            1, 0, 0, 0, // count 1
            //
            // 0, 0, 0, 0, // external function idx 0, item 0 (little endian)
            3, 0, 0, 0, // uni external function idx 0
            //
            // 1, 0, 0, 0, // external function idx 1, item 1
            5, 0, 0, 0, // uni external function idx 1
            //
            // 0, 0, 0, 0, // external function idx 2, item 2
            7, 0, 0, 0, // uni external function idx 2
        ];

        let section = ExternalFunctionIndexSection::load(&section_data);

        let ranges = section.ranges;

        assert_eq!(ranges.len(), 2);
        assert_eq!(ranges[0], RangeItem::new(0, 2,));
        assert_eq!(ranges[1], RangeItem::new(2, 1,));

        let items = section.items;

        assert_eq!(items.len(), 3);
        assert_eq!(items[0], ExternalFunctionIndexItem::new(3));
        assert_eq!(items[1], ExternalFunctionIndexItem::new(5));
        assert_eq!(items[2], ExternalFunctionIndexItem::new(7));

        // test get index item
        assert_eq!(section.get_item_unified_external_function_index(0, 0), 3);

        assert_eq!(section.get_item_unified_external_function_index(0, 1), 5);

        assert_eq!(section.get_item_unified_external_function_index(1, 0), 7);
    }

    #[test]
    fn test_save_section() {
        let ranges = vec![RangeItem::new(0, 2), RangeItem::new(2, 1)];

        let items = vec![
            ExternalFunctionIndexItem::new(3),
            ExternalFunctionIndexItem::new(5),
            ExternalFunctionIndexItem::new(7),
        ];

        let section = ExternalFunctionIndexSection {
            ranges: &ranges,
            items: &items,
        };

        let mut section_data: Vec<u8> = Vec::new();
        section.save(&mut section_data).unwrap();

        assert_eq!(
            section_data,
            vec![
                2u8, 0, 0, 0, // item count (little endian)
                0, 0, 0, 0, // 4 bytes padding
                //
                0, 0, 0, 0, // offset 0 (item 0)
                2, 0, 0, 0, // count 0
                2, 0, 0, 0, // offset 1 (item 1)
                1, 0, 0, 0, // count 1
                //
                // 0, 0, 0, 0, // external function idx 0, item 0 (little endian)
                3, 0, 0, 0, // uni external function idx 0
                //
                // 1, 0, 0, 0, // external function idx 1, item 1
                5, 0, 0, 0, // uni external function idx 1
                //
                // 0, 0, 0, 0, // external function idx 2, item 2
                7, 0, 0, 0, // uni external function idx 2
            ]
        );
    }

    #[test]
    fn test_convert() {
        // todo
    }
}
