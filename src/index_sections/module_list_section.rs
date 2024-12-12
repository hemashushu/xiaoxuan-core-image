// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

//! The binary layout of this section is
//! the same as `ImportModuleSection`

// "import module section" binary layout
//
//              |-----------------------------------------------------------------------------|
//              | item count (u32) | (4 bytes padding)                                        |
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
    entry::ImportModuleEntry,
    module_image::{ModuleSectionId, SectionEntry},
    tableaccess::{load_section_with_table_and_data_area, save_section_with_table_and_data_area},
};

#[derive(Debug, PartialEq)]
pub struct ModuleListSection<'a> {
    pub items: &'a [ModuleListItem],
    pub items_data: &'a [u8],
}

#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct ModuleListItem {
    pub name_offset: u32, // the offset of the name string in data area
    pub name_length: u32, // the length (in bytes) of the name string in data area
    pub value_offset: u32,
    pub value_length: u32,
    pub module_dependent_type: ModuleDependencyType, // u8
    _padding0: [u8; 3],
}

impl ModuleListItem {
    pub fn new(
        name_offset: u32,
        name_length: u32,
        value_offset: u32,
        value_length: u32,
        module_dependent_type: ModuleDependencyType,
    ) -> Self {
        Self {
            name_offset,
            name_length,
            value_offset,
            value_length,
            module_dependent_type,
            _padding0: [0; 3],
        }
    }
}

impl<'a> SectionEntry<'a> for ModuleListSection<'a> {
    fn load(section_data: &'a [u8]) -> Self {
        let (items, names_data) =
            load_section_with_table_and_data_area::<ModuleListItem>(section_data);
        ModuleListSection {
            items,
            items_data: names_data,
        }
    }

    fn save(&'a self, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        save_section_with_table_and_data_area(self.items, self.items_data, writer)
    }

    fn id(&'a self) -> ModuleSectionId {
        ModuleSectionId::ModuleList
    }
}

impl<'a> ModuleListSection<'a> {
    pub fn get_item_name_and_module_dependent_type_and_value(
        &'a self,
        idx: usize,
    ) -> (&'a str, ModuleDependencyType, &'a [u8]) {
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
        )
    }

    pub fn convert_from_entries(entries: &[ImportModuleEntry]) -> (Vec<ModuleListItem>, Vec<u8>) {
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
                    ModuleDependency::Object(_) => ModuleDependencyType::Object,
                };

                ModuleListItem::new(
                    name_offset,
                    name_length,
                    value_offset,
                    value_length,
                    module_dependent_type,
                )
            })
            .collect::<Vec<ModuleListItem>>();

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

    use anc_isa::{DependencyLocal, DependencyRemote, ModuleDependency, ModuleDependencyType};

    use crate::{
        entry::ImportModuleEntry,
        index_sections::module_list_section::{ModuleListItem, ModuleListSection},
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
            0, // module dependent type
            0, 0, 0, // padding
            //
            8, 0, 0, 0, // name offset (item 1)
            4, 0, 0, 0, // name length
            12, 0, 0, 0, // value offset
            6, 0, 0, 0, // value length
            1, // module dependent type
            0, 0, 0, // padding
        ];

        section_data.extend_from_slice(b"foo");
        section_data.extend_from_slice(b"hello");
        section_data.extend_from_slice(b".bar");
        section_data.extend_from_slice(b".world");

        let section = ModuleListSection::load(&section_data);

        assert_eq!(section.items.len(), 2);
        assert_eq!(
            section.items[0],
            ModuleListItem::new(0, 3, 3, 5, ModuleDependencyType::Local)
        );
        assert_eq!(
            section.items[1],
            ModuleListItem::new(8, 4, 12, 6, ModuleDependencyType::Remote,)
        );
        assert_eq!(section.items_data, "foohello.bar.world".as_bytes())
    }

    #[test]
    fn test_save_section() {
        let items = vec![
            ModuleListItem::new(0, 3, 3, 5, ModuleDependencyType::Local),
            ModuleListItem::new(8, 4, 12, 6, ModuleDependencyType::Remote),
        ];

        let section = ModuleListSection {
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
            0, // module dependent type
            0, 0, 0, // padding
            //
            8, 0, 0, 0, // name offset (item 1)
            4, 0, 0, 0, // name length
            12, 0, 0, 0, // value offset
            6, 0, 0, 0, // value length
            1, // module dependent type
            0, 0, 0, // padding
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
                    condition: None,
                    values: None,
                }))),
            ),
            ImportModuleEntry::new(
                "helloworld".to_owned(),
                Box::new(ModuleDependency::Remote(Box::new(DependencyRemote {
                    url: "http://a.b/c".to_owned(),
                    reversion: "v1.0.1".to_owned(),
                    path: "/xyz".to_owned(),
                    values: None,
                    condition: None,
                }))),
            ),
        ];

        let (items, items_data) = ModuleListSection::convert_from_entries(&entries);
        let section = ModuleListSection {
            items: &items,
            items_data: &items_data,
        };

        let (name0, type0, value0) = section.get_item_name_and_module_dependent_type_and_value(0);
        let (name1, type1, value1) = section.get_item_name_and_module_dependent_type_and_value(1);

        assert_eq!((name0, type0), ("foobar", ModuleDependencyType::Local));
        assert_eq!((name1, type1), ("helloworld", ModuleDependencyType::Remote));

        let v0: ModuleDependency =
            ason::from_str(unsafe { str::from_utf8_unchecked(value0) }).unwrap();
        assert_eq!(&v0, entries[0].value.as_ref());

        let v1: ModuleDependency =
            ason::from_str(unsafe { str::from_utf8_unchecked(value1) }).unwrap();
        assert_eq!(&v1, entries[1].value.as_ref());
    }
}
