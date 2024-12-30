// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

//! this section list all internal data.
//! the data names should follow these order:
//! 1. internal read-only data
//! 2. internal read-write data
//! 3. internal uninit data

// "data name section" binary layout
//
//              |--------------------------------------------------------------------------------------------------|
//              | item count (u32) | (4 bytes padding)                                                             |
//              |--------------------------------------------------------------------------------------------------|
//  item 0 -->  | full name offset 0 (u32) | full name length 0 (u32) | vis 0 (u8) | sec type 0 (u8) | pad 2 bytes | <-- table
//  item 1 -->  | full name offset 1       | full name length 1       | vis 1      | sec type 1      | pad 2 bytes |
//              | ...                                                                                              |
//              |--------------------------------------------------------------------------------------------------|
// offset 0 --> | full name string 0 (UTF-8)                                                                       | <-- data area
// offset 1 --> | full name string 1                                                                               |
//              | ...                                                                                              |
//              |--------------------------------------------------------------------------------------------------|

use anc_isa::DataSectionType;

use crate::entry::ExportDataEntry;

use crate::module_image::Visibility;
use crate::{
    module_image::{ModuleSectionId, SectionEntry},
    tableaccess::{read_section_with_table_and_data_area, write_section_with_table_and_data_area},
};

#[derive(Debug, PartialEq, Default)]
pub struct ExportDataSection<'a> {
    pub items: &'a [ExportDataItem],
    pub full_names_data: &'a [u8],
}

// this table only contains the internal data,
// imported data will not be list in this table.
#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct ExportDataItem {
    // about the "full_name" and "name_path"
    // -------------------------------------
    // - "full_name" = "module_name::name_path"
    // - "name_path" = "namespace::identifier"
    // - "namespace" = "sub_module_name"{0,N}
    //
    // e.g.
    // the name path of function "add" in submodule "myapp:utils" is "utils::add",
    // and the full name is "myapp::utils::add"
    pub full_name_offset: u32,
    pub full_name_length: u32,

    pub visibility: Visibility,
    pub section_type: DataSectionType,
    _padding0: [u8; 2],
}

impl ExportDataItem {
    pub fn new(
        full_name_offset: u32,
        full_name_length: u32,
        visibility: Visibility,
        section_type: DataSectionType,
    ) -> Self {
        Self {
            full_name_offset,
            full_name_length,
            visibility,
            section_type,
            _padding0: [0, 0],
        }
    }
}

impl<'a> SectionEntry<'a> for ExportDataSection<'a> {
    fn read(section_data: &'a [u8]) -> Self {
        let (items, full_names_data) =
            read_section_with_table_and_data_area::<ExportDataItem>(section_data);
        ExportDataSection {
            items,
            full_names_data,
        }
    }

    fn write(&'a self, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        write_section_with_table_and_data_area(self.items, self.full_names_data, writer)
    }

    fn id(&'a self) -> ModuleSectionId {
        ModuleSectionId::ExportData
    }
}

impl<'a> ExportDataSection<'a> {
    /// the item index is the 'mixed data internal index'
    ///
    /// the data items in the `export_data_section` are ordered by:
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
    pub fn get_item_index_and_visibility_and_section_type(
        &'a self,
        expected_full_name: &str,
    ) -> Option<(usize, Visibility, DataSectionType)> {
        let items = self.items;
        let full_name_data = self.full_names_data;

        let expected_full_name_data = expected_full_name.as_bytes();

        let opt_idx = items.iter().position(|item| {
            let full_name_data = &full_name_data[item.full_name_offset as usize
                ..(item.full_name_offset + item.full_name_length) as usize];
            full_name_data == expected_full_name_data
        });

        opt_idx.map(|idx| {
            let item = &items[idx];
            (idx, item.visibility, item.section_type)
        })
    }

    /// the item index is the 'mixed data internal index'
    pub fn get_item_full_name_and_visibility_and_section_type(
        &self,
        data_internal_index: usize,
    ) -> (&str, Visibility, DataSectionType) {
        let items = self.items;
        let full_names_data = self.full_names_data;

        let item = &items[data_internal_index];
        let full_name_data = &full_names_data[item.full_name_offset as usize
            ..(item.full_name_offset + item.full_name_length) as usize];
        let full_name = std::str::from_utf8(full_name_data).unwrap();
        (full_name, item.visibility, item.section_type)
    }

    pub fn convert_to_entries(&self) -> Vec<ExportDataEntry> {
        let items = self.items;
        let full_names_data = self.full_names_data;
        items
            .iter()
            .map(|item| {
                let full_name_data = &full_names_data[item.full_name_offset as usize
                    ..(item.full_name_offset + item.full_name_length) as usize];
                let full_name = std::str::from_utf8(full_name_data).unwrap();
                ExportDataEntry::new(full_name.to_owned(), item.visibility, item.section_type)
            })
            .collect()
    }

    pub fn convert_from_entries(entries: &[ExportDataEntry]) -> (Vec<ExportDataItem>, Vec<u8>) {
        let full_name_bytes = entries
            .iter()
            .map(|entry| entry.full_name.as_bytes())
            .collect::<Vec<&[u8]>>();

        let mut next_offset: u32 = 0;

        let items = entries
            .iter()
            .enumerate()
            .map(|(idx, entry)| {
                let full_name_offset = next_offset;
                let full_name_length = full_name_bytes[idx].len() as u32;
                next_offset += full_name_length; // for next offset

                ExportDataItem::new(
                    full_name_offset,
                    full_name_length,
                    entry.visibility,
                    entry.section_type,
                )
            })
            .collect::<Vec<ExportDataItem>>();

        let full_names_data = full_name_bytes
            .iter()
            .flat_map(|bytes| bytes.to_vec())
            .collect::<Vec<u8>>();

        (items, full_names_data)
    }
}

#[cfg(test)]
mod tests {
    use anc_isa::DataSectionType;

    use crate::{
        common_sections::export_data_section::{ExportDataItem, ExportDataSection},
        entry::ExportDataEntry,
        module_image::{SectionEntry, Visibility},
    };

    #[test]
    fn test_write_section() {
        let items: Vec<ExportDataItem> = vec![
            ExportDataItem::new(0, 3, Visibility::Private, DataSectionType::ReadOnly),
            ExportDataItem::new(3, 5, Visibility::Public, DataSectionType::ReadWrite),
        ];

        let section = ExportDataSection {
            items: &items,
            full_names_data: "foohello".as_bytes(),
        };

        let mut section_data: Vec<u8> = vec![];
        section.write(&mut section_data).unwrap();

        let mut expect_data = vec![
            2u8, 0, 0, 0, // item count
            0, 0, 0, 0, // 4 bytes padding
            //
            0, 0, 0, 0, // name offset (item 0)
            3, 0, 0, 0, // name length
            0, // visibility
            0, // section type
            0, 0, // padding
            //
            3, 0, 0, 0, // name offset (item 1)
            5, 0, 0, 0, // name length
            1, // visibility
            1, // section type
            0, 0, // padding
        ];

        expect_data.extend_from_slice(b"foo");
        expect_data.extend_from_slice(b"hello");

        assert_eq!(section_data, expect_data);
    }

    #[test]
    fn test_read_section() {
        let mut section_data = vec![
            2u8, 0, 0, 0, // item count
            0, 0, 0, 0, // 4 bytes padding
            //
            0, 0, 0, 0, // name offset (item 0)
            3, 0, 0, 0, // name length
            0, // visibility
            0, // section type
            0, 0, // padding
            //
            3, 0, 0, 0, // name offset (item 1)
            5, 0, 0, 0, // name length
            1, // visibility
            1, // section type
            0, 0, // padding
        ];

        section_data.extend_from_slice("foo".as_bytes());
        section_data.extend_from_slice("hello".as_bytes());

        let section = ExportDataSection::read(&section_data);

        assert_eq!(section.items.len(), 2);
        assert_eq!(
            section.items[0],
            ExportDataItem::new(0, 3, Visibility::Private, DataSectionType::ReadOnly)
        );
        assert_eq!(
            section.items[1],
            ExportDataItem::new(3, 5, Visibility::Public, DataSectionType::ReadWrite)
        );
        assert_eq!(section.full_names_data, "foohello".as_bytes())
    }

    #[test]
    fn test_convert() {
        let entries: Vec<ExportDataEntry> = vec![
            ExportDataEntry::new(
                "foo".to_string(),
                Visibility::Private,
                DataSectionType::ReadOnly,
            ),
            ExportDataEntry::new(
                "hello".to_string(),
                Visibility::Public,
                DataSectionType::ReadWrite,
            ),
        ];

        let (items, names_data) = ExportDataSection::convert_from_entries(&entries);
        let section = ExportDataSection {
            items: &items,
            full_names_data: &names_data,
        };

        assert_eq!(
            section.get_item_index_and_visibility_and_section_type("foo"),
            Some((0, Visibility::Private, DataSectionType::ReadOnly))
        );
        assert_eq!(
            section.get_item_index_and_visibility_and_section_type("hello"),
            Some((1, Visibility::Public, DataSectionType::ReadWrite))
        );
        assert_eq!(
            section.get_item_index_and_visibility_and_section_type("bar"),
            None
        );

        assert_eq!(
            section.get_item_full_name_and_visibility_and_section_type(0),
            ("foo", Visibility::Private, DataSectionType::ReadOnly)
        );
        assert_eq!(
            section.get_item_full_name_and_visibility_and_section_type(1),
            ("hello", Visibility::Public, DataSectionType::ReadWrite)
        );

        let entries_restore = section.convert_to_entries();
        assert_eq!(entries, entries_restore);
    }
}
