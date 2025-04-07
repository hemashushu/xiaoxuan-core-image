// Copyright (c) 2025 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions.
// For more details, see the LICENSE, LICENSE.additional, and CONTRIBUTING files.

// "Linking Module Section" binary layout:
//
//              |------------------------------------------------|
//              | item count (u32) | extra header length (u32)   |
//              |------------------------------------------------|
//  item 0 -->  | mod name off 0 (u32) | mod name len 0 (u32)    | <-- table
//              | val offset (u32)     | val length 0 (u32)      |
//  item 1 -->  | mod name off 1       | mod name len 1          |
//              | val offset           | val length 1            |
//              | ...                                            |
//              |------------------------------------------------|
// offset 0 --> | name string 0 (UTF-8) | value string 0 (UTF-8) | <-- data
// offset 1 --> | name string 1         | value string 1 (UTF-8) |
//              | ...                                            |
//              |------------------------------------------------|

use crate::{
    datatableaccess::{
        read_section_with_table_and_data_area, write_section_with_table_and_data_area,
    },
    entry::{LinkingModuleEntry, ModuleLocation},
    module_image::{ModuleSectionId, SectionEntry},
};

#[derive(Debug, PartialEq)]
pub struct LinkingModuleSection<'a> {
    // Array of items representing the module entries.
    pub items: &'a [LinkingModuleItem],
    // Raw data area containing the names and values as byte slices.
    pub items_data: &'a [u8],
}

/// Represents a dynamically linked module, including its name and location.
/// The first item in the entries is the main module in the application image.
#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct LinkingModuleItem {
    pub name_offset: u32,  // Offset of the name string in the data area.
    pub name_length: u32,  // Length (in bytes) of the name string in the data area.
    pub value_offset: u32, // Offset of the value string in the data area.
    pub value_length: u32, // Length (in bytes) of the value string in the data area.
}

impl LinkingModuleItem {
    /// Creates a new `LinkingModuleItem` with the specified offsets and lengths.
    pub fn new(name_offset: u32, name_length: u32, value_offset: u32, value_length: u32) -> Self {
        Self {
            name_offset,
            name_length,
            value_offset,
            value_length,
        }
    }
}

impl<'a> SectionEntry<'a> for LinkingModuleSection<'a> {
    /// Reads a `LinkingModuleSection` from the provided binary data.
    fn read(section_data: &'a [u8]) -> Self {
        let (items, names_data) =
            read_section_with_table_and_data_area::<LinkingModuleItem>(section_data);
        LinkingModuleSection {
            items,
            items_data: names_data,
        }
    }

    /// Writes the `LinkingModuleSection` to the provided writer.
    fn write(&'a self, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        write_section_with_table_and_data_area(self.items, self.items_data, writer)
    }

    /// Returns the section ID for the linking module.
    fn id(&'a self) -> ModuleSectionId {
        ModuleSectionId::LinkingModule
    }
}

impl<'a> LinkingModuleSection<'a> {
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

    /// Converts the section into a vector of `LinkingModuleEntry` objects.
    pub fn convert_to_entries(&self) -> Vec<LinkingModuleEntry> {
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
                let module_location: ModuleLocation = ason::from_reader(value_data).unwrap();
                LinkingModuleEntry::new(name, Box::new(module_location))
            })
            .collect()
    }

    /// Converts a vector of `LinkingModuleEntry` objects into a section representation.
    pub fn convert_from_entries(
        entries: &[LinkingModuleEntry],
    ) -> (Vec<LinkingModuleItem>, Vec<u8>) {
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
                next_offset = value_offset + value_length; // Update for the next offset.

                LinkingModuleItem::new(name_offset, name_length, value_offset, value_length)
            })
            .collect::<Vec<LinkingModuleItem>>();

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
        entry::{LinkingModuleEntry, ModuleLocation, ModuleLocationLocal, ModuleLocationShare},
        linking_sections::linking_module_section::{LinkingModuleItem, LinkingModuleSection},
        module_image::SectionEntry,
    };

    #[test]
    fn test_read_section() {
        let mut section_data = vec![
            2u8, 0, 0, 0, // Number of items.
            0, 0, 0, 0, // Extra section header length (u32).
            //
            0, 0, 0, 0, // Name offset (item 0).
            3, 0, 0, 0, // Name length.
            3, 0, 0, 0, // Value offset.
            5, 0, 0, 0, // Value length.
            //
            8, 0, 0, 0, // Name offset (item 1).
            4, 0, 0, 0, // Name length.
            12, 0, 0, 0, // Value offset.
            6, 0, 0, 0, // Value length.
        ];

        section_data.extend_from_slice(b"foo");
        section_data.extend_from_slice(b"hello");
        section_data.extend_from_slice(b".bar");
        section_data.extend_from_slice(b".world");

        let section = LinkingModuleSection::read(&section_data);

        assert_eq!(section.items.len(), 2);
        assert_eq!(section.items[0], LinkingModuleItem::new(0, 3, 3, 5,));
        assert_eq!(section.items[1], LinkingModuleItem::new(8, 4, 12, 6,));
        assert_eq!(section.items_data, "foohello.bar.world".as_bytes())
    }

    #[test]
    fn test_write_section() {
        let items = vec![
            LinkingModuleItem::new(0, 3, 3, 5),
            LinkingModuleItem::new(8, 4, 12, 6),
        ];

        let section = LinkingModuleSection {
            items: &items,
            items_data: b"foohello.bar.world",
        };

        let mut section_data: Vec<u8> = vec![];
        section.write(&mut section_data).unwrap();

        let mut expect_data = vec![
            2u8, 0, 0, 0, // Number of items.
            0, 0, 0, 0, // Extra section header length (u32).
            //
            0, 0, 0, 0, // Name offset (item 0).
            3, 0, 0, 0, // Name length.
            3, 0, 0, 0, // Value offset.
            5, 0, 0, 0, // Value length.
            //
            8, 0, 0, 0, // Name offset (item 1).
            4, 0, 0, 0, // Name length.
            12, 0, 0, 0, // Value offset.
            6, 0, 0, 0, // Value length.
        ];

        expect_data.extend_from_slice(b"foo");
        expect_data.extend_from_slice(b"hello");
        expect_data.extend_from_slice(b".bar");
        expect_data.extend_from_slice(b".world");

        expect_data.extend_from_slice(&[0, 0]); // Padding for 4-byte alignment.

        assert_eq!(section_data, expect_data);
    }

    #[test]
    fn test_convert() {
        let entries = vec![
            LinkingModuleEntry::new(
                "foobar".to_owned(),
                Box::new(ModuleLocation::Local(Box::new(ModuleLocationLocal {
                    module_path: "/path/to/module".to_owned(),
                    hash: "01234567".to_owned(),
                }))),
            ),
            LinkingModuleEntry::new(
                "helloworld".to_owned(),
                Box::new(ModuleLocation::Share(Box::new(ModuleLocationShare {
                    version: "1.2.3".to_owned(),
                    hash: "76543210".to_owned(),
                }))),
            ),
        ];

        let (items, items_data) = LinkingModuleSection::convert_from_entries(&entries);
        let section = LinkingModuleSection {
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

        let entries_restore = section.convert_to_entries();
        assert_eq!(entries_restore, entries);
    }
}
