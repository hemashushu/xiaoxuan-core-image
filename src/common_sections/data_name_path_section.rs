// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

//! the data names should follow these order:
//! 1. internal read-only data
//! 2. internal read-write data
//! 3. internal uninit data

// "data name section" binary layout
//
//              |------------------------------------------------------------------------------|
//              | item count (u32) | (4 bytes padding)                                         |
//              |------------------------------------------------------------------------------|
//  item 0 -->  | name path offset 0 (u32) | name path length 0 (u32) | export 0 (u8) | pad 3 bytes | <-- table
//  item 1 -->  | name path offset 1       | name path length 1       | export 1      | pad 3 bytes |
//              | ...                                                                          |
//              |------------------------------------------------------------------------------|
// offset 0 --> | name path string 0 (UTF-8)                                                   | <-- data area
// offset 1 --> | name path string 1                                                           |
//              | ...                                                                          |
//              |------------------------------------------------------------------------------|

use crate::entry::DataNamePathEntry;

use crate::{
    module_image::{ModuleSectionId, SectionEntry},
    tableaccess::{load_section_with_table_and_data_area, save_section_with_table_and_data_area},
};

#[derive(Debug, PartialEq, Default)]
pub struct DataNamePathSection<'a> {
    pub items: &'a [DataNamePathItem],
    pub name_paths_data: &'a [u8],
}

// this table only contains the internal data,
// imported data will not be list in this table.
#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct DataNamePathItem {
    // about the "full_name" and "name_path"
    // -------------------------------------
    // - "full_name" = "module_name::name_path"
    // - "name_path" = "namespace::identifier"
    // - "namespace" = "sub_module_name"{0,N}
    pub name_path_offset: u32,
    pub name_path_length: u32,

    // Used to indicate the visibility of this item when this
    // module is used as a shared module.
    // Note that in the case of static linking, the item is always
    // visible to other modules, regardless of the value of this property.
    //
    // 0=false, 1=true
    pub export: u8,
    _padding0: [u8; 3],
}

impl DataNamePathItem {
    pub fn new(name_path_offset: u32, name_path_length: u32, export: u8) -> Self {
        Self {
            name_path_offset,
            name_path_length,
            export,
            _padding0: [0, 0, 0],
        }
    }
}

impl<'a> SectionEntry<'a> for DataNamePathSection<'a> {
    fn load(section_data: &'a [u8]) -> Self {
        let (items, names_data) =
            load_section_with_table_and_data_area::<DataNamePathItem>(section_data);
        DataNamePathSection {
            items,
            name_paths_data: names_data,
        }
    }

    fn save(&'a self, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        save_section_with_table_and_data_area(self.items, self.name_paths_data, writer)
    }

    fn id(&'a self) -> ModuleSectionId {
        ModuleSectionId::DataNamePath
    }
}

impl<'a> DataNamePathSection<'a> {
    /// the item index is the 'mixed data internal index'
    ///
    /// the data names in the `data_name_section` is order by:
    /// 1. internal read-only data
    /// 2. internal read-write data
    /// 3. internal uninit data
    ///
    /// note that the data public index is mixed the following items:
    /// - imported read-only data items
    /// - imported read-write data items
    /// - imported uninitilized data items
    /// - internal read-only data items
    /// - internal read-write data items
    /// - internal uninitilized data items
    ///
    /// therefore:
    /// data_public_index = (all import datas) + mixed_data_internal_index
    pub fn get_item_index_and_export(&'a self, expected_name_path: &str) -> Option<(usize, bool)> {
        let items = self.items;
        let name_paths_data = self.name_paths_data;

        let expected_name_path_data = expected_name_path.as_bytes();

        let opt_idx = items.iter().position(|item| {
            let name_path_data = &name_paths_data[item.name_path_offset as usize
                ..(item.name_path_offset + item.name_path_length) as usize];
            name_path_data == expected_name_path_data
        });

        opt_idx.map(|idx| {
            let item = &items[idx];
            (idx, item.export != 0)
        })
    }

    /// the item index is the 'mixed data internal index'
    pub fn get_item_name_and_export(&self, data_internal_index: usize) -> (&str, bool) {
        let items = self.items;
        let name_paths_data = self.name_paths_data;

        let item = &items[data_internal_index];
        let name_path_data = &name_paths_data[item.name_path_offset as usize
            ..(item.name_path_offset + item.name_path_length) as usize];
        let name = unsafe { std::str::from_utf8_unchecked(name_path_data) };
        (name, item.export != 0)
    }

    pub fn convert_from_entries(entries: &[DataNamePathEntry]) -> (Vec<DataNamePathItem>, Vec<u8>) {
        let name_path_bytes = entries
            .iter()
            .map(|entry| entry.name_path.as_bytes())
            .collect::<Vec<&[u8]>>();

        let mut next_offset: u32 = 0;

        let items = entries
            .iter()
            .enumerate()
            .map(|(idx, entry)| {
                let name_path_offset = next_offset;
                let name_path_length = name_path_bytes[idx].len() as u32;
                next_offset += name_path_length; // for next offset

                DataNamePathItem::new(
                    name_path_offset,
                    name_path_length,
                    if entry.export { 1 } else { 0 },
                )
            })
            .collect::<Vec<DataNamePathItem>>();

        let name_paths_data = name_path_bytes
            .iter()
            .flat_map(|bytes| bytes.to_vec())
            .collect::<Vec<u8>>();

        (items, name_paths_data)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        common_sections::data_name_path_section::{DataNamePathItem, DataNamePathSection},
        entry::DataNamePathEntry,
        module_image::SectionEntry,
    };

    #[test]
    fn test_save_section() {
        let items: Vec<DataNamePathItem> = vec![
            DataNamePathItem::new(0, 3, 0),
            DataNamePathItem::new(3, 5, 1),
        ];

        let section = DataNamePathSection {
            items: &items,
            name_paths_data: "foohello".as_bytes(),
        };

        let mut section_data: Vec<u8> = Vec::new();
        section.save(&mut section_data).unwrap();

        let mut expect_data = vec![
            2u8, 0, 0, 0, // item count
            0, 0, 0, 0, // 4 bytes padding
            //
            0, 0, 0, 0, // name offset (item 0)
            3, 0, 0, 0, // name length
            0, // export
            0, 0, 0, // padding
            //
            3, 0, 0, 0, // name offset (item 1)
            5, 0, 0, 0, // name length
            1, // export
            0, 0, 0, // padding
        ];

        expect_data.extend_from_slice(b"foo");
        expect_data.extend_from_slice(b"hello");

        assert_eq!(section_data, expect_data);
    }

    #[test]
    fn test_load_section() {
        let mut section_data = vec![
            2u8, 0, 0, 0, // item count
            0, 0, 0, 0, // 4 bytes padding
            //
            0, 0, 0, 0, // name offset (item 0)
            3, 0, 0, 0, // name length
            0, // export
            0, 0, 0, // padding
            //
            3, 0, 0, 0, // name offset (item 1)
            5, 0, 0, 0, // name length
            1, // export
            0, 0, 0, // padding
        ];

        section_data.extend_from_slice("foo".as_bytes());
        section_data.extend_from_slice("hello".as_bytes());

        let section = DataNamePathSection::load(&section_data);

        assert_eq!(section.items.len(), 2);
        assert_eq!(section.items[0], DataNamePathItem::new(0, 3, 0));
        assert_eq!(section.items[1], DataNamePathItem::new(3, 5, 1));
        assert_eq!(section.name_paths_data, "foohello".as_bytes())
    }

    #[test]
    fn test_convert() {
        let entries: Vec<DataNamePathEntry> = vec![
            DataNamePathEntry::new("foo".to_string(), false),
            DataNamePathEntry::new("hello".to_string(), true),
        ];

        let (items, names_data) = DataNamePathSection::convert_from_entries(&entries);
        let section = DataNamePathSection {
            items: &items,
            name_paths_data: &names_data,
        };

        assert_eq!(section.get_item_index_and_export("foo"), Some((0, false)));
        assert_eq!(section.get_item_index_and_export("hello"), Some((1, true)));
        assert_eq!(section.get_item_index_and_export("bar"), None);

        assert_eq!(section.get_item_name_and_export(0), ("foo", false));
        assert_eq!(section.get_item_name_and_export(1), ("hello", true));
    }
}
