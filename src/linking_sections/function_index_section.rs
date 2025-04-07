// Copyright (c) 2025 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions.
// For more details, see the LICENSE, LICENSE.additional, and CONTRIBUTING files.

// "Function Index Section" binary layout:
//
//         |----------------------------------------------|
//         | item count (u32) | extra header length (u32) |
//         |----------------------------------------------|
// range 0 | offset 0 (u32) | count 0 (u32)               | <-- table 0
// range 1 | offset 1       | count 1                     |
//         | ...                                          |
//         |----------------------------------------------|
//
//           |--------------------------------------------------------|
//         / | target mod idx 0 (u32) | function internal idx 0 (u32) | <-- table 1
// range 0 | | target mod idx 1       | function internal idx 1       |
//         \ | ...                                                    |
//           |--------------------------------------------------------|
//         / | ...                                                    |
// range 1 | | ...                                                    |
//         \ | ...                                                    |
//           |--------------------------------------------------------|

use crate::{
    datatableaccess::{read_section_with_two_tables, write_section_with_two_tables},
    entry::{FunctionIndexEntry, FunctionIndexListEntry},
    module_image::{ModuleSectionId, RangeItem, SectionEntry},
};

/// The index for this item in a specific range is the `function_public_index`.
#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct FunctionIndexItem {
    // Target module index.
    pub target_module_index: u32,

    // Index of the internal function in the module.
    pub function_internal_index: u32,
}

impl FunctionIndexItem {
    /// Creates a new `FunctionIndexItem`.
    pub fn new(target_module_index: u32, function_internal_index: u32) -> Self {
        Self {
            target_module_index,
            function_internal_index,
        }
    }
}

/// The index of range is the current `module_index`.
#[derive(Debug, PartialEq)]
pub struct FunctionIndexSection<'a> {
    pub ranges: &'a [RangeItem],        // Array of range items.
    pub items: &'a [FunctionIndexItem], // Array of function index items.
}

impl<'a> SectionEntry<'a> for FunctionIndexSection<'a> {
    fn read(section_data: &'a [u8]) -> Self {
        let (ranges, items) =
            read_section_with_two_tables::<RangeItem, FunctionIndexItem>(section_data);

        FunctionIndexSection { ranges, items }
    }

    fn write(&'a self, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        write_section_with_two_tables(self.ranges, self.items, writer)
    }

    fn id(&'a self) -> ModuleSectionId {
        ModuleSectionId::FunctionIndex
    }
}

impl FunctionIndexSection<'_> {
    /// Returns the number of items in a specific range (module index).
    pub fn get_items_count(&self, module_index: usize) -> usize {
        let range = &self.ranges[module_index];
        range.count as usize
    }

    /// Retrieves the target module index and internal function index
    /// for a specific item in a range.
    pub fn get_item_target_module_index_and_function_internal_index(
        &self,
        module_index: usize,
        function_public_index: usize,
    ) -> (usize, usize) {
        let range = &self.ranges[module_index];

        let item_index = range.offset as usize + function_public_index;
        let item = &self.items[item_index];
        (
            item.target_module_index as usize,
            item.function_internal_index as usize,
        )
    }

    /// Converts the section into a list of entries.
    pub fn convert_to_entries(&self) -> Vec<FunctionIndexListEntry> {
        self.ranges
            .iter()
            .map(|range| {
                let index_entries = (0..(range.count as usize))
                    .map(|item_index| {
                        let item = &self.items[range.offset as usize + item_index];
                        FunctionIndexEntry::new(
                            item.target_module_index as usize,
                            item.function_internal_index as usize,
                        )
                    })
                    .collect::<Vec<_>>();
                FunctionIndexListEntry::new(index_entries)
            })
            .collect::<Vec<_>>()
    }

    /// Converts a list of entries into ranges and items.
    pub fn convert_from_entries(
        sorted_entries: &[FunctionIndexListEntry],
    ) -> (Vec<RangeItem>, Vec<FunctionIndexItem>) {
        let mut range_start_offset: u32 = 0;
        let range_items = sorted_entries
            .iter()
            .map(|index_module_entry| {
                let count = index_module_entry.index_entries.len() as u32;
                let range_item = RangeItem::new(range_start_offset, count);
                range_start_offset += count;
                range_item
            })
            .collect::<Vec<_>>();

        let function_index_items = sorted_entries
            .iter()
            .flat_map(|index_module_entry| {
                index_module_entry.index_entries.iter().map(|entry| {
                    FunctionIndexItem::new(
                        entry.target_module_index as u32,
                        entry.function_internal_index as u32,
                    )
                })
            })
            .collect::<Vec<_>>();

        (range_items, function_index_items)
    }
}

#[cfg(test)]
mod tests {

    use crate::{
        entry::FunctionIndexEntry,
        linking_sections::function_index_section::{FunctionIndexItem, FunctionIndexSection},
        module_image::{RangeItem, SectionEntry},
    };

    use super::FunctionIndexListEntry;

    #[test]
    fn test_read_section() {
        let section_data = vec![
            2u8, 0, 0, 0, // item count (little endian)
            0, 0, 0, 0, // extra section header len (i32)
            //
            0, 0, 0, 0, // offset 0 (item 0)
            2, 0, 0, 0, // count 0
            2, 0, 0, 0, // offset 1 (item 1)
            1, 0, 0, 0, // count 1
            //
            2, 0, 0, 0, // target module idx 0
            3, 0, 0, 0, // function internal idx 0
            //
            5, 0, 0, 0, // target module idx 1
            7, 0, 0, 0, // function internal idx 1
            //
            11, 0, 0, 0, // target module idx 2
            13, 0, 0, 0, // function internal idx 2
        ];

        let section = FunctionIndexSection::read(&section_data);

        let ranges = section.ranges;

        assert_eq!(ranges.len(), 2);
        assert_eq!(ranges[0], RangeItem::new(0, 2,));
        assert_eq!(ranges[1], RangeItem::new(2, 1,));

        let items = section.items;

        assert_eq!(items.len(), 3);
        assert_eq!(items[0], FunctionIndexItem::new(2, 3,));
        assert_eq!(items[1], FunctionIndexItem::new(5, 7));
        assert_eq!(items[2], FunctionIndexItem::new(11, 13));

        // test get index item
        assert_eq!(
            section.get_item_target_module_index_and_function_internal_index(0, 0),
            (2, 3,)
        );

        assert_eq!(
            section.get_item_target_module_index_and_function_internal_index(0, 1),
            (5, 7,)
        );

        assert_eq!(
            section.get_item_target_module_index_and_function_internal_index(1, 0),
            (11, 13,)
        );
    }

    #[test]
    fn test_write_section() {
        let ranges = vec![RangeItem::new(0, 2), RangeItem::new(2, 1)];

        let items = vec![
            FunctionIndexItem::new(2, 3),
            FunctionIndexItem::new(5, 7),
            FunctionIndexItem::new(11, 13),
        ];

        let section = FunctionIndexSection {
            ranges: &ranges,
            items: &items,
        };

        let mut section_data: Vec<u8> = vec![];
        section.write(&mut section_data).unwrap();

        assert_eq!(
            section_data,
            vec![
                2u8, 0, 0, 0, // item count (little endian)
                0, 0, 0, 0, // extra section header len (i32)
                //
                0, 0, 0, 0, // offset 0 (item 0)
                2, 0, 0, 0, // count 0
                2, 0, 0, 0, // offset 1 (item 1)
                1, 0, 0, 0, // count 1
                //
                2, 0, 0, 0, // target module idx 0
                3, 0, 0, 0, // function internal idx 0
                //
                5, 0, 0, 0, // target module idx 1
                7, 0, 0, 0, // function internal idx 1
                //
                11, 0, 0, 0, // target module idx 2
                13, 0, 0, 0, // function internal idx 2
            ]
        );
    }

    #[test]
    fn test_convert() {
        let entries = vec![
            FunctionIndexListEntry::new(vec![
                FunctionIndexEntry::new(2, 3),
                FunctionIndexEntry::new(5, 7),
            ]),
            FunctionIndexListEntry::new(vec![
                FunctionIndexEntry::new(11, 13),
                FunctionIndexEntry::new(17, 19),
                FunctionIndexEntry::new(23, 29),
            ]),
        ];

        let (ranges, items) = FunctionIndexSection::convert_from_entries(&entries);

        let section = FunctionIndexSection {
            ranges: &ranges,
            items: &items,
        };

        assert_eq!(
            section.get_item_target_module_index_and_function_internal_index(0, 0),
            (2, 3)
        );

        assert_eq!(
            section.get_item_target_module_index_and_function_internal_index(0, 1),
            (5, 7)
        );

        assert_eq!(
            section.get_item_target_module_index_and_function_internal_index(1, 0),
            (11, 13)
        );

        assert_eq!(
            section.get_item_target_module_index_and_function_internal_index(1, 1),
            (17, 19)
        );

        assert_eq!(
            section.get_item_target_module_index_and_function_internal_index(1, 2),
            (23, 29)
        );

        let entries_restore = section.convert_to_entries();
        assert_eq!(entries_restore, entries);
    }
}
