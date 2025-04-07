// Copyright (c) 2025 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions.
// For more details, see the LICENSE, LICENSE.additional, and CONTRIBUTING files.

// "Entry Point Section" binary layout:
//
//              |-----------------------------------------------------|
//              | item count (u32) | extra header length (u32)        |
//              |-----------------------------------------------------|
//  item 0 -->  | unit name offset 0 (u32) | unit name length 0 (u32) |
//              | fn public index 0 (u32)                             | <-- table
//  item 1 -->  | unit name offset 1       | unit name length 1       |
//              | fn public index 1                                   |
//              | ...                                                 |
//              |-----------------------------------------------------|
// offset 0 --> | unit name string 0 (UTF-8)                          | <-- data
// offset 1 --> | unit name string 1                                  |
//              | ...                                                 |
//              |-----------------------------------------------------|

use crate::{
    datatableaccess::{
        read_section_with_table_and_data_area, write_section_with_table_and_data_area,
    },
    entry::EntryPointEntry,
    module_image::{ModuleSectionId, SectionEntry},
};

#[derive(Debug, PartialEq, Default)]
pub struct EntryPointSection<'a> {
    /// A slice of entry point items representing the table.
    pub items: &'a [EntryPointItem],
    /// A slice of UTF-8 encoded unit name strings representing the data area.
    pub unit_names_data: &'a [u8],
}

/// Internal Entry Point Naming Conventions and Execution Behavior
/// --------------------------------------------------------------
///
/// - **Default Entry Point**:
///   - Internal Name: `_start`
///   - Executes Function: `{app_module_name}::_start`
///   - User CLI Unit Name: `""` (empty string)
///
/// - **Additional Executable Units**:
///   - Internal Name: `{submodule_name}`
///   - Executes Function: `{app_module_name}::app::{submodule_name}::_start`
///   - User CLI Unit Name: `:{submodule_name}`
///
/// - **Unit Tests**:
///   - Internal Name: `{submodule_name}::test_*`
///   - Executes Function: `{app_module_name}::tests::{submodule_name}::test_*`
///   - User CLI Unit Name: Name path prefix, e.g., `{submodule_name}`, `{submodule_name}::test_get_`
#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct EntryPointItem {
    /// Offset of the unit name string in the data area.
    pub unit_name_offset: u32,
    /// Length of the unit name string.
    pub unit_name_length: u32,
    /// Public index of the function to be executed.
    ///
    /// The module index is omitted because entry points always exist in the main module.
    pub function_public_index: u32,
}

impl EntryPointItem {
    /// Creates a new `EntryPointItem`.
    pub fn new(unit_name_offset: u32, unit_name_length: u32, function_public_index: u32) -> Self {
        Self {
            unit_name_offset,
            unit_name_length,
            function_public_index,
        }
    }
}

impl<'a> SectionEntry<'a> for EntryPointSection<'a> {
    /// Reads an `EntryPointSection` from the given section data.
    fn read(section_data: &'a [u8]) -> Self {
        let (items, unit_names_data) =
            read_section_with_table_and_data_area::<EntryPointItem>(section_data);
        EntryPointSection {
            items,
            unit_names_data,
        }
    }

    /// Writes the `EntryPointSection` to the provided writer.
    fn write(&'a self, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        write_section_with_table_and_data_area(self.items, self.unit_names_data, writer)
    }

    /// Returns the section ID for the entry point section.
    fn id(&'a self) -> ModuleSectionId {
        ModuleSectionId::EntryPoint
    }
}

impl<'a> EntryPointSection<'a> {
    /// Retrieves the public index of the function corresponding to the given unit name.
    pub fn get_function_public_index(&'a self, expected_unit_name: &str) -> Option<usize> {
        let items = self.items;
        let unit_names_data = self.unit_names_data;

        let expected_unit_name_data = expected_unit_name.as_bytes();

        let opt_idx = items.iter().position(|item| {
            let unit_name_data = &unit_names_data[item.unit_name_offset as usize
                ..(item.unit_name_offset + item.unit_name_length) as usize];
            unit_name_data == expected_unit_name_data
        });

        opt_idx.map(|idx| items[idx].function_public_index as usize)
    }

    /// Converts the section into a vector of `EntryPointEntry` objects.
    pub fn convert_to_entries(&self) -> Vec<EntryPointEntry> {
        let items = self.items;
        let unit_names_data = self.unit_names_data;

        items
            .iter()
            .map(|item| {
                let unit_name_data = &unit_names_data[item.unit_name_offset as usize
                    ..(item.unit_name_offset + item.unit_name_length) as usize];

                let unit_name = std::str::from_utf8(unit_name_data).unwrap().to_owned();
                EntryPointEntry::new(unit_name, item.function_public_index as usize)
            })
            .collect()
    }

    /// Converts a vector of `EntryPointEntry` objects into section data.
    pub fn convert_from_entries(entries: &[EntryPointEntry]) -> (Vec<EntryPointItem>, Vec<u8>) {
        let unit_name_bytes = entries
            .iter()
            .map(|entry| entry.unit_name.as_bytes())
            .collect::<Vec<&[u8]>>();

        let mut next_offset: u32 = 0;

        let items = entries
            .iter()
            .enumerate()
            .map(|(idx, entry)| {
                let unit_name_offset = next_offset;
                let unit_name_length = unit_name_bytes[idx].len() as u32;
                next_offset += unit_name_length; // Update offset for the next entry.

                EntryPointItem::new(
                    unit_name_offset,
                    unit_name_length,
                    entry.function_public_index as u32,
                )
            })
            .collect::<Vec<EntryPointItem>>();

        let unit_names_data = unit_name_bytes
            .iter()
            .flat_map(|bytes| bytes.to_vec())
            .collect::<Vec<u8>>();

        (items, unit_names_data)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        entry::EntryPointEntry,
        linking_sections::entry_point_section::{EntryPointItem, EntryPointSection},
        module_image::SectionEntry,
    };

    #[test]
    fn test_write_section() {
        let items: Vec<EntryPointItem> = vec![
            EntryPointItem::new(0, 6, 11),
            EntryPointItem::new(6, 3, 13),
            EntryPointItem::new(9, 5, 17),
        ];

        let section = EntryPointSection {
            items: &items,
            unit_names_data: "_startfoohello".as_bytes(),
        };

        let mut section_data: Vec<u8> = vec![];
        section.write(&mut section_data).unwrap();

        let mut expect_data = vec![
            3u8, 0, 0, 0, // Number of items.
            0, 0, 0, 0, // Extra section header length (u32).
            //
            0, 0, 0, 0, // Name offset (item 0).
            6, 0, 0, 0, // Name length.
            11, 0, 0, 0, // Function public index.
            //
            6, 0, 0, 0, // Name offset (item 1).
            3, 0, 0, 0, // Name length.
            13, 0, 0, 0, // Function public index.
            //
            9, 0, 0, 0, // Name offset (item 2).
            5, 0, 0, 0, // Name length.
            17, 0, 0, 0, // Function public index.
        ];

        expect_data.extend_from_slice(b"_start");
        expect_data.extend_from_slice(b"foo");
        expect_data.extend_from_slice(b"hello");
        expect_data.extend_from_slice(b"\0\0"); // Section 4-byte alignment.

        assert_eq!(section_data, expect_data);
    }

    #[test]
    fn test_read_section() {
        let mut section_data = vec![
            3u8, 0, 0, 0, // Number of items.
            0, 0, 0, 0, // Extra section header length (u32).
            //
            0, 0, 0, 0, // Name offset (item 0).
            6, 0, 0, 0, // Name length.
            11, 0, 0, 0, // Function public index.
            //
            6, 0, 0, 0, // Name offset (item 1).
            3, 0, 0, 0, // Name length.
            13, 0, 0, 0, // Function public index.
            //
            9, 0, 0, 0, // Name offset (item 2).
            5, 0, 0, 0, // Name length.
            17, 0, 0, 0, // Function public index.
        ];

        section_data.extend_from_slice("_start".as_bytes());
        section_data.extend_from_slice("foo".as_bytes());
        section_data.extend_from_slice("hello".as_bytes());

        let section = EntryPointSection::read(&section_data);

        assert_eq!(section.items.len(), 3);
        assert_eq!(section.items[0], EntryPointItem::new(0, 6, 11));
        assert_eq!(section.items[1], EntryPointItem::new(6, 3, 13));
        assert_eq!(section.items[2], EntryPointItem::new(9, 5, 17));
        assert_eq!(section.unit_names_data, "_startfoohello".as_bytes())
    }

    #[test]
    fn test_convert() {
        let entries: Vec<EntryPointEntry> = vec![
            EntryPointEntry::new("_start".to_string(), 11),
            EntryPointEntry::new("foo".to_string(), 13),
            EntryPointEntry::new("hello".to_string(), 15),
        ];

        let (items, names_data) = EntryPointSection::convert_from_entries(&entries);
        let section = EntryPointSection {
            items: &items,
            unit_names_data: &names_data,
        };

        assert_eq!(section.get_function_public_index("_start"), Some(11));
        assert_eq!(section.get_function_public_index("foo"), Some(13));
        assert_eq!(section.get_function_public_index("hello"), Some(15));

        assert!(section.get_function_public_index("bar").is_none());

        let entries_restore = section.convert_to_entries();
        assert_eq!(entries, entries_restore);
    }
}
