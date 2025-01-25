// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

// "dependent module section" binary layout
//
//              |------------------------------------------------|
//              | item count (u32) | extra header length (u32)   |
//              |------------------------------------------------|
//  item 0 -->  | mod name off 0 (u32) | mod name len 0 (u32)    |  <-- table
//              | val offset (u32)     | val length 0 (u32)      |
//  item 1 -->  | mod name off 1       | mod name len 1          |
//              | val offset           | val offset 1            |
//              | ...                                            |
//              |------------------------------------------------|
// offset 0 --> | name string 0 (UTF-8) | value string 0 (UTF-8) | <-- data area
// offset 1 --> | name string 1         | value string 1 (UTF-8) |
//              | ...                                            |
//              |------------------------------------------------|

use crate::{
    datatableaccess::{
        read_section_with_table_and_data_area, write_section_with_table_and_data_area,
    },
    entry::DynamicLinkModuleEntry,
    module_image::{ModuleSectionId, SectionEntry},
};

#[derive(Debug, PartialEq)]
pub struct DependentModuleSection<'a> {
    pub items: &'a [DependentModuleItem],
    pub items_data: &'a [u8],
}

#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct DependentModuleItem {
    pub name_offset: u32, // the offset of the name string in data area
    pub name_length: u32, // the length (in bytes) of the name string in data area
    pub value_offset: u32,
    pub value_length: u32,
}

impl DependentModuleItem {
    pub fn new(name_offset: u32, name_length: u32, value_offset: u32, value_length: u32) -> Self {
        Self {
            name_offset,
            name_length,
            value_offset,
            value_length,
        }
    }
}

impl<'a> SectionEntry<'a> for DependentModuleSection<'a> {
    fn read(section_data: &'a [u8]) -> Self {
        let (items, names_data) =
            read_section_with_table_and_data_area::<DependentModuleItem>(section_data);
        DependentModuleSection {
            items,
            items_data: names_data,
        }
    }

    fn write(&'a self, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        write_section_with_table_and_data_area(self.items, self.items_data, writer)
    }

    fn id(&'a self) -> ModuleSectionId {
        ModuleSectionId::DynamicLinkModule
    }
}

impl<'a> DependentModuleSection<'a> {
    pub fn get_item_name_and_value(&'a self, idx: usize) -> (&'a str, &'a [u8]) {
        let items = self.items;
        let items_data = self.items_data;

        let item = &items[idx];
        let name_data =
            &items_data[item.name_offset as usize..(item.name_offset + item.name_length) as usize];
        let value_data = &items_data
            [item.value_offset as usize..(item.value_offset + item.value_length) as usize];

        (std::str::from_utf8(name_data).unwrap(), value_data)
    }

    pub fn convert_from_entries(
        entries: &[DynamicLinkModuleEntry],
    ) -> (Vec<DependentModuleItem>, Vec<u8>) {
        let mut name_bytes = entries
            .iter()
            .map(|entry| entry.name.as_bytes().to_vec())
            .collect::<Vec<Vec<u8>>>();

        let mut value_bytes = entries
            .iter()
            .map(|entry| {
                let value = entry.module_location.as_ref();
                let value_string = ason::to_string(value).unwrap();
                value_string.as_bytes().to_vec()
            })
            .collect::<Vec<Vec<u8>>>();

        let mut next_offset: u32 = 0;

        let items = (0..entries.len())
            .map(|idx| {
                let name_length = name_bytes[idx].len() as u32;
                let value_length = value_bytes[idx].len() as u32;
                let name_offset = next_offset;
                let value_offset = name_offset + name_length;
                next_offset = value_offset + value_length; // for next offset

                DependentModuleItem::new(name_offset, name_length, value_offset, value_length)
            })
            .collect::<Vec<DependentModuleItem>>();

        let items_data = name_bytes
            .iter_mut()
            .zip(value_bytes.iter_mut())
            .flat_map(|(name_bytes, value_bytes)| {
                name_bytes.append(value_bytes);
                name_bytes.to_owned()
            })
            .collect::<Vec<u8>>();

        (items, items_data)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        entry::{DynamicLinkModuleEntry, ModuleLocation, ModuleLocationCache, ModuleLocationLocal},
        index_sections::dynamic_link_module_section::{
            DependentModuleItem, DependentModuleSection,
        },
        module_image::SectionEntry,
    };

    #[test]
    fn test_read_section() {
        let mut section_data = vec![
            2u8, 0, 0, 0, // item count
            0, 0, 0, 0, // extra section header len (i32)
            //
            0, 0, 0, 0, // name offset (item 0)
            3, 0, 0, 0, // name length
            3, 0, 0, 0, // value offset
            5, 0, 0, 0, // value length
            //
            8, 0, 0, 0, // name offset (item 1)
            4, 0, 0, 0, // name length
            12, 0, 0, 0, // value offset
            6, 0, 0, 0, // value length
        ];

        section_data.extend_from_slice(b"foo");
        section_data.extend_from_slice(b"hello");
        section_data.extend_from_slice(b".bar");
        section_data.extend_from_slice(b".world");

        let section = DependentModuleSection::read(&section_data);

        assert_eq!(section.items.len(), 2);
        assert_eq!(section.items[0], DependentModuleItem::new(0, 3, 3, 5,));
        assert_eq!(section.items[1], DependentModuleItem::new(8, 4, 12, 6,));
        assert_eq!(section.items_data, "foohello.bar.world".as_bytes())
    }

    #[test]
    fn test_write_section() {
        let items = vec![
            DependentModuleItem::new(0, 3, 3, 5),
            DependentModuleItem::new(8, 4, 12, 6),
        ];

        let section = DependentModuleSection {
            items: &items,
            items_data: b"foohello.bar.world",
        };

        let mut section_data: Vec<u8> = vec![];
        section.write(&mut section_data).unwrap();

        let mut expect_data = vec![
            2u8, 0, 0, 0, // item count
            0, 0, 0, 0, // extra section header len (i32)
            //
            0, 0, 0, 0, // name offset (item 0)
            3, 0, 0, 0, // name length
            3, 0, 0, 0, // value offset
            5, 0, 0, 0, // value length
            //
            8, 0, 0, 0, // name offset (item 1)
            4, 0, 0, 0, // name length
            12, 0, 0, 0, // value offset
            6, 0, 0, 0, // value length
        ];

        expect_data.extend_from_slice(b"foo");
        expect_data.extend_from_slice(b"hello");
        expect_data.extend_from_slice(b".bar");
        expect_data.extend_from_slice(b".world");

        expect_data.extend_from_slice(&[0, 0]); // padding for 4-byte align

        assert_eq!(section_data, expect_data);
    }

    #[test]
    fn test_convert() {
        let entries = vec![
            DynamicLinkModuleEntry::new(
                "foobar".to_owned(),
                Box::new(ModuleLocation::Local(Box::new(ModuleLocationLocal {
                    path: "/path/to/module".to_owned(),
                    hash: "01234567".to_owned(),
                }))),
            ),
            DynamicLinkModuleEntry::new(
                "helloworld".to_owned(),
                Box::new(ModuleLocation::Cache(Box::new(ModuleLocationCache {
                    version: Some("1.2.3".to_owned()),
                    hash: "76543210".to_owned(),
                }))),
            ),
        ];

        let (items, items_data) = DependentModuleSection::convert_from_entries(&entries);
        let section = DependentModuleSection {
            items: &items,
            items_data: &items_data,
        };

        let (name0, value0) = section.get_item_name_and_value(0);
        let (name1, value1) = section.get_item_name_and_value(1);

        assert_eq!(name0, "foobar");
        assert_eq!(name1, "helloworld");

        let v0: ModuleLocation = ason::from_reader(value0).unwrap();
        assert_eq!(&v0, entries[0].module_location.as_ref());

        let v1: ModuleLocation = ason::from_reader(value1).unwrap();
        assert_eq!(&v1, entries[1].module_location.as_ref());
    }
}
