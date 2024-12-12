// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

//! this section is used to map the:
//! `(module[current_module_index]).data_load/store(data_public_index)`
//! to
//! `target_module_index` and `data_internal_index`
//!
//! where
//! - `current_module_index` == `index_of_range_item`
//! - `items[range.offset+data_public_index]` is the entry for the `data_public_index`
//! - note that the `data_internal_index` is section relevant, which
//!   means `data_internal_index` in readwrite/readonly/uninit sections are both start with 0.
//!
//! the data public index is mixed the following items (and are sorted by the following order):
//!
//! - imported read-only data items
//! - imported read-write data items
//! - imported uninitilized data items
//! - internal read-only data items
//! - internal read-write data items
//! - internal uninitilized data items

// "data index section" binary layout
//
//         |--------------------------------------|
//         | item count (u32) | (4 bytes padding) |
//         |--------------------------------------|
// range 0 | offset 0 (u32) | count 0 (u32)       | <-- table 0
// range 1 | offset 1       | count 1             |
//         | ...                                  |
//         |--------------------------------------|
//
//         |------------------------------------------------------------------------------------------------------|
//         | target mod idx 0 (u32) | data internal idx 0 (u32) | target data section type 0 (u8) | pad (3 bytes) | <-- table 1
//         | target mod idx 1       | data internal idx 1       | target data section type 1      |               |
//         | ...                                                                                                  |
//         |------------------------------------------------------------------------------------------------------|

use anc_isa::DataSectionType;

use crate::{
    entry::DataIndexListEntry,
    module_image::{ModuleSectionId, RangeItem, SectionEntry},
    tableaccess::{load_section_with_two_tables, save_section_with_two_tables},
};

#[derive(Debug, PartialEq, Default)]
pub struct DataIndexSection<'a> {
    pub ranges: &'a [RangeItem],
    pub items: &'a [DataIndexItem],
}

/// the index of this item is the `data_public_index`
///
/// the data public index is mixed the following items (and are sorted by the following order):
///
/// - imported read-only data items
/// - imported read-write data items
/// - imported uninitilized data items
/// - internal read-only data items
/// - internal read-write data items
/// - internal uninitilized data items
#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct DataIndexItem {
    // pub data_public_index: u32,

    // target module index
    pub target_module_index: u32,

    // the index of the internal data item in a specified data section
    //
    // note that the `data_internal_index` is section relevant, which
    // means `data_internal_index` in readwrite/readonly/uninit sections are both start with 0.
    //
    // e.g.
    // there are indices 0,1,2,3... in read-only section, and
    // there are also indices 0,1,2,3... in read-write section, and
    // there are also indices 0,1,2,3... in uninitialized section.
    pub data_internal_index: u32,

    // u8, target data section, i.e. 0=READ_ONLY, 1=READ_WRITE, 2=UNINIT
    pub target_data_section_type: DataSectionType,

    _padding0: [u8; 3],
}

impl DataIndexItem {
    pub fn new(
        // data_public_index: u32,
        target_module_index: u32,
        data_internal_index: u32,
        target_data_section_type: DataSectionType,
    ) -> Self {
        Self {
            // data_public_index,
            target_module_index,
            data_internal_index,
            target_data_section_type,
            _padding0: [0, 0, 0],
        }
    }
}

impl<'a> SectionEntry<'a> for DataIndexSection<'a> {
    fn load(section_data: &'a [u8]) -> Self {
        let (ranges, items) =
            load_section_with_two_tables::<RangeItem, DataIndexItem>(section_data);
        DataIndexSection { ranges, items }
    }

    fn save(&'a self, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        save_section_with_two_tables(self.ranges, self.items, writer)
    }

    fn id(&'a self) -> ModuleSectionId {
        ModuleSectionId::DataIndex
    }
}

impl DataIndexSection<'_> {
    pub fn get_item_target_module_index_and_data_internal_index_and_data_section_type(
        &self,
        module_index: usize,
        data_public_index: usize,
    ) -> (usize, usize, DataSectionType) {
        let range = &self.ranges[module_index];

        // bounds check
        #[cfg(feature = "bounds_check")]
        {
            if data_public_index > range.count as usize {
                panic!(
                    "Out of bounds of the data public index, module index: {}, total data items: {}, request data index: {}.",
                    module_index, range.count, data_public_index
                );
            }
        }

        let item_index = range.offset as usize + data_public_index;
        let item = &self.items[item_index];
        (
            item.target_module_index as usize,
            item.data_internal_index as usize,
            item.target_data_section_type,
        )
    }

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
                        // entry.data_public_index as u32,
                        entry.target_module_index as u32,
                        entry.data_internal_index as u32,
                        entry.target_data_section_type,
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
        index_sections::data_index_section::{DataIndexItem, DataIndexSection, RangeItem},
        module_image::SectionEntry,
    };

    use super::DataIndexListEntry;

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
            // 2, 0, 0, 0, // data pub index, item 0 (little endian)
            2, 0, 0, 0, // target module index
            3, 0, 0, 0, // data internal idx
            0, // target data section type
            0, 0, 0, // padding
            //
            // 7, 0, 0, 0, // data pub index, item 1 (little endian)
            5, 0, 0, 0, // target module index
            7, 0, 0, 0, // data internal idx
            1, // target data section type
            0, 0, 0, // padding
            //
            // 17, 0, 0, 0, // data pub index, item 2 (little endian)
            11, 0, 0, 0, // target module index
            13, 0, 0, 0, // data internal idx
            2, // target data section type
            0, 0, 0, // padding
        ];

        let section = DataIndexSection::load(&section_data);

        let ranges = section.ranges;

        assert_eq!(ranges.len(), 2);
        assert_eq!(ranges[0], RangeItem::new(0, 2,));
        assert_eq!(ranges[1], RangeItem::new(2, 1));

        let items = section.items;

        assert_eq!(items.len(), 3);

        assert_eq!(
            items[0],
            DataIndexItem::new(2, 3, DataSectionType::ReadOnly,)
        );

        assert_eq!(
            items[1],
            DataIndexItem::new(5, 7, DataSectionType::ReadWrite,)
        );

        assert_eq!(
            items[2],
            DataIndexItem::new(11, 13, DataSectionType::Uninit,)
        );

        // test get index item
        assert_eq!(
            section
                .get_item_target_module_index_and_data_internal_index_and_data_section_type(0, 0),
            (2, 3, DataSectionType::ReadOnly)
        );

        assert_eq!(
            section
                .get_item_target_module_index_and_data_internal_index_and_data_section_type(0, 1),
            (5, 7, DataSectionType::ReadWrite,)
        );

        assert_eq!(
            section
                .get_item_target_module_index_and_data_internal_index_and_data_section_type(1, 0),
            (11, 13, DataSectionType::Uninit)
        );
    }

    #[test]
    fn test_save_section() {
        let ranges = vec![RangeItem::new(0, 2), RangeItem::new(2, 1)];

        let items = vec![
            DataIndexItem::new(2, 3, DataSectionType::ReadOnly),
            DataIndexItem::new(5, 7, DataSectionType::ReadWrite),
            DataIndexItem::new(11, 13, DataSectionType::Uninit),
        ];

        let section = DataIndexSection {
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
                // 2, 0, 0, 0, // data pub index, item 0 (little endian)
                2, 0, 0, 0, // t module index
                3, 0, 0, 0, // data internal idx
                0, // t data section type
                0, 0, 0, // padding
                //
                // 7, 0, 0, 0, // data pub index, item 1 (little endian)
                5, 0, 0, 0, // t module index
                7, 0, 0, 0, // datainternal  idx
                1, // t data section type
                0, 0, 0, // padding
                //
                // 17, 0, 0, 0, // data pub index, item 2 (little endian)
                11, 0, 0, 0, // t module index
                13, 0, 0, 0, // data internal idx
                2, // t data section type
                0, 0, 0, // padding
            ]
        );
    }

    #[test]
    fn test_convert() {
        let entries = vec![
            DataIndexListEntry::new(vec![
                DataIndexEntry::new(2, 3, DataSectionType::ReadOnly),
                DataIndexEntry::new(5, 7, DataSectionType::ReadWrite),
                DataIndexEntry::new(11, 13, DataSectionType::Uninit),
            ]),
            DataIndexListEntry::new(vec![
                DataIndexEntry::new(17, 19, DataSectionType::ReadWrite),
                DataIndexEntry::new(23, 29, DataSectionType::ReadWrite),
            ]),
        ];

        let (ranges, items) = DataIndexSection::convert_from_entries(&entries);

        let section = DataIndexSection {
            ranges: &ranges,
            items: &items,
        };

        assert_eq!(
            section
                .get_item_target_module_index_and_data_internal_index_and_data_section_type(0, 0),
            (2, 3, DataSectionType::ReadOnly)
        );

        assert_eq!(
            section
                .get_item_target_module_index_and_data_internal_index_and_data_section_type(0, 1),
            (5, 7, DataSectionType::ReadWrite)
        );

        assert_eq!(
            section
                .get_item_target_module_index_and_data_internal_index_and_data_section_type(0, 2),
            (11, 13, DataSectionType::Uninit)
        );

        assert_eq!(
            section
                .get_item_target_module_index_and_data_internal_index_and_data_section_type(1, 0),
            (17, 19, DataSectionType::ReadWrite)
        );

        assert_eq!(
            section
                .get_item_target_module_index_and_data_internal_index_and_data_section_type(1, 1),
            (23, 29, DataSectionType::ReadWrite)
        );
    }
}
