// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use std::fmt::Debug;

use anc_isa::{
    DataSectionType, EffectiveVersion, ExternalLibraryDependency, MemoryDataType, ModuleDependency,
    OperandDataType, SELF_REFERENCE_MODULE_NAME,
};
use serde::{Deserialize, Serialize};

use crate::{
    bytecode_reader::format_bytecode_as_text,
    module_image::{ImageType, RelocateType, Visibility},
};

// Represents the type signature of a function or block, including parameters and results.
#[derive(Debug, PartialEq, Clone)]
pub struct TypeEntry {
    pub params: Vec<OperandDataType>,
    pub results: Vec<OperandDataType>,
}

impl TypeEntry {
    pub fn new(params: Vec<OperandDataType>, results: Vec<OperandDataType>) -> Self {
        Self { params, results }
    }
}

// Represents a list of local variables for a function or block.
#[derive(Debug, PartialEq, Clone)]
pub struct LocalVariableListEntry {
    pub local_variable_entries: Vec<LocalVariableEntry>,
}

impl LocalVariableListEntry {
    pub fn new(local_variable_entries: Vec<LocalVariableEntry>) -> Self {
        Self {
            local_variable_entries,
        }
    }
}

// Represents a single local variable, including its type, length, and alignment.
#[derive(Debug, PartialEq, Clone)]
pub struct LocalVariableEntry {
    pub memory_data_type: MemoryDataType,
    pub length: u32, // Actual length of the variable/data in bytes.
    pub align: u16,  // Alignment requirement in bytes.
}

impl LocalVariableEntry {
    pub fn from_i32() -> Self {
        Self {
            memory_data_type: MemoryDataType::I32,
            length: 4,
            align: 4,
        }
    }

    pub fn from_i64() -> Self {
        Self {
            memory_data_type: MemoryDataType::I64,
            length: 8,
            align: 8,
        }
    }

    pub fn from_f32() -> Self {
        Self {
            memory_data_type: MemoryDataType::F32,
            length: 4,
            align: 4,
        }
    }

    pub fn from_f64() -> Self {
        Self {
            memory_data_type: MemoryDataType::F64,
            length: 8,
            align: 8,
        }
    }

    pub fn from_bytes(length: u32, align: u16) -> Self {
        Self {
            memory_data_type: MemoryDataType::Bytes,
            length,
            align,
        }
    }
}

// Represents a function entry, including its type index, local variable list index, and bytecode.
#[derive(PartialEq)]
pub struct FunctionEntry {
    pub type_index: usize,
    pub local_variable_list_index: usize,
    pub code: Vec<u8>, // Bytecode of the function.
}

impl FunctionEntry {
    pub fn new(type_index: usize, local_variable_list_index: usize, code: Vec<u8>) -> Self {
        Self {
            type_index,
            local_variable_list_index,
            code,
        }
    }
}

impl Debug for FunctionEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FunctionEntry")
            .field("type_index", &self.type_index)
            .field("local_variable_list_index", &self.local_variable_list_index)
            .field("code", &format_bytecode_as_text(&self.code))
            .finish()
    }
}

// Represents initialized data, including its type, content, length, and alignment.
#[derive(Debug, PartialEq, Clone)]
pub struct InitedDataEntry {
    pub memory_data_type: MemoryDataType,
    pub data: Vec<u8>, // Raw data bytes.
    pub length: u32,   // Length of the data in bytes.
    pub align: u16,    // Alignment requirement in bytes.
}

impl InitedDataEntry {
    /// Note that 'i32' in function name means a 32-bit integer, which is equivalent to
    /// the 'uint32_t' in C or 'u32' in Rust. Do not confuse it with 'i32' in Rust.
    /// The same applies to the i8, i16, and i64.
    pub fn from_i32(value: u32) -> Self {
        let mut data: Vec<u8> = Vec::with_capacity(8);
        data.extend(value.to_le_bytes().iter());

        Self {
            memory_data_type: MemoryDataType::I32,
            data,
            length: 4,
            align: 4,
        }
    }

    pub fn from_i64(value: u64) -> Self {
        let mut data: Vec<u8> = Vec::with_capacity(8);
        data.extend(value.to_le_bytes().iter());

        Self {
            memory_data_type: MemoryDataType::I64,
            data,
            length: 8,
            align: 8,
        }
    }

    pub fn from_f32(value: f32) -> Self {
        let mut data: Vec<u8> = Vec::with_capacity(8);
        data.extend(value.to_le_bytes().iter());

        Self {
            memory_data_type: MemoryDataType::F32,
            data,
            length: 4,
            align: 4,
        }
    }

    pub fn from_f64(value: f64) -> Self {
        let mut data: Vec<u8> = Vec::with_capacity(8);
        data.extend(value.to_le_bytes().iter());

        Self {
            memory_data_type: MemoryDataType::F64,
            data,
            length: 8,
            align: 8,
        }
    }

    pub fn from_bytes(data: Vec<u8>, align: u16) -> Self {
        let length = data.len() as u32;

        Self {
            memory_data_type: MemoryDataType::Bytes,
            data,
            length,
            align,
        }
    }
}

// Represents uninitialized data, including its type, length, and alignment.
#[derive(Debug, PartialEq, Clone)]
pub struct UninitDataEntry {
    pub memory_data_type: MemoryDataType,
    pub length: u32, // Length of the data in bytes.
    pub align: u16,  // Alignment requirement in bytes.
}

impl UninitDataEntry {
    pub fn from_i32() -> Self {
        Self {
            memory_data_type: MemoryDataType::I32,
            length: 4,
            align: 4,
        }
    }

    pub fn from_i64() -> Self {
        Self {
            memory_data_type: MemoryDataType::I64,
            length: 8,
            align: 8,
        }
    }

    pub fn from_f32() -> Self {
        Self {
            memory_data_type: MemoryDataType::F32,
            length: 4,
            align: 4,
        }
    }

    pub fn from_f64() -> Self {
        Self {
            memory_data_type: MemoryDataType::F64,
            length: 8,
            align: 8,
        }
    }

    pub fn from_bytes(length: u32, align: u16) -> Self {
        Self {
            memory_data_type: MemoryDataType::Bytes,
            length,
            align,
        }
    }
}

// Represents an external library dependency, including its name and dependency details.
#[derive(Debug, PartialEq, Clone)]
pub struct ExternalLibraryEntry {
    pub name: String,
    pub value: Box<ExternalLibraryDependency>,
}

impl ExternalLibraryEntry {
    pub fn new(name: String, value: Box<ExternalLibraryDependency>) -> Self {
        Self { name, value }
    }
}

// Represents an external function dependency, including its name, library index, and type index.
#[derive(Debug, PartialEq, Clone)]
pub struct ExternalFunctionEntry {
    pub name: String,
    pub external_library_index: usize,
    pub type_index: usize,
}

impl ExternalFunctionEntry {
    pub fn new(name: String, external_library_index: usize, type_index: usize) -> Self {
        Self {
            name,
            external_library_index,
            type_index,
        }
    }
}

// About the "name", "full_name" and "name_path"
// ---------------------------------------------
// - "full_name" = "module_name::name_path"
// - "name_path" = "namespace::identifier"
// - "namespace" = "sub_module_name"{0,N}
//
// e.g.
// the name path of function "add" in submodule "myapp:utils" is "utils::add",
// and the full name is "myapp::utils::add"

// Represents a module dependency, including its name and dependency details.
#[derive(Debug, PartialEq, Clone)]
pub struct ImportModuleEntry {
    // Note that this is the name of module/package,
    // it CANNOT be the name of submodule (i.e. namespace) even if the current image is
    // a "object module", it also CANNOT be the full name or name path.
    pub name: String,
    pub module_dependency: Box<ModuleDependency>,
}

impl ImportModuleEntry {
    pub fn new(name: String, module_dependency: Box<ModuleDependency>) -> Self {
        Self {
            name,
            module_dependency,
        }
    }

    pub fn self_reference_entry() -> Self {
        Self {
            name: SELF_REFERENCE_MODULE_NAME.to_owned(),
            module_dependency: Box::new(ModuleDependency::Module),
        }
    }
}

// Represents a dynamically linked module, including its name and location.
#[derive(Debug, PartialEq, Clone)]
pub struct DynamicLinkModuleEntry {
    // Note that this is the name of module/package,
    // it CANNOT be the name of submodule (i.e. namespace) even if the current image is
    // a "object module", it also CANNOT be the full name or name path.
    pub name: String,

    pub module_location: Box<ModuleLocation>,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[serde(rename = "location")]
pub enum ModuleLocation {
    #[serde(rename = "local")]
    Local(Box<ModuleLocationLocal>),

    #[serde(rename = "remote")]
    Remote(Box<ModuleLocationRemote>),

    #[serde(rename = "share")]
    Share(Box<ModuleLocationShare>),

    #[serde(rename = "runtime")]
    Runtime,

    /// By default, the application's module file (*.ancm) is merged
    /// into the application image file (*.anci) as the first module of all
    /// dependent modules for simplification.
    Embed,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[serde(rename = "local")]
pub struct ModuleLocationLocal {
    // The module path (it is an absolute path).
    pub module_path: String,
    pub hash: String,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[serde(rename = "remote")]
pub struct ModuleLocationRemote {
    pub hash: String,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[serde(rename = "share")]
pub struct ModuleLocationShare {
    pub version: String,
    pub hash: String,
}

impl DynamicLinkModuleEntry {
    pub fn new(name: String, module_location: Box<ModuleLocation>) -> Self {
        Self {
            name,
            module_location,
        }
    }
}

// Represents a function imported from another module, including its full name, module index, and type index.
#[derive(Debug, PartialEq, Clone)]
pub struct ImportFunctionEntry {
    pub full_name: String, // Full name of the imported function (e.g., "module_name::namespace::identifier").
    pub import_module_index: usize,
    pub type_index: usize, // Used for validation during linking.
}

impl ImportFunctionEntry {
    pub fn new(full_name: String, import_module_index: usize, type_index: usize) -> Self {
        Self {
            full_name,
            import_module_index,
            type_index,
        }
    }
}

// Represents data imported from another module, including its full name, module index, and type details.
#[derive(Debug, PartialEq, Clone)]
pub struct ImportDataEntry {
    pub full_name: String, // Full name of the imported data (e.g., "module_name::namespace::identifier").
    pub import_module_index: usize,
    pub data_section_type: DataSectionType, // For validation during linking.
    pub memory_data_type: MemoryDataType,   // For validation during linking.
}

impl ImportDataEntry {
    pub fn new(
        full_name: String,
        import_module_index: usize,
        data_section_type: DataSectionType,
        memory_data_type: MemoryDataType,
    ) -> Self {
        Self {
            full_name,
            import_module_index,
            data_section_type,
            memory_data_type,
        }
    }
}

// Represents a function exported from the module, including its full name and visibility.
#[derive(Debug, PartialEq, Clone)]
pub struct ExportFunctionEntry {
    pub full_name: String, // Full name of the exported function (e.g., "module_name::namespace::identifier").
    pub visibility: Visibility,
}

impl ExportFunctionEntry {
    pub fn new(full_name: String, visibility: Visibility) -> Self {
        Self {
            full_name,
            visibility,
        }
    }
}

// Represents data exported from the module, including its full name, visibility, and section type.
#[derive(Debug, PartialEq, Clone)]
pub struct ExportDataEntry {
    pub full_name: String, // Full name of the exported data (e.g., "module_name::namespace::identifier").
    pub visibility: Visibility,
    pub section_type: DataSectionType,
}

impl ExportDataEntry {
    pub fn new(full_name: String, visibility: Visibility, section_type: DataSectionType) -> Self {
        Self {
            full_name,
            visibility,
            section_type,
        }
    }
}

// Represents a list of relocation entries for a module.
#[derive(Debug, PartialEq, Clone)]
pub struct RelocateListEntry {
    pub relocate_entries: Vec<RelocateEntry>,
}

impl RelocateListEntry {
    pub fn new(relocate_entries: Vec<RelocateEntry>) -> Self {
        Self { relocate_entries }
    }
}

// Represents a single relocation entry, including its offset and relocation type.
#[derive(Debug, PartialEq, Clone)]
pub struct RelocateEntry {
    pub offset_in_function: usize, // Offset in one function bytecode area.
    pub relocate_type: RelocateType, // Type of relocation (e.g., function index, data index).
}

// About re-locating
// -----------------
//
// Certain indices in the instructions need to be re-mapped (re-located) during the linking process.
//
// ## `type_index` and `local_variable_list_index`
//
// These indices are used in the following instructions:
//
// - block                   (param type_index:i32, local_variable_list_index:i32) NO_RETURN
// - block_alt               (param type_index:i32, local_variable_list_index:i32, next_inst_offset:i32) NO_RETURN
// - block_nez               (param local_variable_list_index:i32, next_inst_offset:i32) NO_RETURN
//
// ## `function_public_index`
//
// These indices are used to reference public functions:
//
// - call                    (param function_public_index:i32) (operand args...) -> (values)
// - get_function            (param function_public_index:i32) -> i32
// - host_addr_function      (param function_public_index:i32) -> i64
//
// ## `external_function_index`
//
// These indices are used to reference external functions:
//
// - extcall                 (param external_function_index:i32) (operand args...) -> return_value:void/i32/i64/f32/f64
//
// ## `data_public_index`
//
// These indices are used to reference public data:
//
// - get_data                (param data_public_index:i32) -> i32
// - data_load_*             (param offset_bytes:i16 data_public_index:i32) -> i64
// - data_store_*            (param offset_bytes:i16 data_public_index:i32) (operand value:i64) -> (remain_values)
// - host_addr_data          (param offset_bytes:i16 data_public_index:i32) -> i64
// - data_load_extend_*      (param data_public_index:i32) (operand offset_bytes:i64) -> i64
// - data_store_extend_*     (param data_public_index:i32) (operand offset_bytes:i64 value:i64) -> (remain_values)
// - host_addr_data_extend   (param data_public_index:i32) (operand offset_bytes:i64) -> i64
//
impl RelocateEntry {
    pub fn new(offset_in_function: usize, relocate_type: RelocateType) -> Self {
        Self {
            offset_in_function,
            relocate_type,
        }
    }

    // For instructions:
    // - data_load_*
    // - data_store_*
    // - host_addr_data
    // - data_load_extend_*
    // - data_store_extend_*
    // - host_addr_data_extend
    pub fn from_data_public_index(inst_addr: usize) -> Self {
        RelocateEntry::new(inst_addr + 4, RelocateType::DataPublicIndex)
    }

    // For instructions:
    // - call
    // - get_function
    // - host_addr_function
    pub fn from_function_public_index(inst_addr: usize) -> Self {
        RelocateEntry::new(inst_addr + 4, RelocateType::FunctionPublicIndex)
    }

    // For instruction:
    // - extcall
    pub fn from_external_function_index(inst_addr: usize) -> Self {
        RelocateEntry::new(inst_addr + 4, RelocateType::ExternalFunctionIndex)
    }

    // For instructions:
    // - block
    // - block_alt
    pub fn from_block_with_type_and_local_variables(inst_addr: usize) -> Vec<Self> {
        vec![
            RelocateEntry::new(inst_addr + 4, RelocateType::TypeIndex),
            RelocateEntry::new(inst_addr + 8, RelocateType::LocalVariableListIndex),
        ]
    }

    // For instruction:
    // - block_nez
    pub fn from_block_with_local_variables(inst_addr: usize) -> Self {
        RelocateEntry::new(inst_addr + 4, RelocateType::LocalVariableListIndex)
    }
}

#[derive(Debug, PartialEq)]
pub struct FunctionIndexEntry {
    pub target_module_index: usize,
    pub function_internal_index: usize,
}

impl FunctionIndexEntry {
    pub fn new(target_module_index: usize, function_internal_index: usize) -> Self {
        Self {
            target_module_index,
            function_internal_index,
        }
    }
}

/// FunctionIndexListEntry per Module
#[derive(Debug, PartialEq)]
pub struct FunctionIndexListEntry {
    pub index_entries: Vec<FunctionIndexEntry>,
}

impl FunctionIndexListEntry {
    pub fn new(index_entries: Vec<FunctionIndexEntry>) -> Self {
        Self { index_entries }
    }
}

#[derive(Debug, PartialEq)]
pub struct DataIndexEntry {
    pub target_module_index: usize,
    pub data_internal_index: usize,
    pub target_data_section_type: DataSectionType,
}

impl DataIndexEntry {
    pub fn new(
        target_module_index: usize,
        data_internal_index: usize,
        target_data_section_type: DataSectionType,
    ) -> Self {
        Self {
            target_module_index,
            data_internal_index,
            target_data_section_type,
        }
    }
}

/// DataIndexListEntry per Module
#[derive(Debug, PartialEq)]
pub struct DataIndexListEntry {
    pub index_entries: Vec<DataIndexEntry>,
}

impl DataIndexListEntry {
    pub fn new(index_entries: Vec<DataIndexEntry>) -> Self {
        Self { index_entries }
    }
}

#[derive(Debug, PartialEq)]
pub struct ExternalFunctionIndexListEntry {
    pub index_entries: Vec<ExternalFunctionIndexEntry>,
}

impl ExternalFunctionIndexListEntry {
    pub fn new(index_entries: Vec<ExternalFunctionIndexEntry>) -> Self {
        Self { index_entries }
    }
}

#[derive(Debug, PartialEq)]
pub struct ExternalFunctionIndexEntry {
    pub unified_external_function_index: usize,
}

impl ExternalFunctionIndexEntry {
    pub fn new(unified_external_function_index: usize) -> Self {
        Self {
            unified_external_function_index,
        }
    }
}

/// Internal Entry Point Names
/// --------------------------
///
/// This section describes the naming conventions and execution behavior of internal entry points.
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
#[derive(Debug, PartialEq)]
pub struct EntryPointEntry {
    pub unit_name: String, // Internal name of the entry point.
    pub function_public_index: usize,
}

impl EntryPointEntry {
    pub fn new(unit_name: String, function_public_index: usize) -> Self {
        Self {
            unit_name,
            function_public_index,
        }
    }
}

// Represents common properties of the module image, including its name, version, and type.
#[derive(Debug)]
pub struct ImageCommonEntry {
    // The name of module/package,
    //
    // Note: It CANNOT be the name of submodule (i.e. namespace) even if the current image is
    // a "object module", it also CANNOT be the full name or name path.
    //
    // Note that only [a-zA-Z0-9_] and unicode chars are allowed for the name of (sub)module(/source file).
    pub name: String,
    pub version: EffectiveVersion,
    pub image_type: ImageType,

    pub type_entries: Vec<TypeEntry>,
    pub local_variable_list_entries: Vec<LocalVariableListEntry>,
    pub function_entries: Vec<FunctionEntry>,

    pub read_only_data_entries: Vec<InitedDataEntry>,
    pub read_write_data_entries: Vec<InitedDataEntry>,
    pub uninit_data_entries: Vec<UninitDataEntry>,

    // The dependencies
    pub import_module_entries: Vec<ImportModuleEntry>,

    // The following entries are used for linking:
    // - import_function_entries
    // - import_data_entries
    // - export_function_entries
    // - export_data_entries
    pub import_function_entries: Vec<ImportFunctionEntry>,
    pub import_data_entries: Vec<ImportDataEntry>,

    // The name path entries only contain the internal functions.
    pub export_function_entries: Vec<ExportFunctionEntry>,

    // The name path entries only contain the internal data items.
    pub export_data_entries: Vec<ExportDataEntry>,

    pub relocate_list_entries: Vec<RelocateListEntry>,

    // The dependencies
    pub external_library_entries: Vec<ExternalLibraryEntry>,
    pub external_function_entries: Vec<ExternalFunctionEntry>,
}

#[derive(Debug)]
pub struct ImageIndexEntry {
    pub function_index_list_entries: Vec<FunctionIndexListEntry>,
    pub data_index_list_entries: Vec<DataIndexListEntry>,
    //
    pub external_function_index_entries: Vec<ExternalFunctionIndexListEntry>,
    //
    pub unified_external_library_entries: Vec<ExternalLibraryEntry>,
    pub unified_external_type_entries: Vec<TypeEntry>,
    pub unified_external_function_entries: Vec<ExternalFunctionEntry>,
    //
    pub dynamic_link_module_entries: Vec<DynamicLinkModuleEntry>,
    pub entry_point_entries: Vec<EntryPointEntry>,
}
