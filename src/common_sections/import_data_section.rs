// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

// "import data section" binary layout
//
//              |------------------------------------------------------------------------------------------------------------------------------------------------|
//              | item count (u32) | (4 bytes padding)                                                                                                           |
//              |------------------------------------------------------------------------------------------------------------------------------------------------|
//  item 0 -->  | data name path off 0 (u32) | data name path len 0 (u32) | import module idx 0 (u32) | dat sec type 0 (u8) | mem data type 0 (u8) | pad 2 bytes | <-- table
//  item 1 -->  | data name path off 1       | data name path len 1       | import module idx 1       | dat sec type 1                                           |
//              | ...                                                                                                                                            |
//              |------------------------------------------------------------------------------------------------------------------------------------------------|
// offset 0 --> | name path string 0 (UTF-8)                                                                                                                     | <-- data area
// offset 1 --> | name path string 1                                                                                                                             |
//              | ...                                                                                                                                            |
//              |------------------------------------------------------------------------------------------------------------------------------------------------|

use anc_isa::{DataSectionType, MemoryDataType};

use crate::{
    entry::ImportDataEntry,
    module_image::{ModuleSectionId, SectionEntry},
    tableaccess::{load_section_with_table_and_data_area, save_section_with_table_and_data_area},
};

#[derive(Debug, PartialEq)]
pub struct ImportDataSection<'a> {
    pub items: &'a [ImportDataItem],
    pub name_paths_data: &'a [u8],
}

#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct ImportDataItem {
    // about the "full_name" and "name_path"
    // -------------------------------------
    // - "full_name" = "module_name::name_path"
    // - "name_path" = "namespace::identifier"
    // - "namespace" = "sub_module_name"{0,N}
    pub name_path_offset: u32, // the offset of the name string in data area
    pub name_path_length: u32, // the length (in bytes) of the name string in data area
    pub import_module_index: u32,
    pub data_section_type: DataSectionType,
    pub memory_data_type: MemoryDataType,
    _padding0: [u8; 2],
}

impl ImportDataItem {
    pub fn new(
        name_path_offset: u32,
        name_path_length: u32,
        import_module_index: u32,
        data_section_type: DataSectionType,
        memory_data_type: MemoryDataType,
    ) -> Self {
        Self {
            name_path_offset,
            name_path_length,
            import_module_index,
            data_section_type,
            memory_data_type,
            _padding0: [0; 2],
        }
    }
}

impl<'a> SectionEntry<'a> for ImportDataSection<'a> {
    fn load(section_data: &'a [u8]) -> Self {
        let (items, name_paths_data) =
            load_section_with_table_and_data_area::<ImportDataItem>(section_data);
        ImportDataSection { items, name_paths_data }
    }

    fn save(&'a self, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        save_section_with_table_and_data_area(self.items, self.name_paths_data, writer)
    }

    fn id(&'a self) -> ModuleSectionId {
        ModuleSectionId::ImportData
    }
}

impl<'a> ImportDataSection<'a> {
    pub fn get_item_name_path_and_import_module_index_and_data_section_type_and_memory_data_type(
        &'a self,
        idx: usize,
    ) -> (&'a str, usize, DataSectionType, MemoryDataType) {
        let items = self.items;
        let name_paths_data = self.name_paths_data;

        let item = &items[idx];
        let name_data =
            &name_paths_data[item.name_path_offset as usize..(item.name_path_offset + item.name_path_length) as usize];

        (
            std::str::from_utf8(name_data).unwrap(),
            item.import_module_index as usize,
            item.data_section_type,
            item.memory_data_type,
        )
    }

    pub fn convert_from_entries(entries: &[ImportDataEntry]) -> (Vec<ImportDataItem>, Vec<u8>) {
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

                ImportDataItem::new(
                    name_path_offset,
                    name_path_length,
                    entry.import_module_index as u32,
                    entry.data_section_type,
                    entry.memory_data_type,
                )
            })
            .collect::<Vec<ImportDataItem>>();

        let name_paths_data = name_path_bytes
            .iter()
            .flat_map(|bytes| bytes.to_vec())
            .collect::<Vec<u8>>();

        (items, name_paths_data)
    }
}

#[cfg(test)]
mod tests {
    use anc_isa::{DataSectionType, MemoryDataType};

    use crate::{
        common_sections::import_data_section::{ImportDataItem, ImportDataSection},
        entry::ImportDataEntry,
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
            11, 0, 0, 0, // import module index
            0, // data section type
            0, // mem data type
            0, 0, // pading
            //
            3, 0, 0, 0, // name offset (item 1)
            5, 0, 0, 0, // name length
            13, 0, 0, 0, // import module index
            1, // data section type
            1, // mem data type
            0, 0, // pading
        ];

        section_data.extend_from_slice(b"foo");
        section_data.extend_from_slice(b"hello");

        let section = ImportDataSection::load(&section_data);

        assert_eq!(section.items.len(), 2);
        assert_eq!(
            section.items[0],
            ImportDataItem::new(0, 3, 11, DataSectionType::ReadOnly, MemoryDataType::I32,)
        );
        assert_eq!(
            section.items[1],
            ImportDataItem::new(3, 5, 13, DataSectionType::ReadWrite, MemoryDataType::I64,)
        );
        assert_eq!(section.name_paths_data, "foohello".as_bytes())
    }

    #[test]
    fn test_save_section() {
        let items = vec![
            ImportDataItem::new(0, 3, 11, DataSectionType::ReadOnly, MemoryDataType::I32),
            ImportDataItem::new(3, 5, 13, DataSectionType::ReadWrite, MemoryDataType::I64),
        ];

        let section = ImportDataSection {
            items: &items,
            name_paths_data: b"foohello",
        };

        let mut section_data: Vec<u8> = Vec::new();
        section.save(&mut section_data).unwrap();

        let mut expect_data = vec![
            2u8, 0, 0, 0, // item count
            0, 0, 0, 0, // 4 bytes padding
            //
            0, 0, 0, 0, // name offset (item 0)
            3, 0, 0, 0, // name length
            11, 0, 0, 0, // import module index
            0, // data section type
            0, // mem data type
            0, 0, // pading
            //
            3, 0, 0, 0, // name offset (item 1)
            5, 0, 0, 0, // name length
            13, 0, 0, 0, // import module index
            1, // data section type
            1, // mem data type
            0, 0, // pading
        ];

        expect_data.extend_from_slice(b"foo");
        expect_data.extend_from_slice(b"hello");

        assert_eq!(section_data, expect_data);
    }

    #[test]
    fn test_convert() {
        let entries = vec![
            ImportDataEntry::new(
                "foobar".to_string(),
                11,
                DataSectionType::ReadOnly,
                MemoryDataType::I32,
            ),
            ImportDataEntry::new(
                "helloworld".to_string(),
                13,
                DataSectionType::ReadWrite,
                MemoryDataType::I64,
            ),
        ];

        let (items, names_data) = ImportDataSection::convert_from_entries(&entries);
        let section = ImportDataSection {
            items: &items,
            name_paths_data: &names_data,
        };

        assert_eq!(
            section
                .get_item_name_path_and_import_module_index_and_data_section_type_and_memory_data_type(
                    0
                ),
            ("foobar", 11, DataSectionType::ReadOnly, MemoryDataType::I32)
        );

        assert_eq!(
            section
                .get_item_name_path_and_import_module_index_and_data_section_type_and_memory_data_type(
                    1
                ),
            (
                "helloworld",
                13,
                DataSectionType::ReadWrite,
                MemoryDataType::I64
            )
        );
    }
}
