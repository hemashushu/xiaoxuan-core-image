// Copyright (c) 2025 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions.
// For more details, see the LICENSE, LICENSE.additional, and CONTRIBUTING files.

// "Import Module Section" binary layout:
//
//              |---------------------------------------------------------|
//              | item count (u32) | extra header length (u32)            |
//              |---------------------------------------------------------|
//  item 0 -->  | module name offset 0 (u32) | module name length 0 (u32) |
//              | value offset (u32) | value length 0 (u32)               | <-- table
//  item 1 -->  | module name offset 1       | module name length 1       |
//              | value offset       | value offset 1                     |
//              | ...                                                     |
//              |---------------------------------------------------------|
// offset 0 --> | name string 0 (UTF-8) | value string 0 (UTF-8)          | <-- data
// offset 1 --> | name string 1         | value string 1 (UTF-8)          |
//              | ...                                                     |
//              |---------------------------------------------------------|

use anc_isa::ModuleDependency;

use crate::{
    datatableaccess::{
        read_section_with_table_and_data_area, write_section_with_table_and_data_area,
    },
    entry::ImportModuleEntry,
    module_image::{ModuleSectionId, SectionEntry},
};

#[derive(Debug, PartialEq, Default)]
pub struct ImportModuleSection<'a> {
    pub items: &'a [ImportModuleItem],
    pub items_data: &'a [u8],
}

#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct ImportModuleItem {
    pub name_offset: u32,  // Offset of the name string in the data area
    pub name_length: u32,  // Length (in bytes) of the name string in the data area
    pub value_offset: u32, // Offset of the value string in the data area
    pub value_length: u32, // Length (in bytes) of the value string in the data area
}

impl ImportModuleItem {
    pub fn new(name_offset: u32, name_length: u32, value_offset: u32, value_length: u32) -> Self {
        Self {
            name_offset,
            name_length,
            value_offset,
            value_length,
        }
    }
}

impl<'a> SectionEntry<'a> for ImportModuleSection<'a> {
    fn read(section_data: &'a [u8]) -> Self {
        let (items, names_data) =
            read_section_with_table_and_data_area::<ImportModuleItem>(section_data);
        ImportModuleSection {
            items,
            items_data: names_data,
        }
    }

    fn write(&'a self, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        write_section_with_table_and_data_area(self.items, self.items_data, writer)
    }

    fn id(&'a self) -> ModuleSectionId {
        ModuleSectionId::ImportModule
    }
}

impl<'a> ImportModuleSection<'a> {
    /// Retrieves the name and value of an item at the specified index.
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

    /// Converts the section into a vector of `ImportModuleEntry` objects.
    pub fn convert_to_entries(&self) -> Vec<ImportModuleEntry> {
        let items = self.items;
        let items_data = self.items_data;

        items
            .iter()
            .map(|item| {
                let name_data = &items_data
                    [item.name_offset as usize..(item.name_offset + item.name_length) as usize];
                let value_data = &items_data
                    [item.value_offset as usize..(item.value_offset + item.value_length) as usize];

                let name = std::str::from_utf8(name_data).unwrap().to_owned();
                let module_dependency: ModuleDependency = ason::from_reader(value_data).unwrap();
                ImportModuleEntry::new(name, Box::new(module_dependency))
            })
            .collect()
    }

    /// Converts a vector of `ImportModuleEntry` objects into the section's internal representation.
    pub fn convert_from_entries(entries: &[ImportModuleEntry]) -> (Vec<ImportModuleItem>, Vec<u8>) {
        let mut name_bytes = entries
            .iter()
            .map(|entry| entry.name.as_bytes().to_vec())
            .collect::<Vec<Vec<u8>>>();

        let mut value_bytes = entries
            .iter()
            .map(|entry| {
                let value = entry.module_dependency.as_ref();
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

                ImportModuleItem::new(name_offset, name_length, value_offset, value_length)
            })
            .collect::<Vec<ImportModuleItem>>();

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
    use std::collections::HashMap;

    use anc_isa::{DependencyCondition, DependencyLocal, DependencyRemote, ModuleDependency};

    use crate::{
        common_sections::import_module_section::{ImportModuleItem, ImportModuleSection},
        entry::ImportModuleEntry,
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

        let section = ImportModuleSection::read(&section_data);

        assert_eq!(section.items.len(), 2);
        assert_eq!(section.items[0], ImportModuleItem::new(0, 3, 3, 5));
        assert_eq!(section.items[1], ImportModuleItem::new(8, 4, 12, 6));
        assert_eq!(section.items_data, "foohello.bar.world".as_bytes())
    }

    #[test]
    fn test_write_section() {
        let items = vec![
            ImportModuleItem::new(0, 3, 3, 5),
            ImportModuleItem::new(8, 4, 12, 6),
        ];

        let section = ImportModuleSection {
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
            ImportModuleEntry::new(
                "foobar".to_owned(),
                Box::new(ModuleDependency::Local(Box::new(DependencyLocal {
                    path: "hello".to_owned(),
                    condition: DependencyCondition::True,
                    parameters: HashMap::default(),
                }))),
            ),
            ImportModuleEntry::new(
                "helloworld".to_owned(),
                Box::new(ModuleDependency::Remote(Box::new(DependencyRemote {
                    url: "http://a.b/c".to_owned(),
                    dir: Some("/modules/helloworld".to_owned()),
                    reversion: "v1.0.1".to_owned(),
                    condition: DependencyCondition::True,
                    parameters: HashMap::default(),
                }))),
            ),
        ];

        let (items, items_data) = ImportModuleSection::convert_from_entries(&entries);
        let section = ImportModuleSection {
            items: &items,
            items_data: &items_data,
        };

        let (name0, value0) = section.get_item_name_and_value(0);
        let (name1, value1) = section.get_item_name_and_value(1);

        assert_eq!(name0, "foobar");
        assert_eq!(name1, "helloworld");

        let v0: ModuleDependency = ason::from_reader(value0).unwrap();
        assert_eq!(&v0, entries[0].module_dependency.as_ref());

        let v1: ModuleDependency = ason::from_reader(value1).unwrap();
        assert_eq!(&v1, entries[1].module_dependency.as_ref());

        let entries_restore = section.convert_to_entries();
        assert_eq!(entries, entries_restore);
    }
}
