// Copyright (c) 2025 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions.
// For more details, see the LICENSE, LICENSE.additional, and CONTRIBUTING files.

// A module consists of two main parts: data and code (instructions), divided into several sections:
//
// - Type Section: Contains function signatures (used for functions, blocks, and external functions).
// - Local Variables Section: Defines local variables for functions or blocks.
// - Function Section: Contains the bytecode of functions.
// - Data Sections: Includes three types:
//   - Read-Only Data: Immutable data.
//   - Read-Write Data: Mutable data, cloned for each thread.
//   - Uninitialized Data: Memory allocated but not initialized.
// - Import/Export Sections: Define imported and exported functions and data.
// - Relocation Section: Contains relocation information for linking.
// - External Library/Function Sections: Define external dependencies.
// - Property Section: Contains metadata about the module.
//
// A minimal module requires only the following sections:
// - Type Section
// - Local Variables Section
// - Function Section
// - Property Section
//
// Optional sections include:
// - Data Sections (Read-Only, Read-Write, Uninitialized)
// - Import/Export Sections (for linking and debugging)
// - Relocation Section (for linking)
// - External Library/Function Sections (for linking)
//
// Applications consist of one or more modules. When linked, all imports are resolved, and additional sections are created:
// - Function Index Section
// - Entry Point Section
// - Dynamic Link Module Section
//
// Optional sections for applications include:
// - Data Index Section
// - Unified External Library/Function/Type Sections
// - External Function Index Section

// The binary layout of a module image file:
//
// Header:
//
// |-------------------------------------------------------------|
// | Magic Number (u64)                                          | 8 bytes, offset=0
// |-------------------------------------------------------------|
// | Image Type (u16)        | Extra Header Length (u16)         | 4 bytes, offset=8
// | Image Format Minor Ver (u16) | Image Format Major Ver (u16) | 4 bytes, offset=12
// |-------------------------------------------------------------|
//
// Base Header Length = 16 bytes
//
// Body:
//
// |------------------------------------------------------|
// | Section Item Count (u32) | Extra Header Length (u32) | 8 bytes, offset=16
// |------------------------------------------------------|
// | Section ID 0 (u32) | Offset 0 (u32) | Length 0 (u32) | <-- Table
// | Section ID 1       | Offset 1       | Length 1       |
// | ...                                                  |
// |------------------------------------------------------|
// | Section Data 0                                       | <-- Data
// | Section Data 1                                       |
// | ...                                                  |
// |------------------------------------------------------|

use anc_isa::{IMAGE_FORMAT_MAJOR_VERSION, IMAGE_FORMAT_MINOR_VERSION};

use crate::{
    common_sections::{
        data_name_section::DataNameSection, external_function_section::ExternalFunctionSection,
        external_library_section::ExternalLibrarySection,
        function_name_section::FunctionNameSection, function_section::FunctionSection,
        import_data_section::ImportDataSection, import_function_section::ImportFunctionSection,
        import_module_section::ImportModuleSection, local_variable_section::LocalVariableSection,
        property_section::PropertySection, read_only_data_section::ReadOnlyDataSection,
        read_write_data_section::ReadWriteDataSection, relocate_section::RelocateSection,
        type_section::TypeSection, uninit_data_section::UninitDataSection,
    },
    datatableaccess::{
        read_section_with_table_and_data_area, write_section_with_table_and_data_area,
    },
    linking_sections::{
        data_index_section::DataIndexSection, entry_point_section::EntryPointSection,
        external_function_index_section::ExternalFunctionIndexSection,
        function_index_section::FunctionIndexSection,
        linking_module_section::LinkingModuleSection,
        unified_external_function_section::UnifiedExternalFunctionSection,
        unified_external_library_section::UnifiedExternalLibrarySection,
        unified_external_type_section::UnifiedExternalTypeSection,
    },
    ImageError, ImageErrorType,
};

// Each record in the table must be multiple of this value.
// Note: The image file consists of many sections, each section is actually a table,
// and each table consists of many records. This alignment ensures that
// each record is aligned to the boundary.
pub const TABLE_RECORD_ALIGN_BYTES: usize = 4;

// Each data item (i.e., items in ".rodata", ".data" and ".bss") must be multiple of this value.
pub const DATA_ITEM_ALIGN_BYTES: usize = 8;

// The magic number in the image file header, "ancmod" stands for the "XiaoXuan Core Module".
pub const IMAGE_FILE_MAGIC_NUMBER: &[u8; 8] = b"ancmod\0\0";

pub const BASE_MODULE_HEADER_LENGTH: usize = 16;
pub const BASE_SECTION_HEADER_LENGTH: usize = 8;

// Represents a module image, including its type, section items, and section data.
#[derive(Debug, PartialEq)]
pub struct ModuleImage<'a> {
    pub image_type: ImageType, // Type of the image (e.g., Application, SharedModule, ObjectFile).
    pub items: &'a [ModuleSectionItem], // Section metadata.
    pub sections_data: &'a [u8], // Raw section data.
}

// Represents a single section item in the module, including its ID, offset, and length.
#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct ModuleSectionItem {
    pub id: ModuleSectionId, // Section ID (e.g., Type, Function, Data).
    pub offset: u32,         // Offset of the section data in bytes.
    pub length: u32,         // Length of the section data in bytes.
}

impl ModuleSectionItem {
    pub fn new(id: ModuleSectionId, offset: u32, length: u32) -> Self {
        Self { id, offset, length }
    }
}

// Represents the ID of a module section.
#[repr(u32)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ModuleSectionId {
    // Essential sections
    Property = 0x0010, // Metadata about the module.
    Type,              // Function signatures.
    LocalVariable,     // Local variables for functions or blocks.
    Function,          // Function bytecode.

    // Optional sections
    ReadOnlyData = 0x0020, // Immutable data.
    ReadWriteData,         // Mutable data.
    UninitData,            // Uninitialized data.

    // Optional sections for linking and debugging
    FunctionName = 0x0030, // Exported functions.
    DataName,              // Exported data.
    Relocate,              // Relocation information.

    // Optional sections for linking
    ImportModule = 0x0040, // Imported modules.
    ImportFunction,        // Imported functions.
    ImportData,            // Imported data.
    ExternalLibrary,       // External libraries.
    ExternalFunction,      // External functions.

    // Essential sections for applications
    EntryPoint = 0x0080, // Entry points.
    FunctionIndex,       // Function index mapping.
    LinkingModule,       // Dynamically linked modules.

    // Optional sections for applications
    DataIndex = 0x0090,           // Data index mapping.
    UnifiedExternalType = 0x00a0, // Unified external types.
    UnifiedExternalLibrary,       // Unified external libraries.
    UnifiedExternalFunction,      // Unified external functions.
    ExternalFunctionIndex,        // Mapping of external functions to unified external functions.
}

// Represents the type of a module image (e.g., Application, SharedModule, ObjectFile).
#[repr(u16)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ImageType {
    Application,  // `*.anca`
    SharedModule, // `*.ancm`
    ObjectFile,   // `*.anco`
}

// Represents the visibility of functions and data between shared modules.
#[repr(u8)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Visibility {
    Private, // Accessible only within the same module.
    Public,  // Accessible across different modules.
}

// Represents the type of relocation required for linking.
#[repr(u8)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum RelocateType {
    TypeIndex,              // Relocation for type indices.
    LocalVariableListIndex, // Relocation for local variable list indices.
    FunctionPublicIndex,    // Relocation for public function indices.
    ExternalFunctionIndex,  // Relocation for external function indices.
    DataPublicIndex,        // Relocation for public data indices.
}

// `RangeItem` is used for data index section and function index section.
//
// Note that one range item per module, e.g., consider the following items:
//
// module 0 ----- index item 0
//            |-- index item 1
//            |-- index item 2
//
// module 1 ----- index item 3
//            |-- index item 4
//
// Since there are 2 modules, there will be
// 2 range items as the following:
//
// range 0 = {offset:0, count:3}
// range 1 = {offset:3, count:2}
//
// Use the C style struct memory layout.
// See also:
// https://doc.rust-lang.org/reference/type-layout.html#reprc-structs
#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct RangeItem {
    pub offset: u32,
    pub count: u32,
}

impl RangeItem {
    pub fn new(offset: u32, count: u32) -> Self {
        Self { offset, count }
    }
}

pub trait SectionEntry<'a> {
    fn id(&'a self) -> ModuleSectionId;
    fn read(section_data: &'a [u8]) -> Self
    where
        Self: Sized;
    fn write(&'a self, writer: &mut dyn std::io::Write) -> std::io::Result<()>;
}

impl<'a> ModuleImage<'a> {
    pub fn read(image_binary: &'a [u8]) -> Result<Self, ImageError> {
        let magic_slice = &image_binary[0..8];
        if magic_slice != IMAGE_FILE_MAGIC_NUMBER {
            return Err(ImageError::new(ImageErrorType::InvalidImage));
        }

        let ptr = image_binary.as_ptr();

        let ptr_image_type = unsafe { ptr.offset(8) };
        let image_type = unsafe { std::ptr::read(ptr_image_type as *const ImageType) };

        let ptr_extra_header_length = unsafe { ptr.offset(10) };
        let extra_header_length = unsafe { std::ptr::read(ptr_extra_header_length as *const u16) };

        let ptr_declared_module_format_image_version = unsafe { ptr.offset(12) };
        let declared_module_image_version =
            unsafe { std::ptr::read(ptr_declared_module_format_image_version as *const u32) };

        let supported_module_format_image_version =
            ((IMAGE_FORMAT_MAJOR_VERSION as u32) << 16) | (IMAGE_FORMAT_MINOR_VERSION as u32);
        if declared_module_image_version > supported_module_format_image_version {
            return Err(ImageError::new(ImageErrorType::RequireNewVersionRuntime));
        }

        let image_body =
            &image_binary[(BASE_MODULE_HEADER_LENGTH + extra_header_length as usize)..];

        let (items, sections_data) =
            read_section_with_table_and_data_area::<ModuleSectionItem>(image_body);

        Ok(Self {
            image_type,
            items,
            sections_data,
        })
    }

    pub fn write(&'a self, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        const EXTRA_HEADER_LENGTH: u16 = 0;

        writer.write_all(IMAGE_FILE_MAGIC_NUMBER)?;
        writer.write_all(&(self.image_type as u16).to_le_bytes())?;
        writer.write_all(&EXTRA_HEADER_LENGTH.to_le_bytes())?;
        writer.write_all(&IMAGE_FORMAT_MINOR_VERSION.to_le_bytes())?;
        writer.write_all(&IMAGE_FORMAT_MAJOR_VERSION.to_le_bytes())?;

        write_section_with_table_and_data_area(self.items, self.sections_data, writer)
    }

    pub fn convert_from_section_entries(
        entries: &[&'a dyn SectionEntry<'a>],
    ) -> (Vec<ModuleSectionItem>, Vec<u8>) {
        let mut image_binary: Vec<u8> = vec![];

        let mut data_increment_lengths: Vec<usize> = vec![];

        for entry in entries {
            entry.write(&mut image_binary).unwrap();
            data_increment_lengths.push(image_binary.len());
        }

        let mut offsets: Vec<usize> = vec![0];
        offsets.extend(data_increment_lengths.iter());
        offsets.pop();

        let lengths = data_increment_lengths
            .iter()
            .zip(offsets.iter())
            .map(|(next, current)| next - current)
            .collect::<Vec<usize>>();

        let items = entries
            .iter()
            .zip(offsets.iter().zip(lengths.iter()))
            .map(|(entry, (offset, length))| {
                ModuleSectionItem::new(entry.id(), *offset as u32, *length as u32)
            })
            .collect::<Vec<ModuleSectionItem>>();

        (items, image_binary)
    }

    pub fn get_section_index_by_id(&'a self, section_id: ModuleSectionId) -> Option<usize> {
        self.items.iter().enumerate().find_map(|(idx, item)| {
            if item.id == section_id {
                Some(idx)
            } else {
                None
            }
        })
    }

    fn get_section_data_by_id(&'a self, section_id: ModuleSectionId) -> Option<&'a [u8]> {
        self.items.iter().find_map(|item| {
            if item.id == section_id {
                let data =
                    &self.sections_data[item.offset as usize..(item.offset + item.length) as usize];
                Some(data)
            } else {
                None
            }
        })
    }

    pub fn get_property_section(&'a self) -> PropertySection {
        self.get_section_data_by_id(ModuleSectionId::Property)
            .map_or_else(
                || panic!("Cannot find the common property section."),
                PropertySection::read,
            )
    }

    pub fn get_type_section(&'a self) -> TypeSection<'a> {
        self.get_section_data_by_id(ModuleSectionId::Type)
            .map_or_else(
                || panic!("Cannot find the type section."),
                TypeSection::read,
            )
    }

    pub fn get_local_variable_section(&'a self) -> LocalVariableSection<'a> {
        self.get_section_data_by_id(ModuleSectionId::LocalVariable)
            .map_or_else(
                || panic!("Cannot find the local variable section."),
                LocalVariableSection::read,
            )
    }

    pub fn get_function_section(&'a self) -> FunctionSection<'a> {
        self.get_section_data_by_id(ModuleSectionId::Function)
            .map_or_else(
                || panic!("Cannot find the function section."),
                FunctionSection::read,
            )
    }

    pub fn get_entry_point_section(&'a self) -> EntryPointSection<'a> {
        self.get_section_data_by_id(ModuleSectionId::EntryPoint)
            .map_or_else(
                || panic!("Cannot find the entry point section."),
                EntryPointSection::read,
            )
    }

    pub fn get_dynamic_link_module_list_section(&'a self) -> LinkingModuleSection<'a> {
        self.get_section_data_by_id(ModuleSectionId::LinkingModule)
            .map_or_else(
                || panic!("Cannot find the index property section."),
                LinkingModuleSection::read,
            )
    }

    pub fn get_function_index_section(&'a self) -> FunctionIndexSection<'a> {
        self.get_section_data_by_id(ModuleSectionId::FunctionIndex)
            .map_or_else(
                || panic!("Cannot find the function index section."),
                FunctionIndexSection::read,
            )
    }

    pub fn get_optional_read_only_data_section(&'a self) -> Option<ReadOnlyDataSection<'a>> {
        self.get_section_data_by_id(ModuleSectionId::ReadOnlyData)
            .map(ReadOnlyDataSection::read)
    }

    pub fn get_optional_read_write_data_section(&'a self) -> Option<ReadWriteDataSection<'a>> {
        self.get_section_data_by_id(ModuleSectionId::ReadWriteData)
            .map(ReadWriteDataSection::read)
    }

    pub fn get_optional_uninit_data_section(&'a self) -> Option<UninitDataSection<'a>> {
        self.get_section_data_by_id(ModuleSectionId::UninitData)
            .map(UninitDataSection::read)
    }

    pub fn get_optional_export_function_section(&'a self) -> Option<FunctionNameSection<'a>> {
        self.get_section_data_by_id(ModuleSectionId::FunctionName)
            .map(FunctionNameSection::read)
    }

    pub fn get_optional_export_data_section(&'a self) -> Option<DataNameSection<'a>> {
        self.get_section_data_by_id(ModuleSectionId::DataName)
            .map(DataNameSection::read)
    }

    pub fn get_optional_relocate_section(&'a self) -> Option<RelocateSection<'a>> {
        self.get_section_data_by_id(ModuleSectionId::Relocate)
            .map(RelocateSection::read)
    }

    pub fn get_optional_import_module_section(&'a self) -> Option<ImportModuleSection<'a>> {
        self.get_section_data_by_id(ModuleSectionId::ImportModule)
            .map(ImportModuleSection::read)
    }

    pub fn get_optional_import_function_section(&'a self) -> Option<ImportFunctionSection<'a>> {
        self.get_section_data_by_id(ModuleSectionId::ImportFunction)
            .map(ImportFunctionSection::read)
    }

    pub fn get_optional_import_data_section(&'a self) -> Option<ImportDataSection<'a>> {
        self.get_section_data_by_id(ModuleSectionId::ImportData)
            .map(ImportDataSection::read)
    }

    pub fn get_optional_external_library_section(&'a self) -> Option<ExternalLibrarySection<'a>> {
        self.get_section_data_by_id(ModuleSectionId::ExternalLibrary)
            .map(ExternalLibrarySection::read)
    }

    pub fn get_optional_external_function_section(&'a self) -> Option<ExternalFunctionSection<'a>> {
        self.get_section_data_by_id(ModuleSectionId::ExternalFunction)
            .map(ExternalFunctionSection::read)
    }

    pub fn get_optional_data_index_section(&'a self) -> Option<DataIndexSection<'a>> {
        self.get_section_data_by_id(ModuleSectionId::DataIndex)
            .map(DataIndexSection::read)
    }

    pub fn get_optional_unified_external_type_section(
        &'a self,
    ) -> Option<UnifiedExternalTypeSection<'a>> {
        self.get_section_data_by_id(ModuleSectionId::UnifiedExternalType)
            .map(UnifiedExternalTypeSection::read)
    }

    pub fn get_optional_unified_external_library_section(
        &'a self,
    ) -> Option<UnifiedExternalLibrarySection<'a>> {
        self.get_section_data_by_id(ModuleSectionId::UnifiedExternalLibrary)
            .map(UnifiedExternalLibrarySection::read)
    }

    pub fn get_optional_unified_external_function_section(
        &'a self,
    ) -> Option<UnifiedExternalFunctionSection<'a>> {
        self.get_section_data_by_id(ModuleSectionId::UnifiedExternalFunction)
            .map(UnifiedExternalFunctionSection::read)
    }

    pub fn get_optional_external_function_index_section(
        &'a self,
    ) -> Option<ExternalFunctionIndexSection<'a>> {
        self.get_section_data_by_id(ModuleSectionId::ExternalFunctionIndex)
            .map(ExternalFunctionIndexSection::read)
    }
}

#[cfg(test)]
mod tests {
    use anc_isa::{MemoryDataType, OperandDataType, RUNTIME_EDITION};

    use crate::{
        common_sections::{
            local_variable_section::{LocalVariableItem, LocalVariableSection},
            property_section::PropertySection,
            type_section::TypeSection,
        },
        entry::{LocalVariableEntry, LocalVariableListEntry, TypeEntry},
        module_image::{
            ImageType, ModuleImage, SectionEntry, BASE_MODULE_HEADER_LENGTH,
            IMAGE_FILE_MAGIC_NUMBER,
        },
    };

    #[test]
    fn test_module_image_read_and_write() {
        let property_section =
            PropertySection::new("bar", *RUNTIME_EDITION, 7, 11, 13 /* 17, 19 */);

        let type_entries = vec![
            TypeEntry {
                params: vec![OperandDataType::I32, OperandDataType::I64],
                results: vec![OperandDataType::F32],
            },
            TypeEntry {
                params: vec![],
                results: vec![OperandDataType::F64],
            },
        ];

        let (type_items, types_data) = TypeSection::convert_from_entries(&type_entries);
        let type_section = TypeSection {
            items: &type_items,
            types_data: &types_data,
        };

        let local_variable_list_entries = vec![
            LocalVariableListEntry::new(vec![
                LocalVariableEntry::from_i32(),
                LocalVariableEntry::from_i64(),
            ]),
            LocalVariableListEntry::new(vec![LocalVariableEntry::from_bytes(12, 4)]),
        ];

        let (local_variable_lists, local_variable_list_data) =
            LocalVariableSection::convert_from_entries(&local_variable_list_entries);
        let local_variable_section = LocalVariableSection {
            lists: &local_variable_lists,
            list_data: &local_variable_list_data,
        };

        let section_entries: Vec<&dyn SectionEntry> =
            vec![&type_section, &local_variable_section, &property_section];

        let (section_items, sections_data) =
            ModuleImage::convert_from_section_entries(&section_entries);
        let module_image = ModuleImage {
            image_type: ImageType::ObjectFile,
            items: &section_items,
            sections_data: &sections_data,
        };

        let mut image_binary: Vec<u8> = vec![];
        module_image.write(&mut image_binary).unwrap();

        assert_eq!(&image_binary[0..8], IMAGE_FILE_MAGIC_NUMBER);
        assert_eq!(&image_binary[8..10], &[2, 0]);
        assert_eq!(&image_binary[10..12], &[0, 0]);
        assert_eq!(&image_binary[12..14], &[0, 0]);
        assert_eq!(&image_binary[14..16], &[1, 0]);

        let extra_header_length: u16 =
            u16::from_le_bytes((&image_binary[10..12]).try_into().unwrap());
        let remains = &image_binary[(BASE_MODULE_HEADER_LENGTH + extra_header_length as usize)..];

        let (section_count_data, remains) = remains.split_at(8);
        assert_eq!(&section_count_data[0..4], &[3, 0, 0, 0]);
        assert_eq!(&section_count_data[4..8], &[0, 0, 0, 0]);

        let (section_table_data, remains) = remains.split_at(36);

        // section table
        assert_eq!(
            section_table_data,
            &[
                0x11u8, 0, 0, 0, // section id, type section
                0, 0, 0, 0, // offset: 0
                36, 0, 0, 0, // length: header 8 + rec 12 * 2 + data 4
                //
                0x12, 0, 0, 0, // section id, local variable section
                36, 0, 0, 0, // offset: 36
                68, 0, 0, 0, // length: header 8 + rec 12 * 2 + data 12 * 3
                //
                0x10, 0, 0, 0, // section id, common property section
                104, 0, 0, 0, // offset: 104
                20, 1, 0, 0 // length: prop 20 + name 256
            ]
        );

        // type section

        let (type_section_data, remains) = remains.split_at(36);
        assert_eq!(
            type_section_data,
            &[
                2u8, 0, 0, 0, // item count
                0, 0, 0, 0, // padding
                //
                2, 0, // param len 0
                1, 0, // result len 0
                0, 0, 0, 0, // param offset 0
                2, 0, 0, 0, // result offset 0
                //
                0, 0, // param len 1
                1, 0, // result len 1
                3, 0, 0, 0, // param offset 1
                3, 0, 0, 0, // result offset 1
                //
                0, // I32
                1, // I64
                2, // F32
                3, // F64
            ]
        );

        // local variable list section

        let (local_variable_section_data, remains) = remains.split_at(68);
        assert_eq!(
            local_variable_section_data,
            &[
                // header
                2, 0, 0, 0, // item count
                0, 0, 0, 0, // extra section header len (i32)
                // table
                0, 0, 0, 0, // offset
                2, 0, 0, 0, // count
                16, 0, 0, 0, // alloc bytes
                //
                24, 0, 0, 0, // offset (2 items * 12 bytes/item)
                1, 0, 0, 0, // count
                16, 0, 0, 0, // alloc bytes
                //
                // data
                //
                // list 0
                0, 0, 0, 0, // var offset (i32)
                4, 0, 0, 0, // var len
                0, // data type
                0, // padding
                4, 0, // align
                //
                8, 0, 0, 0, // var offset (i64)
                8, 0, 0, 0, // var len
                1, // data type
                0, // padding
                8, 0, // align
                //
                // list 1
                0, 0, 0, 0, // var offset
                12, 0, 0, 0, // var len
                4, // data type
                0, // padding
                4, 0, // align
            ]
        );

        // common property section
        let mut expected_property_section_data = vec![];
        expected_property_section_data.append(&mut RUNTIME_EDITION.to_vec());
        expected_property_section_data.append(&mut vec![
            7, 0, // version patch
            11, 0, // version minor
            13, 0, // version major
            0, 0, // version padding
            //
            /*
            17, 0, 0, 0, // import_data_count
            19, 0, 0, 0, // import_function_count
             */
            //
            3, 0, 0, 0, // name length
        ]);

        assert_eq!(&remains[..20], &expected_property_section_data);

        let module_image_restore = ModuleImage::read(&image_binary).unwrap();
        assert_eq!(module_image_restore.items.len(), 3);
        assert_eq!(module_image_restore.image_type, ImageType::ObjectFile);

        // check type section

        let type_section_restore = module_image_restore.get_type_section();
        assert_eq!(type_section_restore.items.len(), 2);

        assert_eq!(
            type_section_restore.get_item_params_and_results(0),
            (
                vec![OperandDataType::I32, OperandDataType::I64].as_ref(),
                vec![OperandDataType::F32].as_ref(),
            )
        );

        assert_eq!(
            type_section_restore.get_item_params_and_results(1),
            ([].as_ref(), vec![OperandDataType::F64].as_ref(),)
        );

        // check local variable list section

        let local_variable_section_restore = module_image_restore.get_local_variable_section();
        assert_eq!(local_variable_section_restore.lists.len(), 2);

        assert_eq!(
            local_variable_section_restore.get_local_variable_list(0),
            &[
                LocalVariableItem::new(0, 4, MemoryDataType::I32, 4),
                LocalVariableItem::new(8, 8, MemoryDataType::I64, 8),
            ]
        );

        assert_eq!(
            local_variable_section_restore.get_local_variable_list(1),
            &[LocalVariableItem::new(0, 12, MemoryDataType::Bytes, 4),]
        );

        // check common property section

        let property_section_restore = module_image_restore.get_property_section();
        assert_eq!(property_section_restore.version_patch, 7);
        assert_eq!(property_section_restore.version_minor, 11);
        assert_eq!(property_section_restore.version_major, 13);
        // assert_eq!(property_section_restore.import_data_count, 17);
        // assert_eq!(property_section_restore.import_function_count, 19);
        assert_eq!(property_section_restore.get_module_name(), "bar");
    }
}
