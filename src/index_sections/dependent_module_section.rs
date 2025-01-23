// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

// "dependent module section" binary layout
//
//              |-----------------------------------------------------------------------------|
//              | item count (u32) | extra header length (u32)                                |
//              |-----------------------------------------------------------------------------|
//  item 0 -->  | mod name off 0 (u32) | mod name len 0 (u32)                                 |  <-- table
//              | val offset (u32)     | val length 0 (u32)   | mod type 0 (u8) | pad 3 bytes |
//  item 1 -->  | mod name off 1       | mod name len 1                                       |
//              | val offset           | val offset 1         | mod type 1      |             |
//              | ...                                                                         |
//              |-----------------------------------------------------------------------------|
// offset 0 --> | name string 0 (UTF-8) | value string 0 (UTF-8)                              | <-- data area
// offset 1 --> | name string 1         | value string 1 (UTF-8)                              |
//              | ...                                                                         |
//              |-----------------------------------------------------------------------------|

use anc_isa::{ModuleDependency, ModuleDependencyType};

use crate::{
    entry::DependentModuleEntry,
    module_image::{ModuleSectionId, SectionEntry},
    datatableaccess::{read_section_with_table_and_data_area, write_section_with_table_and_data_area},
    DependencyHash,
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
    pub module_dependent_type: ModuleDependencyType, // u8
    _padding0: [u8; 3],

    // the hash of parameters and compile environment variables,
    // only exists in Local/Remote/Share dependencies
    pub hash: DependencyHash,
}

impl DependentModuleItem {
    pub fn new(
        name_offset: u32,
        name_length: u32,
        value_offset: u32,
        value_length: u32,
        module_dependent_type: ModuleDependencyType,
        hash: DependencyHash,
    ) -> Self {
        Self {
            name_offset,
            name_length,
            value_offset,
            value_length,
            module_dependent_type,
            _padding0: [0; 3],
            hash,
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
        ModuleSectionId::DependentModule
    }
}

impl<'a> DependentModuleSection<'a> {
    pub fn get_item_name_and_module_dependent_type_and_value_and_hash(
        &'a self,
        idx: usize,
    ) -> (&'a str, ModuleDependencyType, &'a [u8], &'a DependencyHash) {
        let items = self.items;
        let items_data = self.items_data;

        let item = &items[idx];
        let name_data =
            &items_data[item.name_offset as usize..(item.name_offset + item.name_length) as usize];
        let value_data = &items_data
            [item.value_offset as usize..(item.value_offset + item.value_length) as usize];

        (
            std::str::from_utf8(name_data).unwrap(),
            item.module_dependent_type,
            value_data,
            &item.hash,
        )
    }

    pub fn convert_from_entries(
        entries: &[DependentModuleEntry],
    ) -> (Vec<DependentModuleItem>, Vec<u8>) {
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

                let module_dependent_type = match entry.value.as_ref() {
                    ModuleDependency::Local(_) => ModuleDependencyType::Local,
                    ModuleDependency::Remote(_) => ModuleDependencyType::Remote,
                    ModuleDependency::Share(_) => ModuleDependencyType::Share,
                    ModuleDependency::Runtime => ModuleDependencyType::Runtime,
                    ModuleDependency::Current => ModuleDependencyType::Current,
                };

                DependentModuleItem::new(
                    name_offset,
                    name_length,
                    value_offset,
                    value_length,
                    module_dependent_type,
                    entry.hash,
                )
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

    use std::collections::HashMap;

    use anc_isa::{
        DependencyCondition, DependencyLocal, DependencyRemote, ModuleDependency,
        ModuleDependencyType,
    };

    use crate::{
        entry::DependentModuleEntry,
        index_sections::dependent_module_section::{DependentModuleItem, DependentModuleSection},
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
            0, // module dependent type
            0, 0, 0, // padding
            11, 13, 17, 19, 0, 0, 0, 0, // hash group 0
            0, 0, 0, 0, 0, 0, 0, 0, // hash group 1
            0, 0, 0, 0, 0, 0, 0, 0, // hash group 2
            0, 0, 0, 0, 0, 0, 0, 0, // hash group 3
            //
            8, 0, 0, 0, // name offset (item 1)
            4, 0, 0, 0, // name length
            12, 0, 0, 0, // value offset
            6, 0, 0, 0, // value length
            1, // module dependent type
            0, 0, 0, // padding
            23, 29, 31, 37, 0, 0, 0, 0, // hash group 0
            0, 0, 0, 0, 0, 0, 0, 0, // hash group 1
            0, 0, 0, 0, 0, 0, 0, 0, // hash group 2
            0, 0, 0, 0, 0, 0, 0, 0, // hash group 3
        ];

        section_data.extend_from_slice(b"foo");
        section_data.extend_from_slice(b"hello");
        section_data.extend_from_slice(b".bar");
        section_data.extend_from_slice(b".world");

        let section = DependentModuleSection::read(&section_data);

        assert_eq!(section.items.len(), 2);
        assert_eq!(
            section.items[0],
            DependentModuleItem::new(
                0,
                3,
                3,
                5,
                ModuleDependencyType::Local,
                [
                    11, 13, 17, 19, 0, 0, 0, 0, // hash group 0
                    0, 0, 0, 0, 0, 0, 0, 0, // hash group 1
                    0, 0, 0, 0, 0, 0, 0, 0, // hash group 2
                    0, 0, 0, 0, 0, 0, 0, 0, // hash group 3
                ]
            )
        );
        assert_eq!(
            section.items[1],
            DependentModuleItem::new(
                8,
                4,
                12,
                6,
                ModuleDependencyType::Remote,
                [
                    23, 29, 31, 37, 0, 0, 0, 0, // hash group 0
                    0, 0, 0, 0, 0, 0, 0, 0, // hash group 1
                    0, 0, 0, 0, 0, 0, 0, 0, // hash group 2
                    0, 0, 0, 0, 0, 0, 0, 0, // hash group 3
                ]
            )
        );
        assert_eq!(section.items_data, "foohello.bar.world".as_bytes())
    }

    #[test]
    fn test_write_section() {
        let items = vec![
            DependentModuleItem::new(
                0,
                3,
                3,
                5,
                ModuleDependencyType::Local,
                [
                    11, 13, 17, 19, 0, 0, 0, 0, // hash group 0
                    0, 0, 0, 0, 0, 0, 0, 0, // hash group 1
                    0, 0, 0, 0, 0, 0, 0, 0, // hash group 2
                    0, 0, 0, 0, 0, 0, 0, 0, // hash group 3
                ],
            ),
            DependentModuleItem::new(
                8,
                4,
                12,
                6,
                ModuleDependencyType::Remote,
                [
                    23, 29, 31, 37, 0, 0, 0, 0, // hash group 0
                    0, 0, 0, 0, 0, 0, 0, 0, // hash group 1
                    0, 0, 0, 0, 0, 0, 0, 0, // hash group 2
                    0, 0, 0, 0, 0, 0, 0, 0, // hash group 3
                ],
            ),
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
            0, // module dependent type
            0, 0, 0, // padding
            11, 13, 17, 19, 0, 0, 0, 0, // hash group 0
            0, 0, 0, 0, 0, 0, 0, 0, // hash group 1
            0, 0, 0, 0, 0, 0, 0, 0, // hash group 2
            0, 0, 0, 0, 0, 0, 0, 0, // hash group 3
            //
            8, 0, 0, 0, // name offset (item 1)
            4, 0, 0, 0, // name length
            12, 0, 0, 0, // value offset
            6, 0, 0, 0, // value length
            1, // module dependent type
            0, 0, 0, // padding
            23, 29, 31, 37, 0, 0, 0, 0, // hash group 0
            0, 0, 0, 0, 0, 0, 0, 0, // hash group 1
            0, 0, 0, 0, 0, 0, 0, 0, // hash group 2
            0, 0, 0, 0, 0, 0, 0, 0, // hash group 3
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
            DependentModuleEntry::new(
                "foobar".to_owned(),
                Box::new(ModuleDependency::Local(Box::new(DependencyLocal {
                    path: "hello".to_owned(),
                    condition: DependencyCondition::True,
                    parameters: HashMap::default(),
                }))),
                [11_u8; 32],
            ),
            DependentModuleEntry::new(
                "helloworld".to_owned(),
                Box::new(ModuleDependency::Remote(Box::new(DependencyRemote {
                    url: "http://a.b/c".to_owned(),
                    reversion: "v1.0.1".to_owned(),
                    path: "/xyz".to_owned(),
                    condition: DependencyCondition::True,
                    parameters: HashMap::default(),
                }))),
                [13_u8; 32],
            ),
        ];

        let (items, items_data) = DependentModuleSection::convert_from_entries(&entries);
        let section = DependentModuleSection {
            items: &items,
            items_data: &items_data,
        };

        let (name0, type0, value0, hash0) =
            section.get_item_name_and_module_dependent_type_and_value_and_hash(0);
        let (name1, type1, value1, hash1) =
            section.get_item_name_and_module_dependent_type_and_value_and_hash(1);

        assert_eq!(
            (name0, type0, hash0),
            ("foobar", ModuleDependencyType::Local, &[11_u8; 32])
        );
        assert_eq!(
            (name1, type1, hash1),
            ("helloworld", ModuleDependencyType::Remote, &[13_u8; 32])
        );

        let v0: ModuleDependency = ason::from_reader(value0).unwrap();
        assert_eq!(&v0, entries[0].value.as_ref());

        let v1: ModuleDependency = ason::from_reader(value1).unwrap();
        assert_eq!(&v1, entries[1].value.as_ref());
    }
}
