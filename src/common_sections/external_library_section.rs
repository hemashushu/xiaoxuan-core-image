// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

// "external library section" binary layout
//
//              |-----------------------------------------------------------------------------|
//              | item count (u32) | (4 bytes padding)                                        |
//              |-----------------------------------------------------------------------------|
//  item 0 -->  | lib name off 0 (u32) | lib name len 0 (u32)                                 | <-- table
//              | value offset 0 (u32) | value length 0 (u32) | lib type 0 (u8) | pad 3 bytes |
//  item 1 -->  | lib name off 1       | lib name len 1                                       |
//              | value offset 0 (u32) | value length 0 (u32) | lib type 0 (u8) | pad 3 bytes |
//              | ...                                                                         |
//              |-----------------------------------------------------------------------------|
// offset 0 --> | name string 0 (UTF-8) | value string 0 (UTF-8)                              | <-- data area
// offset 1 --> | name string 1         | value string 1 (UTF-8)                              |
//              | ...                                                                         |
//              |-----------------------------------------------------------------------------|

use anc_isa::{ExternalLibraryDependency, ExternalLibraryDependencyType};

use crate::{
    entry::ExternalLibraryEntry,
    module_image::{ModuleSectionId, SectionEntry},
    tableaccess::{load_section_with_table_and_data_area, save_section_with_table_and_data_area},
};

#[derive(Debug, PartialEq, Default)]
pub struct ExternalLibrarySection<'a> {
    pub items: &'a [ExternalLibraryItem],
    pub items_data: &'a [u8],
}

#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct ExternalLibraryItem {
    pub name_offset: u32, // the offset of the name string in data area
    pub name_length: u32, // the length (in bytes) of the name string in data area
    pub value_offset: u32,
    pub value_length: u32,
    pub external_library_dependent_type: ExternalLibraryDependencyType, // u8
    _padding0: [u8; 3],
}

impl ExternalLibraryItem {
    pub fn new(
        name_offset: u32,
        name_length: u32,
        value_offset: u32,
        value_length: u32,
        external_library_dependent_type: ExternalLibraryDependencyType,
    ) -> Self {
        Self {
            name_offset,
            name_length,
            value_offset,
            value_length,
            external_library_dependent_type,
            _padding0: [0; 3],
        }
    }
}

impl<'a> SectionEntry<'a> for ExternalLibrarySection<'a> {
    fn load(section_data: &'a [u8]) -> Self {
        let (items, items_data) =
            load_section_with_table_and_data_area::<ExternalLibraryItem>(section_data);
        ExternalLibrarySection { items, items_data }
    }

    fn save(&'a self, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        save_section_with_table_and_data_area(self.items, self.items_data, writer)
    }

    fn id(&'a self) -> ModuleSectionId {
        ModuleSectionId::ExternalLibrary
    }
}

impl<'a> ExternalLibrarySection<'a> {
    pub fn get_item_name_and_external_library_dependent_type_and_value(
        &'a self,
        idx: usize,
    ) -> (&'a str, ExternalLibraryDependencyType, &'a [u8]) {
        let items = self.items;
        let items_data = self.items_data;

        let item = &items[idx];
        let name_data =
            &items_data[item.name_offset as usize..(item.name_offset + item.name_length) as usize];
        let value_data = &items_data
            [item.value_offset as usize..(item.value_offset + item.value_length) as usize];

        (
            std::str::from_utf8(name_data).unwrap(),
            item.external_library_dependent_type,
            value_data,
        )
    }

    pub fn convert_from_entries(
        entries: &[ExternalLibraryEntry],
    ) -> (Vec<ExternalLibraryItem>, Vec<u8>) {
        let mut name_bytes = entries
            .iter()
            .map(|entry| entry.name.as_bytes().to_vec())
            .collect::<Vec<Vec<u8>>>();

        let mut value_bytes = entries
            .iter()
            .map(|entry| {
                let value = entry.value.as_ref();
                let value_string = ason::to_string(value).unwrap();
                value_string.as_bytes().to_vec()
            })
            .collect::<Vec<Vec<u8>>>();

        let mut next_offset: u32 = 0;

        let items = entries
            .iter()
            .enumerate()
            .map(|(idx, entry)| {
                let name_length = name_bytes[idx].len() as u32;
                let value_length = value_bytes[idx].len() as u32;
                let name_offset = next_offset;
                let value_offset = name_offset + name_length;
                next_offset = value_offset + value_length; // for next offset

                let external_library_dependent_type = match entry.value.as_ref() {
                    ExternalLibraryDependency::Local(_) => ExternalLibraryDependencyType::Local,
                    ExternalLibraryDependency::Remote(_) => ExternalLibraryDependencyType::Remote,
                    ExternalLibraryDependency::Share(_) => ExternalLibraryDependencyType::Share,
                    ExternalLibraryDependency::Runtime => ExternalLibraryDependencyType::Runtime,
                    ExternalLibraryDependency::System(_) => ExternalLibraryDependencyType::System,
                };

                ExternalLibraryItem::new(
                    name_offset,
                    name_length,
                    value_offset,
                    value_length,
                    external_library_dependent_type,
                )
            })
            .collect::<Vec<ExternalLibraryItem>>();

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
    use core::str;

    use anc_isa::{
        DependencyLocal, DependencyRemote, ExternalLibraryDependency, ExternalLibraryDependencyType,
    };

    use crate::{
        common_sections::external_library_section::{ExternalLibraryItem, ExternalLibrarySection},
        entry::ExternalLibraryEntry,
        module_image::SectionEntry,
    };

    #[test]
    fn test_load_section() {
        let mut section_data = vec![
            2u8, 0, 0, 0, // item count
            0, 0, 0, 0, // 4 bytes padding
            //
            0, 0, 0, 0, // name offset (item 0)
            3, 0, 0, 0, // name length
            3, 0, 0, 0, // value offset
            5, 0, 0, 0, // value length
            0, // library dependent type
            0, 0, 0, // padding
            //
            8, 0, 0, 0, // name offset (item 1)
            4, 0, 0, 0, // name length
            12, 0, 0, 0, // value offset
            6, 0, 0, 0, // value length
            1, // library dependent type
            0, 0, 0, // padding
        ];

        section_data.extend_from_slice(b"foo");
        section_data.extend_from_slice(b"hello");
        section_data.extend_from_slice(b".bar");
        section_data.extend_from_slice(b".world");

        let section = ExternalLibrarySection::load(&section_data);

        assert_eq!(section.items.len(), 2);
        assert_eq!(
            section.items[0],
            ExternalLibraryItem::new(0, 3, 3, 5, ExternalLibraryDependencyType::Local)
        );
        assert_eq!(
            section.items[1],
            ExternalLibraryItem::new(8, 4, 12, 6, ExternalLibraryDependencyType::Remote)
        );
        assert_eq!(section.items_data, "foohello.bar.world".as_bytes())
    }

    #[test]
    fn test_save_section() {
        let items = vec![
            ExternalLibraryItem::new(0, 3, 3, 5, ExternalLibraryDependencyType::Local),
            ExternalLibraryItem::new(8, 4, 12, 6, ExternalLibraryDependencyType::Remote),
        ];

        let section = ExternalLibrarySection {
            items: &items,
            items_data: b"foohello.bar.world",
        };

        let mut section_data: Vec<u8> = Vec::new();
        section.save(&mut section_data).unwrap();

        let mut expect_data = vec![
            2u8, 0, 0, 0, // item count
            0, 0, 0, 0, // 4 bytes padding
            //
            0, 0, 0, 0, // name offset (item 0)
            3, 0, 0, 0, // name length
            3, 0, 0, 0, // value offset
            5, 0, 0, 0, // value length
            0, // library dependent type
            0, 0, 0, // padding
            //
            8, 0, 0, 0, // name offset (item 1)
            4, 0, 0, 0, // name length
            12, 0, 0, 0, // value offset
            6, 0, 0, 0, // value length
            1, // library dependent type
            0, 0, 0, // padding
        ];

        expect_data.extend_from_slice(b"foo");
        expect_data.extend_from_slice(b"hello");
        expect_data.extend_from_slice(b".bar");
        expect_data.extend_from_slice(b".world");

        // append padding which is inserted by function 'save()' for 4-byte align
        expect_data.extend_from_slice(&[0, 0]);

        assert_eq!(section_data, expect_data);
    }

    #[test]
    fn test_convert() {
        let entries = vec![
            ExternalLibraryEntry::new(
                "foobar".to_owned(),
                Box::new(ExternalLibraryDependency::Local(Box::new(
                    DependencyLocal {
                        path: "hello.so.1".to_owned(),
                        values: None,
                        condition: None,
                    },
                ))),
            ),
            ExternalLibraryEntry::new(
                "helloworld".to_owned(),
                Box::new(ExternalLibraryDependency::Remote(Box::new(
                    DependencyRemote {
                        url: "http://a.b/c".to_owned(),
                        reversion: "v1.0.1".to_owned(),
                        path: "/xyz.so.2".to_owned(),
                        values: None,
                        condition: None,
                    },
                ))),
            ),
        ];

        let (items, items_data) = ExternalLibrarySection::convert_from_entries(&entries);
        let section = ExternalLibrarySection {
            items: &items,
            items_data: &items_data,
        };

        let (name0, type0, value0) =
            section.get_item_name_and_external_library_dependent_type_and_value(0);
        let (name1, type1, value1) =
            section.get_item_name_and_external_library_dependent_type_and_value(1);

        assert_eq!(
            (name0, type0),
            ("foobar", ExternalLibraryDependencyType::Local)
        );
        assert_eq!(
            (name1, type1),
            ("helloworld", ExternalLibraryDependencyType::Remote)
        );

        let v0: ExternalLibraryDependency =
            ason::from_str(unsafe { str::from_utf8_unchecked(value0) }).unwrap();
        assert_eq!(&v0, entries[0].value.as_ref());

        let v1: ExternalLibraryDependency =
            ason::from_str(unsafe { str::from_utf8_unchecked(value1) }).unwrap();
        assert_eq!(&v1, entries[1].value.as_ref());
    }
}
