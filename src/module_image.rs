// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

// a module consists of two parts, data and code (i.e., instructions), which
// are divided into several sections:
//
// - type section
//   the signature of a function, the types are also applied to the code blocks and external functions.
// - local variables section
//   a function is consists of a type, a list of local variables, and instructions
// - function section
// - data sections
//   there are 3 types of data sections:
//   - read-only
//   - read-write
//   - uninit(uninitialized)
//   all data is thread-local, so the read-write section will be cloned and the
//   uninitialized section will be reallocated when a new thread is created.
// - import module section
// - import function section
// - import data section
// - export function section
// - export data section
// - relocate section
// - external library section
// - external function section
// - property section
//
// a minimal module needs only 4 sections:
//
// - type section
// - local variable section
// - function section
// - property section
//
// data sections are optional:
//
// - read-only data section
// - read-write data section
// - uninitialized data section
//
// other sections are not needed at the runtime,
// they are used for debugging and linking:
//
// - export function section
// - export data section
// - relocate section
// - import module section
// - import function section
// - import data section
// - external library section
// - external function section
//
// note that if the 'bridge function feature' is enable, the
// export function section and the export data section are required.

// an application consists of one or more modules,
// when the main module and other modules are linked,
// all import data and functions are resolved and
// stored in the following sections:
//
// - function index section
// - entry point section
// - module list section
//
// there are also some optional sections:
//
// - data index section
// - external function index section
// - unified external library section
// - unified external function section


// the design of the module
// ------------------------
//
// loading and starting XiaoXuan Core modules is extremely fast, because:
// - there is no parsing process and copying overhead, the load process actually
//   does only two things: maps the module image file into memory, and
//   locates the start and end positions of each section.
// - the instructions are executed directly on the bytecode.
//
// this allows the XiaoXuan Core applications to have almost no startup delay.
//
// since the XiaoXuan Core application starts almost instantly, it is suitable for
// use as a 'function' in other applications.

// the data type of section fields
// -------------------------------
//
// - u8
//   data type, data section type, module share type
// - u16
//   'local variables' and 'data' store/load offset, data align,
//   block break/recur skip depth, params count, results count
// - u32
//   section id, syscall number, env call number,
//   module index, function type index, data index, local (variable list) index,
//   function index, dynamic function index, external function index

use anc_isa::{IMAGE_FORMAT_MAJOR_VERSION, IMAGE_FORMAT_MINOR_VERSION};

use crate::{
    common_sections::{
        data_section::{ReadOnlyDataSection, ReadWriteDataSection, UninitDataSection},
        export_data_section::ExportDataSection,
        export_function_section::ExportFunctionSection,
        external_function_section::ExternalFunctionSection,
        external_library_section::ExternalLibrarySection,
        function_section::FunctionSection,
        import_data_section::ImportDataSection,
        import_function_section::ImportFunctionSection,
        import_module_section::ImportModuleSection,
        local_variable_section::LocalVariableSection,
        property_section::PropertySection,
        relocate_section::RelocateSection,
        type_section::TypeSection,
    },
    index_sections::{
        data_index_section::DataIndexSection, entry_point_section::EntryPointSection,
        external_function_index_section::ExternalFunctionIndexSection,
        external_function_section::UnifiedExternalFunctionSection,
        external_library_section::UnifiedExternalLibrarySection,
        external_type_section::UnifiedExternalTypeSection,
        function_index_section::FunctionIndexSection, module_list_section::ModuleListSection,
    },
    tableaccess::{read_section_with_table_and_data_area, write_section_with_table_and_data_area},
    ImageError, ImageErrorType,
};

// the "module image file" binary layout:
//
//                 header
//              |---------------------------------------------------|
//              | magic number (u64)                                | 8 bytes, off=0
//              |---------------------------------------------------|
//              | image type (u16)        | extra header len (u16)  | 4 bytes, off=8
//              | img fmt minor ver (u16) | img fmt major ver (u16) | 4 bytes, off=12
//              |---------------------------------------------------|
//                 base header length = 16 bytes

//                 body
//              |------------------------------------------------------|
//              | section item count (u32) | extra header length (u32) | 8 bytes, off=16
//              |------------------------------------------------------|
//   item 0 --> | section id 0 (u32) | offset 0 (u32) | length 0 (u32) | <-- table
//   item 1 --> | section id 1       | offset 1       | length 1       |
//              | ...                                                  |
//              |------------------------------------------------------|
// offset 0 --> | section data 0                                       | <-- data
// offset 1 --> | section data 1                                       |
//              | ...                                                  |
//              |------------------------------------------------------|

pub const DATA_ALIGN_BYTES: usize = 4;
pub const IMAGE_FILE_MAGIC_NUMBER: &[u8; 8] = b"ancmod\0\0"; // stands for the "XiaoXuan Core Module"

pub const BASE_MODULE_HEADER_LENGTH: usize = 16;
pub const BASE_SECTION_HEADER_LENGTH: usize = 8;

#[derive(Debug, PartialEq)]
pub struct ModuleImage<'a> {
    pub image_type: ImageType,
    pub items: &'a [ModuleSectionItem],
    pub sections_data: &'a [u8],
}

#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct ModuleSectionItem {
    pub id: ModuleSectionId, // u32
    pub offset: u32,
    pub length: u32,
}

impl ModuleSectionItem {
    pub fn new(id: ModuleSectionId, offset: u32, length: u32) -> Self {
        Self { id, offset, length }
    }
}

#[repr(u32)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ModuleSectionId {
    // essential
    Property = 0x0010, // 0x10
    Type,              // 0x11
    LocalVariable,     // 0x12
    Function,          // 0x13

    // optional
    ReadOnlyData = 0x0020, // 0x20
    ReadWriteData,         // 0x21
    UninitData,            // 0x22

    // optional (for debug and linking)
    //
    // if the feature 'bridge function' is required (i.e.,
    // embed the XiaoXuan Core VM in another Rust applicaton) ,
    // the section 'ExportFunction' and 'ExportData' are required also.
    ExportFunction = 0x0030, // 0x30
    ExportData,              // 0x31
    Relocate,                // 0x32

    // optional (for debug and linking)
    ImportModule = 0x0040, // 0x40
    ImportFunction,        // 0x41
    ImportData,            // 0x42
    ExternalLibrary,       // 0x43
    ExternalFunction,      // 0x43

    /*
     essential (application only)
    */
    EntryPoint = 0x0080, // 0x80
    FunctionIndex,       // 0x81
    ModuleList,          // 0x82, this section is used by the module loader

    /*
    optional (application only)
    */
    DataIndex = 0x0090, // 0x90

    UnifiedExternalType = 0x00a0, // 0xa0
    UnifiedExternalLibrary,       // 0xa1
    UnifiedExternalFunction,      // 0xa2

    // the section ExternalFunctionIndex is used for mapping
    // 'external function' to 'unified external function'
    ExternalFunctionIndex, // 0xa3
}

#[repr(u16)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ImageType {
    // `*.anca`
    Application,

    // `*.ancm`
    SharedModule,

    // `*.anco`
    ObjectFile,
}

/// The visibility of function and data between shared modules.
#[repr(u8)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Visibility {
    /// Disallows functions and data to be shared between
    /// different modules.
    ///
    /// Note that all functions and data (within different submodules) are accessible
    /// in the same module, even if the value of `Visibility` is `Private`.
    /// In particular, this is also true when merging (statically linking) two modules.
    Private,

    /// Allows functions and data to be shared between
    /// different modules.
    Public,
}

// About re-locating
// -----------------
//
// there are indices in the instructions need to re-locate (re-map) when linking
//
// ## type_index and local_variable_list_index
//
// - block                   (param type_index:i32, local_variable_list_index:i32) NO_RETURN
// - block_alt               (param type_index:i32, local_variable_list_index:i32, next_inst_offset:i32) NO_RETURN
// - block_nez               (param local_variable_list_index:i32, next_inst_offset:i32) NO_RETURN
//
// ## function_public_index
//
// - call                    (param function_public_index:i32) (operand args...) -> (values)
// - get_function            (param function_public_index:i32) -> i32
// - host_addr_function      (param function_public_index:i32) -> i64
//
// ## external_function_index
//
// - extcall                 (param external_function_index:i32) (operand args...) -> return_value:void/i32/i64/f32/f64
//
// ## data_public_index
//
// - data_load_*             (param offset_bytes:i16 data_public_index:i32) -> i64
// - data_store_*            (param offset_bytes:i16 data_public_index:i32) (operand value:i64) -> (remain_values)
// - host_addr_data          (param offset_bytes:i16 data_public_index:i32) -> i64
// - data_load_extend_*      (param data_public_index:i32) (operand offset_bytes:i64) -> i64
// - data_store_extend_*     (param data_public_index:i32) (operand offset_bytes:i64 value:i64) -> (remain_values)
// - host_addr_data_extend   (param data_public_index:i32) (operand offset_bytes:i64) -> i64
//
#[repr(u8)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum RelocateType {
    TypeIndex,
    LocalVariableListIndex,
    FunctionPublicIndex,
    ExternalFunctionIndex,
    DataPublicIndex,
}

// `RangeItem` is used for data index section and function index section
//
// note that one range item per module, e.g., consider the following items:
//
// module 0 ----- index item 0
//            |-- index item 1
//            |-- index item 2
//
// module 1 ----- index item 3
//            |-- index item 4
//
// since there are 2 modules, so there will be
// 2 range items as the following:
//
// range 0 = {offset:0, count:3}
// range 1 = {offset:3, count:2}
//
// use the C style struct memory layout
// see also:
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
    // there is a approach to 'downcast' a section entry to section object, e.g.
    //
    // ```rust
    // fn downcast_section_entry<'a, T>(entry: &'a dyn SectionEntry) -> &'a T {
    //     /* the 'entry' is a fat pointer, it contains (object_pointer, vtable) */
    //     let ptr = entry as *const dyn SectionEntry as *const T; /* get the first part of the fat pointer */
    //     unsafe { &*ptr }
    // }
    // ```

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

        // there is another safe approach for obtaining the version number:
        //
        // ```rust
        //     let version_data: [u8;4] = (&image_binary[4..8]).try_into().unwrap();
        //     let version = u32::from_le_bytes(version_data);
        // ```

        let ptr = image_binary.as_ptr();

        let ptr_image_type = unsafe { ptr.offset(8) };
        let image_type = unsafe { std::ptr::read(ptr_image_type as *const ImageType) };

        let ptr_extra_header_length = unsafe { ptr.offset(10) };
        let extra_header_length = unsafe { std::ptr::read(ptr_extra_header_length as *const u16) };

        let ptr_declared_module_format_image_version = unsafe { ptr.offset(12) };
        let declared_module_image_version =
            unsafe { std::ptr::read(ptr_declared_module_format_image_version as *const u32) };

        let supported_module_format_image_version =
            ((IMAGE_FORMAT_MAJOR_VERSION as u32) << 16) | (IMAGE_FORMAT_MINOR_VERSION as u32); // supported version 1.0
        if declared_module_image_version > supported_module_format_image_version {
            return Err(ImageError::new(ImageErrorType::RequireNewVersionRuntime));
        }

        let image_body =
            &image_binary[(BASE_MODULE_HEADER_LENGTH + extra_header_length as usize)..];

        // since the structure of module image and a section are the same,
        // that is, the module image itself can be thought of
        // as a 'big' section that contains many child sections.
        // so we can load module by reusing function
        // `load_section_with_table_and_data_area` as well.
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

        // write header
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

        // len0, len0+1, len0+1+2..., len total
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

    // essential section
    pub fn get_property_section(&'a self) -> PropertySection {
        self.get_section_data_by_id(ModuleSectionId::Property)
            .map_or_else(
                || panic!("Can not find the common property section."),
                PropertySection::read,
            )
    }

    // essential section
    pub fn get_type_section(&'a self) -> TypeSection<'a> {
        self.get_section_data_by_id(ModuleSectionId::Type)
            .map_or_else(
                || panic!("Can not find the type section."),
                TypeSection::read,
            )
    }

    // essential section
    pub fn get_local_variable_section(&'a self) -> LocalVariableSection<'a> {
        self.get_section_data_by_id(ModuleSectionId::LocalVariable)
            .map_or_else(
                || panic!("Can not find the local variable section."),
                LocalVariableSection::read,
            )
    }

    // essential section
    pub fn get_function_section(&'a self) -> FunctionSection<'a> {
        self.get_section_data_by_id(ModuleSectionId::Function)
            .map_or_else(
                || panic!("Can not find the function section."),
                FunctionSection::read,
            )
    }

    // essential section (application only)
    pub fn get_entry_point_section(&'a self) -> EntryPointSection<'a> {
        self.get_section_data_by_id(ModuleSectionId::EntryPoint)
            .map_or_else(
                || panic!("Can not find the entry point section."),
                EntryPointSection::read,
            )
    }

    // essential section (application only)
    pub fn get_module_list_section(&'a self) -> ModuleListSection<'a> {
        self.get_section_data_by_id(ModuleSectionId::ModuleList)
            .map_or_else(
                || panic!("Can not find the index property section."),
                ModuleListSection::read,
            )
    }

    // essential section (application only)
    pub fn get_function_index_section(&'a self) -> FunctionIndexSection<'a> {
        self.get_section_data_by_id(ModuleSectionId::FunctionIndex)
            .map_or_else(
                || panic!("Can not find the function index section."),
                FunctionIndexSection::read,
            )
    }

    // optional section
    pub fn get_optional_read_only_data_section(&'a self) -> Option<ReadOnlyDataSection<'a>> {
        self.get_section_data_by_id(ModuleSectionId::ReadOnlyData)
            .map(ReadOnlyDataSection::read)
    }

    // optional section
    pub fn get_optional_read_write_data_section(&'a self) -> Option<ReadWriteDataSection<'a>> {
        self.get_section_data_by_id(ModuleSectionId::ReadWriteData)
            .map(ReadWriteDataSection::read)
    }

    // optional section
    pub fn get_optional_uninit_data_section(&'a self) -> Option<UninitDataSection<'a>> {
        self.get_section_data_by_id(ModuleSectionId::UninitData)
            .map(UninitDataSection::read)
    }

    // optional section (for debug, link only and bridge function calling)
    pub fn get_optional_export_function_section(&'a self) -> Option<ExportFunctionSection<'a>> {
        self.get_section_data_by_id(ModuleSectionId::ExportFunction)
            .map(ExportFunctionSection::read)
    }

    // optional section (for debug, link only and bridge function calling)
    pub fn get_optional_export_data_section(&'a self) -> Option<ExportDataSection<'a>> {
        self.get_section_data_by_id(ModuleSectionId::ExportData)
            .map(ExportDataSection::read)
    }

    // optional section (for debug and link only)
    pub fn get_optional_relocate_section(&'a self) -> Option<RelocateSection<'a>> {
        self.get_section_data_by_id(ModuleSectionId::Relocate)
            .map(RelocateSection::read)
    }

    // optional section (for debug and link only)
    pub fn get_optional_import_module_section(&'a self) -> Option<ImportModuleSection<'a>> {
        self.get_section_data_by_id(ModuleSectionId::ImportModule)
            .map(ImportModuleSection::read)
    }

    // optional section (for debug and link only)
    pub fn get_optional_import_function_section(&'a self) -> Option<ImportFunctionSection<'a>> {
        self.get_section_data_by_id(ModuleSectionId::ImportFunction)
            .map(ImportFunctionSection::read)
    }

    // optional section (for debug and link only)
    pub fn get_optional_import_data_section(&'a self) -> Option<ImportDataSection<'a>> {
        self.get_section_data_by_id(ModuleSectionId::ImportData)
            .map(ImportDataSection::read)
    }

    // optional section (for debug and link only)
    pub fn get_optional_external_library_section(&'a self) -> Option<ExternalLibrarySection<'a>> {
        self.get_section_data_by_id(ModuleSectionId::ExternalLibrary)
            .map(ExternalLibrarySection::read)
    }

    // optional section (for debug and link only)
    pub fn get_optional_external_function_section(&'a self) -> Option<ExternalFunctionSection<'a>> {
        self.get_section_data_by_id(ModuleSectionId::ExternalFunction)
            .map(ExternalFunctionSection::read)
    }

    // optional section (application only)
    pub fn get_optional_data_index_section(&'a self) -> Option<DataIndexSection<'a>> {
        self.get_section_data_by_id(ModuleSectionId::DataIndex)
            .map(DataIndexSection::read)
    }

    // optional section (application only)
    pub fn get_optional_unified_external_type_section(
        &'a self,
    ) -> Option<UnifiedExternalTypeSection<'a>> {
        self.get_section_data_by_id(ModuleSectionId::UnifiedExternalType)
            .map(UnifiedExternalTypeSection::read)
    }

    // optional section (application only)
    pub fn get_optional_unified_external_library_section(
        &'a self,
    ) -> Option<UnifiedExternalLibrarySection<'a>> {
        self.get_section_data_by_id(ModuleSectionId::UnifiedExternalLibrary)
            .map(UnifiedExternalLibrarySection::read)
    }

    // optional section (application only)
    pub fn get_optional_unified_external_function_section(
        &'a self,
    ) -> Option<UnifiedExternalFunctionSection<'a>> {
        self.get_section_data_by_id(ModuleSectionId::UnifiedExternalFunction)
            .map(UnifiedExternalFunctionSection::read)
    }

    // optional section (application only)
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
        // build property section
        let property_section = PropertySection::new("bar", 17, 19);

        // build TypeSection instance
        // note: arbitrary types
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

        // build LocalVariableSection instance
        // note: arbitrary local variables
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

        // build ModuleImage instance
        let section_entries: Vec<&dyn SectionEntry> =
            vec![&type_section, &local_variable_section, &property_section];

        let (section_items, sections_data) =
            ModuleImage::convert_from_section_entries(&section_entries);
        let module_image = ModuleImage {
            image_type: ImageType::ObjectFile,
            items: &section_items,
            sections_data: &sections_data,
        };

        // save
        let mut image_binary: Vec<u8> = vec![];
        module_image.write(&mut image_binary).unwrap();

        assert_eq!(&image_binary[0..8], IMAGE_FILE_MAGIC_NUMBER);
        assert_eq!(&image_binary[8..10], &[2, 0]); // image type
        assert_eq!(&image_binary[10..12], &[0, 0]); // extra header length
        assert_eq!(&image_binary[12..14], &[0, 0]); // image format minor version number, little endian
        assert_eq!(&image_binary[14..16], &[1, 0]); // image format major version number, little endian

        // body
        let extra_header_length: u16 =
            u16::from_le_bytes((&image_binary[10..12]).try_into().unwrap());
        let remains = &image_binary[(BASE_MODULE_HEADER_LENGTH + extra_header_length as usize)..];

        // section count
        let (section_count_data, remains) = remains.split_at(8);
        assert_eq!(&section_count_data[0..4], &[3, 0, 0, 0]); // section item count
        assert_eq!(&section_count_data[4..8], &[0, 0, 0, 0]); // padding

        // section table length = 12 (the record length) * 3= 36
        let (section_table_data, remains) = remains.split_at(36);

        // section table
        assert_eq!(
            section_table_data,
            &[
                0x11u8, 0, 0, 0, // section id, type section
                0, 0, 0, 0, // offset 0
                36, 0, 0, 0, // length 0
                //
                0x12, 0, 0, 0, // section id, local variable section
                36, 0, 0, 0, // offset 2
                68, 0, 0, 0, // length 2
                //
                0x10, 0, 0, 0, // section id, common property section
                104, 0, 0, 0, // offset 6,
                20, 1, 0, 0 // length 256 + 20
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
            17, 0, 0, 0, // import_data_count
            19, 0, 0, 0, // import_function_count
            3, 0, 0, 0, // name length
        ]);

        assert_eq!(&remains[..20], &expected_property_section_data);

        // load
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
        assert_eq!(property_section_restore.import_data_count, 17);
        assert_eq!(property_section_restore.import_function_count, 19);

        assert_eq!(property_section_restore.get_module_name(), "bar");
    }
}
