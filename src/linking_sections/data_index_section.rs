// Copyright (c) 2025 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions.
// For more details, see the LICENSE, LICENSE.additional, and CONTRIBUTING files.

// "Data Index Section" binary layout:
//
//         |----------------------------------------------|
//         | item count (u32) | extra header length (u32) |
//         |----------------------------------------------|
// range 0 | offset 0 (u32) | count 0 (u32)               | <-- table 0
// range 1 | offset 1       | count 1                     |
//         | ...                                          |
//         |----------------------------------------------|
//
//                  |-------------------------------------------------|
//         / item 0 | target module idx 0 (u32)                       |
//         |        | target data section type 0 (u8) | pad (3 bytes) | <-- table 1
//         |        | data internal idx in section 0 (u32)            |
// range 0 | item 1 | target module idx 1                             |
//         |        | target data section type 1      | pad           |
//         |        | data internal idx in section 1                  |
//         \ ...    | ...                                             |
//                  |-------------------------------------------------|
//         / item 0 | ...                                             |
// range 1 | item 1 | ...                                             |
//         \ ...    | ...                                             |
//                  |-------------------------------------------------|

// This section represents a mapping table that associates
// `(module_index, data_public_index)` with
// `(target_module_index, target_data_section_type, data_internal_index_in_section)`.

use anc_isa::DataSectionType;

use crate::{
    datatableaccess::{read_section_with_two_tables, write_section_with_two_tables},
    entry::{DataIndexEntry, DataIndexListEntry},
    module_image::{ModuleSectionId, RangeItem, SectionEntry},
};

/// The index of this item in a specific range is the `data_public_index`.
#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct DataIndexItem {
    // Target module index.
    pub target_module_index: u32,

    // Target data section type (e.g., 0=READ_ONLY, 1=READ_WRITE, 2=UNINIT).
    pub target_data_section_type: DataSectionType,

    // Padding to align the structure.
    _padding0: [u8; 3],

    // Index of the data item within the specified data section in the module.
    // Note: The `data_internal_index` is section-specific, meaning indices
    // in read-only, read-write, and uninitialized sections all start from 0.
    pub data_internal_index_in_section: u32,
}

impl DataIndexItem {
    /// Creates a new `DataIndexItem`.
    pub fn new(
        target_module_index: u32,
        target_data_section_type: DataSectionType,
        data_internal_index_in_section: u32,
    ) -> Self {
        Self {
            target_module_index,
            target_data_section_type,
            _padding0: [0, 0, 0],
            data_internal_index_in_section,
        }
    }
}

/// The index of range is the current `module_index`.
#[derive(Debug, PartialEq, Default)]
pub struct DataIndexSection<'a> {
    pub ranges: &'a [RangeItem],    // Array of range items.
    pub items: &'a [DataIndexItem], // Array of data index items.
}

impl<'a> SectionEntry<'a> for DataIndexSection<'a> {
    fn read(section_data: &'a [u8]) -> Self {
        let (ranges, items) =
            read_section_with_two_tables::<RangeItem, DataIndexItem>(section_data);
        DataIndexSection { ranges, items }
    }

    fn write(&'a self, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        write_section_with_two_tables(self.ranges, self.items, writer)
    }

    fn id(&'a self) -> ModuleSectionId {
        ModuleSectionId::DataIndex
    }
}

impl DataIndexSection<'_> {
    /// Returns the number of items in a specific range (module index).
    pub fn get_items_count(&self, module_index: usize) -> usize {
        let range = &self.ranges[module_index];
        range.count as usize
    }

    /// Retrieves the target module index, data section type, and internal index
    /// for a specific item in a range.
    pub fn get_item_target_module_index_and_data_section_type_and_data_internal_index_in_section(
        &self,
        module_index: usize,
        data_public_index: usize,
    ) -> (
        usize, // Target module index.
        DataSectionType,
        usize, // Internal index in the section.
    ) {
        let range = &self.ranges[module_index];
        let item_index = range.offset as usize + data_public_index;
        let item = &self.items[item_index];
        (
            item.target_module_index as usize,
            item.target_data_section_type,
            item.data_internal_index_in_section as usize,
        )
    }

    /// Converts the section into a list of entries.
    pub fn convert_to_entries(&self) -> Vec<DataIndexListEntry> {
        self.ranges
            .iter()
            .map(|range| {
                let index_entries = (0..(range.count as usize))
                    .map(|item_index| {
                        let item = &self.items[range.offset as usize + item_index];
                        DataIndexEntry::new(
                            item.target_module_index as usize,
                            item.target_data_section_type,
                            item.data_internal_index_in_section as usize,
                        )
                    })
                    .collect::<Vec<_>>();
                DataIndexListEntry::new(index_entries)
            })
            .collect::<Vec<_>>()
    }

    /// Converts a list of entries into ranges and items.
    pub fn convert_from_entries(
        sorted_module_entries: &[DataIndexListEntry],
    ) -> (Vec<RangeItem>, Vec<DataIndexItem>) {
        let mut range_start_offset: u32 = 0;

        let range_items = sorted_module_entries
            .iter()
            .map(|index_module_entry| {
                let count = index_module_entry.index_entries.len() as u32;
                let range_item = RangeItem::new(range_start_offset, count);
                range_start_offset += count;
                range_item
            })
            .collect::<Vec<_>>();

        let data_index_items = sorted_module_entries
            .iter()
            .flat_map(|index_module_entry| {
                index_module_entry.index_entries.iter().map(|entry| {
                    DataIndexItem::new(
                        entry.target_module_index as u32,
                        entry.target_data_section_type,
                        entry.data_internal_index_in_section as u32,
                    )
                })
            })
            .collect::<Vec<_>>();

        (range_items, data_index_items)
    }
}

#[cfg(test)]
mod tests {
    use anc_isa::DataSectionType;

    use crate::{
        entry::DataIndexEntry,
        linking_sections::data_index_section::{DataIndexItem, DataIndexSection, RangeItem},
        module_image::SectionEntry,
    };

    use super::DataIndexListEntry;

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
            2, 0, 0, 0, // target module index
            0, // target data section type
            0, 0, 0, // padding
            3, 0, 0, 0, // data internal idx in section
            //
            5, 0, 0, 0, // target module index
            1, // target data section type
            0, 0, 0, // padding
            7, 0, 0, 0, // data internal idx in section
            //
            11, 0, 0, 0, // target module index
            2, // target data section type
            0, 0, 0, // padding
            13, 0, 0, 0, // data internal idx in section
        ];

        let section = DataIndexSection::read(&section_data);

        let ranges = section.ranges;

        assert_eq!(ranges.len(), 2);
        assert_eq!(ranges[0], RangeItem::new(0, 2,));
        assert_eq!(ranges[1], RangeItem::new(2, 1));

        let items = section.items;

        assert_eq!(items.len(), 3);

        assert_eq!(
            items[0],
            DataIndexItem::new(2, DataSectionType::ReadOnly, 3)
        );

        assert_eq!(
            items[1],
            DataIndexItem::new(5, DataSectionType::ReadWrite, 7)
        );

        assert_eq!(
            items[2],
            DataIndexItem::new(11, DataSectionType::Uninit, 13)
        );

        // test get index item
        assert_eq!(
            section
                .get_item_target_module_index_and_data_section_type_and_data_internal_index_in_section(0, 0),
            (2, DataSectionType::ReadOnly, 3, )
        );

        assert_eq!(
            section
                .get_item_target_module_index_and_data_section_type_and_data_internal_index_in_section(0, 1),
            (5, DataSectionType::ReadWrite,7, )
        );

        assert_eq!(
            section
                .get_item_target_module_index_and_data_section_type_and_data_internal_index_in_section(1, 0),
            (11,DataSectionType::Uninit ,13, )
        );
    }

    #[test]
    fn test_write_section() {
        let ranges = vec![RangeItem::new(0, 2), RangeItem::new(2, 1)];

        let items = vec![
            DataIndexItem::new(2, DataSectionType::ReadOnly, 3),
            DataIndexItem::new(5, DataSectionType::ReadWrite, 7),
            DataIndexItem::new(11, DataSectionType::Uninit, 13),
        ];

        let section = DataIndexSection {
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
                2, 0, 0, 0, // target module index
                0, // target data section type
                0, 0, 0, // padding
                3, 0, 0, 0, // data internal idx in section
                //
                5, 0, 0, 0, // target module index
                1, // target data section type
                0, 0, 0, // padding
                7, 0, 0, 0, // data internal  idx in section
                //
                11, 0, 0, 0, // target module index
                2, // target data section type
                0, 0, 0, // padding
                13, 0, 0, 0, // data internal idx in section
            ]
        );
    }

    #[test]
    fn test_convert() {
        let entries = vec![
            DataIndexListEntry::new(vec![
                DataIndexEntry::new(2, DataSectionType::ReadOnly, 3),
                DataIndexEntry::new(5, DataSectionType::ReadWrite, 7),
                DataIndexEntry::new(11, DataSectionType::Uninit, 13),
            ]),
            DataIndexListEntry::new(vec![
                DataIndexEntry::new(17, DataSectionType::ReadWrite, 19),
                DataIndexEntry::new(23, DataSectionType::ReadWrite, 29),
            ]),
        ];

        let (ranges, items) = DataIndexSection::convert_from_entries(&entries);

        let section = DataIndexSection {
            ranges: &ranges,
            items: &items,
        };

        assert_eq!(
            section
                .get_item_target_module_index_and_data_section_type_and_data_internal_index_in_section(0, 0),
            (2,  DataSectionType::ReadOnly, 3,)
        );

        assert_eq!(
            section
                .get_item_target_module_index_and_data_section_type_and_data_internal_index_in_section(0, 1),
            (5,  DataSectionType::ReadWrite, 7,)
        );

        assert_eq!(
            section
                .get_item_target_module_index_and_data_section_type_and_data_internal_index_in_section(0, 2),
            (11,  DataSectionType::Uninit, 13,)
        );

        assert_eq!(
            section
                .get_item_target_module_index_and_data_section_type_and_data_internal_index_in_section(1, 0),
            (17, DataSectionType::ReadWrite, 19, )
        );

        assert_eq!(
            section
                .get_item_target_module_index_and_data_section_type_and_data_internal_index_in_section(1, 1),
            (23, DataSectionType::ReadWrite, 29, )
        );

        let entries_restore = section.convert_to_entries();
        assert_eq!(entries_restore, entries);
    }
}
