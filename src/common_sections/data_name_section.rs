// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

// In the default VM implementation, this section contains a list of all internal data,
// including both public and private data. While it is not mandatory,
// public data should be included to facilitate export and archive linking.
//
// Private data names are primarily used for debugging purposes, and are not
// essential for the VM's functionality.

// Data is accessed using the `data_public_index`, which is calculated as:
// `data_public_index = (number of all imported data) + mixed_data_internal_index`
//
// Where the imported data are combined with:
// - Imported read-only data
// - Imported read-write data
// - Imported uninitialized data
//
// And the `mixed data internal index` is a combination of:
// - Internal read-only data
// - Internal read-write data
// - Internal uninitialized data
//
// The diagram below illustrates the relationship between the
// mixed data internal index and the data public index:
//
//               /-----------------------------\ <--\
// number of     | Imported read-only data     |    |
// imported  --> | Imported read-write data    |    |
// data          | Imported uninitialized data |    |
//               |-----------------------------|    |
//               |                             |    |
// mixed         | Internal read-only data     |    |
// data      --> | Internal read-write data    |    | <-- data public index
// internal      | Internal uninitialized data |    |
// index         |                             |    |
//               \-----------------------------/ <--/
//
// Note: This structure is not mandatory.

// "Data Name Section" binary layout:
//
//              |-------------------------------------------------------|
//              | item count (u32) | extra header length (u32)          |
//              |-------------------------------------------------------|
//  item 0 -->  | full name offset 0 (u32) | full name length 0 (u32)   |
//              | visibility 0 (u8) | section type 0 (u8) | pad 2 bytes | <-- table
//              | internal_index_in_section (u32)                       |
//              |                                                       |
//  item 1 -->  | full name offset 1       | full name length 1         |
//              | visibility 1      | section type 1      | pad 2 bytes |
//              | internal_index_in_section (u32)                       |
//              |                                                       |
//              | ...                                                   |
//              |-------------------------------------------------------|
// offset 0 --> | full name string 0 (UTF-8)                            | <-- data
// offset 1 --> | full name string 1                                    |
//              | ...                                                   |
//              |-------------------------------------------------------|

use anc_isa::DataSectionType;

use crate::entry::DataNameEntry;

use crate::module_image::Visibility;
use crate::{
    datatableaccess::{
        read_section_with_table_and_data_area, write_section_with_table_and_data_area,
    },
    module_image::{ModuleSectionId, SectionEntry},
};

#[derive(Debug, PartialEq, Default)]
pub struct DataNameSection<'a> {
    pub items: &'a [DataNameItem],
    pub full_names_data: &'a [u8],
}

// This table only contains internal data.
// Imported data is not listed in this table.
#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct DataNameItem {
    // Explanation of "full_name" and "name_path":
    // ------------------------------------------
    // - "full_name"  = "module_name::name_path"
    // - "name_path"  = "namespaces::identifier"
    // - "namespaces" = "sub_module_name"{0,N}
    //
    // For example, assuming there is an object named "config" in the submodule "myapp::settings":
    // - The full name is "myapp::settings::config".
    // - The module name is "myapp".
    // - The name path is "settings::config".
    pub full_name_offset: u32,
    pub full_name_length: u32,
    pub visibility: Visibility,
    pub section_type: DataSectionType,
    _padding0: [u8; 2],

    /// The data index in a specific data section.
    pub internal_index_in_section: u32,
}

impl DataNameItem {
    pub fn new(
        full_name_offset: u32,
        full_name_length: u32,
        visibility: Visibility,
        section_type: DataSectionType,
        internal_index_in_section: u32,
    ) -> Self {
        Self {
            full_name_offset,
            full_name_length,
            visibility,
            section_type,
            _padding0: [0, 0],
            internal_index_in_section,
        }
    }
}

impl<'a> SectionEntry<'a> for DataNameSection<'a> {
    fn read(section_data: &'a [u8]) -> Self {
        let (items, full_names_data) =
            read_section_with_table_and_data_area::<DataNameItem>(section_data);
        DataNameSection {
            items,
            full_names_data,
        }
    }

    fn write(&'a self, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        write_section_with_table_and_data_area(self.items, self.full_names_data, writer)
    }

    fn id(&'a self) -> ModuleSectionId {
        ModuleSectionId::DataName
    }
}

impl<'a> DataNameSection<'a> {
    /// Get `(visibility, section_type, data_internal_index_in_section)` by full name.
    pub fn get_item_visibility_and_section_type_and_data_internal_index_in_section(
        &'a self,
        expected_full_name: &str,
    ) -> Option<(
        Visibility,
        DataSectionType,
        usize, // data_internal_index_in_section
    )> {
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
            (
                item.visibility,
                item.section_type,
                item.internal_index_in_section as usize,
            )
        })
    }

    /// Get `(full_name, visibility)`
    /// by data section type and data internal index.
    pub fn get_item_full_name_and_visibility(
        &self,
        data_section_type: DataSectionType,
        data_internal_index: usize,
    ) -> Option<(
        &str, // full_name
        Visibility,
    )> {
        let items = self.items;
        let full_names_data = self.full_names_data;

        let opt_idx = items.iter().position(|item| {
            item.section_type == data_section_type
                && item.internal_index_in_section as usize == data_internal_index
        });

        opt_idx.map(|idx| {
            let item = &items[idx];
            let full_name_data = &full_names_data[item.full_name_offset as usize
                ..(item.full_name_offset + item.full_name_length) as usize];
            let full_name = std::str::from_utf8(full_name_data).unwrap();
            (full_name, item.visibility)
        })
    }

    /// Converts the section into a vector of `ExportDataEntry`.
    pub fn convert_to_entries(&self) -> Vec<DataNameEntry> {
        let items = self.items;
        let full_names_data = self.full_names_data;
        items
            .iter()
            .map(|item| {
                let full_name_data = &full_names_data[item.full_name_offset as usize
                    ..(item.full_name_offset + item.full_name_length) as usize];
                let full_name = std::str::from_utf8(full_name_data).unwrap();
                DataNameEntry::new(
                    full_name.to_owned(),
                    item.visibility,
                    item.section_type,
                    item.internal_index_in_section as usize,
                )
            })
            .collect()
    }

    /// Converts a vector of `ExportDataEntry` into section data.
    pub fn convert_from_entries(entries: &[DataNameEntry]) -> (Vec<DataNameItem>, Vec<u8>) {
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

                DataNameItem::new(
                    full_name_offset,
                    full_name_length,
                    entry.visibility,
                    entry.section_type,
                    entry.internal_index_in_section as u32,
                )
            })
            .collect::<Vec<DataNameItem>>();

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
        common_sections::data_name_section::{DataNameItem, DataNameSection},
        entry::DataNameEntry,
        module_image::{SectionEntry, Visibility},
    };

    #[test]
    fn test_write_section() {
        let items: Vec<DataNameItem> = vec![
            DataNameItem::new(0, 3, Visibility::Private, DataSectionType::ReadOnly, 11),
            DataNameItem::new(3, 5, Visibility::Public, DataSectionType::ReadWrite, 13),
        ];

        let section = DataNameSection {
            items: &items,
            full_names_data: "foohello".as_bytes(),
        };

        let mut section_data: Vec<u8> = vec![];
        section.write(&mut section_data).unwrap();

        let mut expect_data = vec![
            2u8, 0, 0, 0, // item count
            0, 0, 0, 0, // extra section header len (i32)
            //
            0, 0, 0, 0, // name offset (item 0)
            3, 0, 0, 0, // name length
            0, // visibility
            0, // section type
            0, 0, // padding
            11, 0, 0, 0, // internal index in section
            //
            3, 0, 0, 0, // name offset (item 1)
            5, 0, 0, 0, // name length
            1, // visibility
            1, // section type
            0, 0, // padding
            13, 0, 0, 0, // internal index in section
        ];

        expect_data.extend_from_slice(b"foo");
        expect_data.extend_from_slice(b"hello");

        assert_eq!(section_data, expect_data);
    }

    #[test]
    fn test_read_section() {
        let mut section_data = vec![
            2u8, 0, 0, 0, // item count
            0, 0, 0, 0, // extra section header len (i32)
            //
            0, 0, 0, 0, // name offset (item 0)
            3, 0, 0, 0, // name length
            0, // visibility
            0, // section type
            0, 0, // padding
            11, 0, 0, 0, // internal index in section
            //
            3, 0, 0, 0, // name offset (item 1)
            5, 0, 0, 0, // name length
            1, // visibility
            1, // section type
            0, 0, // padding
            13, 0, 0, 0, // internal index in section
        ];

        section_data.extend_from_slice("foo".as_bytes());
        section_data.extend_from_slice("hello".as_bytes());

        let section = DataNameSection::read(&section_data);

        assert_eq!(section.items.len(), 2);
        assert_eq!(
            section.items[0],
            DataNameItem::new(0, 3, Visibility::Private, DataSectionType::ReadOnly, 11)
        );
        assert_eq!(
            section.items[1],
            DataNameItem::new(3, 5, Visibility::Public, DataSectionType::ReadWrite, 13)
        );
        assert_eq!(section.full_names_data, "foohello".as_bytes())
    }

    #[test]
    fn test_convert() {
        let entries: Vec<DataNameEntry> = vec![
            DataNameEntry::new(
                "foo".to_string(),
                Visibility::Private,
                DataSectionType::ReadOnly,
                11,
            ),
            DataNameEntry::new(
                "hello".to_string(),
                Visibility::Public,
                DataSectionType::ReadWrite,
                13,
            ),
        ];

        let (items, names_data) = DataNameSection::convert_from_entries(&entries);
        let section = DataNameSection {
            items: &items,
            full_names_data: &names_data,
        };

        assert_eq!(
            section.get_item_visibility_and_section_type_and_data_internal_index_in_section("foo"),
            Some((Visibility::Private, DataSectionType::ReadOnly, 11))
        );
        assert_eq!(
            section.get_item_visibility_and_section_type_and_data_internal_index_in_section("hello"),
            Some((Visibility::Public, DataSectionType::ReadWrite, 13))
        );
        assert_eq!(section.get_item_visibility_and_section_type_and_data_internal_index_in_section("bar"), None);

        assert_eq!(
            section.get_item_full_name_and_visibility(
                DataSectionType::ReadOnly,
                11
            ),
            Some(("foo", Visibility::Private))
        );
        assert_eq!(
            section.get_item_full_name_and_visibility(
                DataSectionType::ReadWrite,
                13
            ),
            Some(("hello", Visibility::Public))
        );

        let entries_restore = section.convert_to_entries();
        assert_eq!(entries, entries_restore);
    }
}
