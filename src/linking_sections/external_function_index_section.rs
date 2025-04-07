// Copyright (c) 2025 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions.
// For more details, see the LICENSE, LICENSE.additional, and CONTRIBUTING files.

// "External Function Index Section" binary layout:
//
//         |----------------------------------------------|
//         | item count (u32) | extra header length (u32) |
//         |----------------------------------------------|
// range 0 | offset 0 (u32) | count 0 (u32)               | <-- table 0
// range 1 | offset 1       | count 1                     |
//         | ...                                          |
//         |----------------------------------------------|
//
//           |---------------------------------------|
//         / | unified external function idx 0 (u32) | <-- table 1
// range 0 | | unified external function idx 1       |
//         \ | ...                                   |
//           |---------------------------------------|
//         / | ...                                   |
// range 1 | | ...                                   |
//         \ | ...                                   |
//           |---------------------------------------|

use crate::{
    datatableaccess::{read_section_with_two_tables, write_section_with_two_tables},
    entry::{ExternalFunctionIndexEntry, ExternalFunctionIndexListEntry},
    module_image::{ModuleSectionId, RangeItem, SectionEntry},
};

/// The index of this item in a specific range is `external_function_index`.
#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct ExternalFunctionIndexItem {
    pub unified_external_function_index: u32,
}

impl ExternalFunctionIndexItem {
    /// Creates a new `ExternalFunctionIndexItem` with the given unified index.
    pub fn new(unified_external_function_index: u32) -> Self {
        Self {
            unified_external_function_index,
        }
    }
}

/// The index of range is the current `module_index`.
#[derive(Debug, PartialEq, Default)]
pub struct ExternalFunctionIndexSection<'a> {
    pub ranges: &'a [RangeItem],
    pub items: &'a [ExternalFunctionIndexItem],
}

impl<'a> SectionEntry<'a> for ExternalFunctionIndexSection<'a> {
    /// Reads the section data and parses it into ranges and items.
    fn read(section_data: &'a [u8]) -> Self {
        let (ranges, items) =
            read_section_with_two_tables::<RangeItem, ExternalFunctionIndexItem>(section_data);

        ExternalFunctionIndexSection { ranges, items }
    }

    /// Writes the ranges and items into the section data format.
    fn write(&'a self, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        write_section_with_two_tables(self.ranges, self.items, writer)
    }

    /// Returns the section ID for the external function index section.
    fn id(&'a self) -> ModuleSectionId {
        ModuleSectionId::ExternalFunctionIndex
    }
}

impl ExternalFunctionIndexSection<'_> {
    /// Returns the number of items in a specific range identified by `module_index`.
    pub fn get_items_count(&self, module_index: usize) -> usize {
        let range = &self.ranges[module_index];
        range.count as usize
    }

    /// Retrieves the unified external function index for a specific item in a range.
    pub fn get_item_unified_external_function_index(
        &self,
        module_index: usize,
        external_function_index: usize,
    ) -> usize {
        let range = &self.ranges[module_index];
        let item_index = range.offset as usize + external_function_index;
        let item = &self.items[item_index];
        item.unified_external_function_index as usize
    }

    /// Converts the section data into a list of entries for external function indices.
    pub fn convert_to_entries(&self) -> Vec<ExternalFunctionIndexListEntry> {
        self.ranges
            .iter()
            .map(|range| {
                let index_entries = (0..(range.count as usize))
                    .map(|item_index| {
                        let item = &self.items[range.offset as usize + item_index];
                        ExternalFunctionIndexEntry::new(
                            item.unified_external_function_index as usize,
                        )
                    })
                    .collect::<Vec<_>>();
                ExternalFunctionIndexListEntry::new(index_entries)
            })
            .collect::<Vec<_>>()
    }

    /// Converts a list of entries into ranges and items for the section.
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
                    ExternalFunctionIndexItem::new(entry.unified_external_function_index as u32)
                })
            })
            .collect::<Vec<_>>();

        (range_items, external_function_index_items)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        entry::{ExternalFunctionIndexEntry, ExternalFunctionIndexListEntry},
        linking_sections::external_function_index_section::{
            ExternalFunctionIndexItem, ExternalFunctionIndexSection,
        },
        module_image::{RangeItem, SectionEntry},
    };

    #[test]
    fn test_read_section() {
        let section_data = vec![
            2u8, 0, 0, 0, // item count (little endian)
            0, 0, 0, 0, // extra section header length (u32)
            //
            0, 0, 0, 0, // offset 0 (range 0)
            2, 0, 0, 0, // count 0
            2, 0, 0, 0, // offset 1 (range 1)
            1, 0, 0, 0, // count 1
            //
            3, 0, 0, 0, // unified external function idx 0
            5, 0, 0, 0, // unified external function idx 1
            7, 0, 0, 0, // unified external function idx 2
        ];

        let section = ExternalFunctionIndexSection::read(&section_data);

        let ranges = section.ranges;

        assert_eq!(ranges.len(), 2);
        assert_eq!(ranges[0], RangeItem::new(0, 2));
        assert_eq!(ranges[1], RangeItem::new(2, 1));

        let items = section.items;

        assert_eq!(items.len(), 3);
        assert_eq!(items[0], ExternalFunctionIndexItem::new(3));
        assert_eq!(items[1], ExternalFunctionIndexItem::new(5));
        assert_eq!(items[2], ExternalFunctionIndexItem::new(7));

        // Test retrieving unified external function indices
        assert_eq!(section.get_item_unified_external_function_index(0, 0), 3);
        assert_eq!(section.get_item_unified_external_function_index(0, 1), 5);
        assert_eq!(section.get_item_unified_external_function_index(1, 0), 7);
    }

    #[test]
    fn test_write_section() {
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

        let mut section_data: Vec<u8> = vec![];
        section.write(&mut section_data).unwrap();

        assert_eq!(
            section_data,
            vec![
                2u8, 0, 0, 0, // item count (little endian)
                0, 0, 0, 0, // extra section header length (u32)
                //
                0, 0, 0, 0, // offset 0 (range 0)
                2, 0, 0, 0, // count 0
                2, 0, 0, 0, // offset 1 (range 1)
                1, 0, 0, 0, // count 1
                //
                3, 0, 0, 0, // unified external function idx 0
                5, 0, 0, 0, // unified external function idx 1
                7, 0, 0, 0, // unified external function idx 2
            ]
        );
    }

    #[test]
    fn test_convert() {
        let entries = vec![
            ExternalFunctionIndexListEntry::new(vec![
                ExternalFunctionIndexEntry::new(11),
                ExternalFunctionIndexEntry::new(13),
                ExternalFunctionIndexEntry::new(17),
            ]),
            ExternalFunctionIndexListEntry::new(vec![
                ExternalFunctionIndexEntry::new(23),
                ExternalFunctionIndexEntry::new(29),
            ]),
        ];

        let (ranges, items) = ExternalFunctionIndexSection::convert_from_entries(&entries);

        let section = ExternalFunctionIndexSection {
            ranges: &ranges,
            items: &items,
        };

        assert_eq!(section.get_item_unified_external_function_index(0, 0), 11);
        assert_eq!(section.get_item_unified_external_function_index(0, 1), 13);
        assert_eq!(section.get_item_unified_external_function_index(0, 2), 17);

        assert_eq!(section.get_item_unified_external_function_index(1, 0), 23);
        assert_eq!(section.get_item_unified_external_function_index(1, 1), 29);

        let entries_restore = section.convert_to_entries();
        assert_eq!(entries_restore, entries);
    }
}
