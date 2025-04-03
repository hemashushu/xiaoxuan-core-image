// Copyright (c) 2025 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions.
// For more details, see the LICENSE, LICENSE.additional, and CONTRIBUTING files.

// "Import Data Section" binary layout:
//
//              |--------------------------------------------------------------------------------------------------------------------------------------|
//              | item count (u32) | extra header length (u32)                                                                                         |
//              |--------------------------------------------------------------------------------------------------------------------------------------|
//  item 0 -->  | full name off 0 (u32) | full name len 0 (u32) | import module idx 0 (u32) | dat sec type 0 (u8) | mem data type 0 (u8) | pad 2 bytes | <-- table
//  item 1 -->  | full name off 1       | full name len 1       | import module idx 1       | dat sec type 1                                           |
//              | ...                                                                                                                                  |
//              |--------------------------------------------------------------------------------------------------------------------------------------|
// offset 0 --> | full name string 0 (UTF-8)                                                                                                           | <-- data area
// offset 1 --> | full name string 1                                                                                                                   |
//              | ...                                                                                                                                  |
//              |--------------------------------------------------------------------------------------------------------------------------------------|

use anc_isa::{DataSectionType, MemoryDataType};

use crate::{
    datatableaccess::{
        read_section_with_table_and_data_area, write_section_with_table_and_data_area,
    },
    entry::ImportDataEntry,
    module_image::{ModuleSectionId, SectionEntry},
};

#[derive(Debug, PartialEq, Default)]
pub struct ImportDataSection<'a> {
    pub items: &'a [ImportDataItem],
    pub full_names_data: &'a [u8],
}

#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct ImportDataItem {
    // Defination of the "full_name":
    // -------------------------------------
    // - "full_name" = "module_name::name_path"
    // - "name_path" = "namespace::identifier"
    // - "namespace" = "sub_module_name"{0,N}
    //
    // Example:
    // For a data item "config" in submodule "myapp::settings", the name path is "settings::config",
    // and the full name is "myapp::settings::config".
    pub full_name_offset: u32, // Offset of the full name string in the data area
    pub full_name_length: u32, // Length (in bytes) of the full name string in the data area
    pub import_module_index: u32, // Index of the import module
    pub data_section_type: DataSectionType, // Type of the data section
    pub memory_data_type: MemoryDataType, // Type of the memory data
    _padding0: [u8; 2],        // Padding for alignment
}

impl ImportDataItem {
    pub fn new(
        full_name_offset: u32,
        full_name_length: u32,
        import_module_index: u32,
        data_section_type: DataSectionType,
        memory_data_type: MemoryDataType,
    ) -> Self {
        Self {
            full_name_offset,
            full_name_length,
            import_module_index,
            data_section_type,
            memory_data_type,
            _padding0: [0; 2],
        }
    }
}

impl<'a> SectionEntry<'a> for ImportDataSection<'a> {
    fn read(section_data: &'a [u8]) -> Self {
        let (items, full_names_data) =
            read_section_with_table_and_data_area::<ImportDataItem>(section_data);
        ImportDataSection {
            items,
            full_names_data,
        }
    }

    fn write(&'a self, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        write_section_with_table_and_data_area(self.items, self.full_names_data, writer)
    }

    fn id(&'a self) -> ModuleSectionId {
        ModuleSectionId::ImportData
    }
}

impl<'a> ImportDataSection<'a> {
    /// Retrieves the full name, import module index, data section type, and memory data type of an item at the specified index.
    pub fn get_item_full_name_and_import_module_index_and_data_section_type_and_memory_data_type(
        &'a self,
        idx: usize,
    ) -> (&'a str, usize, DataSectionType, MemoryDataType) {
        let items = self.items;
        let full_names_data = self.full_names_data;

        let item = &items[idx];
        let full_name_data = &full_names_data[item.full_name_offset as usize
            ..(item.full_name_offset + item.full_name_length) as usize];

        (
            std::str::from_utf8(full_name_data).unwrap(),
            item.import_module_index as usize,
            item.data_section_type,
            item.memory_data_type,
        )
    }

    /// Converts the section into a vector of `ImportDataEntry` objects.
    pub fn convert_to_entries(&self) -> Vec<ImportDataEntry> {
        let items = self.items;
        let full_names_data = self.full_names_data;

        items
            .iter()
            .map(|item| {
                let full_name_data = &full_names_data[item.full_name_offset as usize
                    ..(item.full_name_offset + item.full_name_length) as usize];
                let full_name = std::str::from_utf8(full_name_data).unwrap().to_owned();
                ImportDataEntry::new(
                    full_name,
                    item.import_module_index as usize,
                    item.data_section_type,
                    item.memory_data_type,
                )
            })
            .collect()
    }

    /// Converts a vector of `ImportDataEntry` objects into the section's internal representation.
    pub fn convert_from_entries(entries: &[ImportDataEntry]) -> (Vec<ImportDataItem>, Vec<u8>) {
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

                ImportDataItem::new(
                    full_name_offset,
                    full_name_length,
                    entry.import_module_index as u32,
                    entry.data_section_type,
                    entry.memory_data_type,
                )
            })
            .collect::<Vec<ImportDataItem>>();

        let full_names_data = full_name_bytes
            .iter()
            .flat_map(|bytes| bytes.to_vec())
            .collect::<Vec<u8>>();

        (items, full_names_data)
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
    fn test_read_section() {
        let mut section_data = vec![
            2u8, 0, 0, 0, // item count
            0, 0, 0, 0, // extra section header len (i32)
            //
            0, 0, 0, 0, // name offset (item 0)
            3, 0, 0, 0, // name length
            11, 0, 0, 0, // import module index
            0, // data section type
            0, // mem data type
            0, 0, // padding
            //
            3, 0, 0, 0, // name offset (item 1)
            5, 0, 0, 0, // name length
            13, 0, 0, 0, // import module index
            1, // data section type
            1, // mem data type
            0, 0, // padding
        ];

        section_data.extend_from_slice(b"foo");
        section_data.extend_from_slice(b"hello");

        let section = ImportDataSection::read(&section_data);

        assert_eq!(section.items.len(), 2);
        assert_eq!(
            section.items[0],
            ImportDataItem::new(0, 3, 11, DataSectionType::ReadOnly, MemoryDataType::I32,)
        );
        assert_eq!(
            section.items[1],
            ImportDataItem::new(3, 5, 13, DataSectionType::ReadWrite, MemoryDataType::I64,)
        );
        assert_eq!(section.full_names_data, "foohello".as_bytes())
    }

    #[test]
    fn test_write_section() {
        let items = vec![
            ImportDataItem::new(0, 3, 11, DataSectionType::ReadOnly, MemoryDataType::I32),
            ImportDataItem::new(3, 5, 13, DataSectionType::ReadWrite, MemoryDataType::I64),
        ];

        let section = ImportDataSection {
            items: &items,
            full_names_data: b"foohello",
        };

        let mut section_data: Vec<u8> = vec![];
        section.write(&mut section_data).unwrap();

        let mut expect_data = vec![
            2u8, 0, 0, 0, // item count
            0, 0, 0, 0, // extra section header len (i32)
            //
            0, 0, 0, 0, // name offset (item 0)
            3, 0, 0, 0, // name length
            11, 0, 0, 0, // import module index
            0, // data section type
            0, // mem data type
            0, 0, // padding
            //
            3, 0, 0, 0, // name offset (item 1)
            5, 0, 0, 0, // name length
            13, 0, 0, 0, // import module index
            1, // data section type
            1, // mem data type
            0, 0, // padding
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
            full_names_data: &names_data,
        };

        assert_eq!(
            section
                .get_item_full_name_and_import_module_index_and_data_section_type_and_memory_data_type(
                    0
                ),
            ("foobar", 11, DataSectionType::ReadOnly, MemoryDataType::I32)
        );

        assert_eq!(
            section
                .get_item_full_name_and_import_module_index_and_data_section_type_and_memory_data_type(
                    1
                ),
            (
                "helloworld",
                13,
                DataSectionType::ReadWrite,
                MemoryDataType::I64
            )
        );

        let entries_restore = section.convert_to_entries();
        assert_eq!(entries, entries_restore);
    }
}
