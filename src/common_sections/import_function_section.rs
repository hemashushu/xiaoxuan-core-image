// Copyright (c) 2025 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions.
// For more details, see the LICENSE, LICENSE.additional, and CONTRIBUTING files.

// Note that the order of imported functions are unrelated to
// the "function internal index" or the "imported module index".

// "Import Function Section" binary layout:
//
//              |-----------------------------------------------------|
//              | item count (u32) | extra header length (u32)        |
//              |-----------------------------------------------------|
//  item 0 -->  | full name offset 0 (u32) | full name length 0 (u32) |
//              | import module idx 0 (u32) | type index 0 (u32)      | <-- table
//  item 1 -->  | full name offset 1       | full name length 1       |
//              | import module idx 1       | type index 1            |
//              | ...                                                 |
//              |-----------------------------------------------------|
// offset 0 --> | full name string 0 (UTF-8)                          | <-- data
// offset 1 --> | full name string 1                                  |
//              | ...                                                 |
//              |-----------------------------------------------------|

use crate::{
    datatableaccess::{
        read_section_with_table_and_data_area, write_section_with_table_and_data_area,
    },
    entry::ImportFunctionEntry,
    module_image::{ModuleSectionId, SectionEntry},
};

#[derive(Debug, PartialEq, Default)]
pub struct ImportFunctionSection<'a> {
    pub items: &'a [ImportFunctionItem],
    pub full_names_data: &'a [u8],
}

#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct ImportFunctionItem {
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
    pub full_name_offset: u32, // Offset of the full name string in the data area
    pub full_name_length: u32, // Length (in bytes) of the full name string in the data area
    pub import_module_index: u32, // Index of the import module
    pub type_index: u32,       // Index of the function type
}

impl ImportFunctionItem {
    pub fn new(
        full_name_offset: u32,
        full_name_length: u32,
        import_module_index: u32,
        type_index: u32,
    ) -> Self {
        Self {
            full_name_offset,
            full_name_length,
            import_module_index,
            type_index,
        }
    }
}

impl<'a> SectionEntry<'a> for ImportFunctionSection<'a> {
    fn read(section_data: &'a [u8]) -> Self {
        let (items, full_names_data) =
            read_section_with_table_and_data_area::<ImportFunctionItem>(section_data);
        ImportFunctionSection {
            items,
            full_names_data,
        }
    }

    fn write(&'a self, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        write_section_with_table_and_data_area(self.items, self.full_names_data, writer)
    }

    fn id(&'a self) -> ModuleSectionId {
        ModuleSectionId::ImportFunction
    }
}

impl<'a> ImportFunctionSection<'a> {
    /// Retrieves the full name, import module index, and type index of an item at the specified index.
    pub fn get_item_full_name_and_import_module_index_and_type_index(
        &'a self,
        idx: usize,
    ) -> (&'a str, usize, usize) {
        let items = self.items;
        let full_names_data = self.full_names_data;

        let item = &items[idx];
        let full_name_data = &full_names_data[item.full_name_offset as usize
            ..(item.full_name_offset + item.full_name_length) as usize];

        (
            std::str::from_utf8(full_name_data).unwrap(),
            item.import_module_index as usize,
            item.type_index as usize,
        )
    }

    /// Converts the section into a vector of `ImportFunctionEntry` objects.
    pub fn convert_to_entries(&self) -> Vec<ImportFunctionEntry> {
        let items = self.items;
        let full_names_data = self.full_names_data;

        items
            .iter()
            .map(|item| {
                let full_name_data = &full_names_data[item.full_name_offset as usize
                    ..(item.full_name_offset + item.full_name_length) as usize];
                let full_name = std::str::from_utf8(full_name_data).unwrap().to_owned();
                ImportFunctionEntry::new(
                    full_name,
                    item.import_module_index as usize,
                    item.type_index as usize,
                )
            })
            .collect()
    }

    /// Converts a vector of `ImportFunctionEntry` objects into the section's internal representation.
    pub fn convert_from_entries(
        entries: &[ImportFunctionEntry],
    ) -> (Vec<ImportFunctionItem>, Vec<u8>) {
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

                ImportFunctionItem::new(
                    full_name_offset,
                    full_name_length,
                    entry.import_module_index as u32,
                    entry.type_index as u32,
                )
            })
            .collect::<Vec<ImportFunctionItem>>();

        let full_names_data = full_name_bytes
            .iter()
            .flat_map(|bytes| bytes.to_vec())
            .collect::<Vec<u8>>();

        (items, full_names_data)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        common_sections::import_function_section::{ImportFunctionItem, ImportFunctionSection},
        entry::ImportFunctionEntry,
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
            13, 0, 0, 0, // type index
            //
            3, 0, 0, 0, // name offset (item 1)
            5, 0, 0, 0, // name length
            15, 0, 0, 0, // import module index
            17, 0, 0, 0, // type index
        ];

        section_data.extend_from_slice(b"foo");
        section_data.extend_from_slice(b"hello");

        let section = ImportFunctionSection::read(&section_data);

        assert_eq!(section.items.len(), 2);
        assert_eq!(section.items[0], ImportFunctionItem::new(0, 3, 11, 13,));
        assert_eq!(section.items[1], ImportFunctionItem::new(3, 5, 15, 17));
        assert_eq!(section.full_names_data, "foohello".as_bytes())
    }

    #[test]
    fn test_write_section() {
        let items = vec![
            ImportFunctionItem::new(0, 3, 11, 13),
            ImportFunctionItem::new(3, 5, 15, 17),
        ];

        let section = ImportFunctionSection {
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
            13, 0, 0, 0, // type index
            //
            3, 0, 0, 0, // name offset (item 1)
            5, 0, 0, 0, // name length
            15, 0, 0, 0, // import module index
            17, 0, 0, 0, // type index
        ];

        expect_data.extend_from_slice(b"foo");
        expect_data.extend_from_slice(b"hello");

        assert_eq!(section_data, expect_data);
    }

    #[test]
    fn test_convert() {
        let entries = vec![
            ImportFunctionEntry::new("foobar".to_string(), 17, 19),
            ImportFunctionEntry::new("helloworld".to_string(), 23, 29),
        ];

        let (items, names_data) = ImportFunctionSection::convert_from_entries(&entries);
        let section = ImportFunctionSection {
            items: &items,
            full_names_data: &names_data,
        };

        assert_eq!(
            section.get_item_full_name_and_import_module_index_and_type_index(0),
            ("foobar", 17, 19)
        );

        assert_eq!(
            section.get_item_full_name_and_import_module_index_and_type_index(1),
            ("helloworld", 23, 29)
        );

        let entries_restore = section.convert_to_entries();
        assert_eq!(entries, entries_restore);
    }
}
