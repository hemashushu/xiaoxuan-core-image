// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

//! Entries are used to simplify the creation and parsing of sections.

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

// both function and block can contains a 'local variables list'
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

#[derive(Debug, PartialEq, Clone)]
pub struct LocalVariableEntry {
    pub memory_data_type: MemoryDataType,

    // actual length of the variable/data
    pub length: u32,

    pub align: u16,
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

#[derive(PartialEq)]
pub struct FunctionEntry {
    pub type_index: usize,
    pub local_variable_list_index: usize,
    pub code: Vec<u8>,
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

#[derive(Debug, PartialEq, Clone)]
pub struct InitedDataEntry {
    pub memory_data_type: MemoryDataType,
    pub data: Vec<u8>,
    pub length: u32,
    pub align: u16, // should not be '0'
}

impl InitedDataEntry {
    /// note that 'i32' in function name means a 32-bit integer, which is equivalent to
    /// the 'uint32_t' in C or 'u32' in Rust. do not confuse it with 'i32' in Rust.
    /// the same applies to the i8, i16 and i64.
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

#[derive(Debug, PartialEq, Clone)]
pub struct UninitDataEntry {
    pub memory_data_type: MemoryDataType,
    pub length: u32,
    pub align: u16, // should not be '0'
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

#[derive(Debug, PartialEq, Clone)]
pub struct ImportModuleEntry {
    // Note that this is the name of module/package,
    // it CANNOT be the name of submodule (i.e. namespace) even if the current image is
    // a "object module", it also CANNOT be the full name or name path.
    //
    // about the "full_name" and "name_path"
    // -------------------------------------
    // - "full_name" = "module_name::name_path"
    // - "name_path" = "namespace::identifier"
    // - "namespace" = "sub_module_name"{0,N}
    //
    // e.g.
    // the name path of function "add" in submodule "myapp:utils" is "utils::add",
    // and the full name is "myapp::utils::add"
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

#[derive(Debug, PartialEq, Clone)]
pub struct DynamicLinkModuleEntry {
    // Note that this is the name of module/package,
    // it CANNOT be the name of submodule (i.e. namespace) even if the current image is
    // a "object module", it also CANNOT be the full name or name path.
    //
    // about the "full_name" and "name_path"
    // -------------------------------------
    // - "full_name" = "module_name::name_path"
    // - "name_path" = "namespace::identifier"
    // - "namespace" = "sub_module_name"{0,N}
    //
    // e.g.
    // the name path of function "add" in submodule "myapp:utils" is "utils::add",
    // and the full name is "myapp::utils::add"
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

    /// By defuault, the application's module file (*.ancm) is merged
    /// into the application image file (*.anci) as the first module of all
    /// dependent modules for simplification.
    Embed,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[serde(rename = "local")]
pub struct ModuleLocationLocal {
    // The module path (it is absolute path).
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

#[derive(Debug, PartialEq, Clone)]
pub struct ImportFunctionEntry {
    // the full name of imported function
    //
    //
    // about the "full_name" and "name_path"
    // -------------------------------------
    // - "full_name" = "module_name::name_path"
    // - "name_path" = "namespace::identifier"
    // - "namespace" = "sub_module_name"{0,N}
    //
    // e.g.
    // the name path of function "add" in submodule "myapp:utils" is "utils::add",
    // and the full name is "myapp::utils::add"
    //
    // note that the module name can not be the name "module" of virtual module.
    pub full_name: String,
    pub import_module_index: usize,
    pub type_index: usize, // used for validation when linking
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

#[derive(Debug, PartialEq, Clone)]
pub struct ImportDataEntry {
    // the full name of imported data
    //
    // about the "full_name" and "name_path"
    // -------------------------------------
    // - "full_name" = "module_name::name_path"
    // - "name_path" = "namespace::identifier"
    // - "namespace" = "sub_module_name"{0,N}
    //
    // e.g.
    // the name path of function "add" in submodule "myapp:utils" is "utils::add",
    // and the full name is "myapp::utils::add"
    //
    // note that the module name can not be the name "module" of virtual module.
    pub full_name: String,
    pub import_module_index: usize,
    pub data_section_type: DataSectionType, // for validation when linking
    pub memory_data_type: MemoryDataType,   // for validation when linking
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

#[derive(Debug, PartialEq, Clone)]
pub struct ExportFunctionEntry {
    // the full name of the exported function
    //
    // about the "full_name" and "name_path"
    // -------------------------------------
    // - "full_name" = "module_name::name_path"
    // - "name_path" = "namespace::identifier"
    // - "namespace" = "sub_module_name"{0,N}
    //
    // e.g.
    // the name path of function "add" in submodule "myapp:utils" is "utils::add",
    // and the full name is "myapp::utils::add"
    //
    // note that the module name can not be the name "module" of virtual module.
    pub full_name: String,
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

#[derive(Debug, PartialEq, Clone)]
pub struct ExportDataEntry {
    // the full name of exported data
    //
    // about the "full_name" and "name_path"
    // -------------------------------------
    // - "full_name" = "module_name::name_path"
    // - "name_path" = "namespace::identifier"
    // - "namespace" = "sub_module_name"{0,N}
    //
    // e.g.
    // the name path of function "add" in submodule "myapp:utils" is "utils::add",
    // and the full name is "myapp::utils::add"
    //
    // note that the module name can not be the name "module" of virtual module.
    pub full_name: String,
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

#[derive(Debug, PartialEq, Clone)]
pub struct RelocateListEntry {
    pub relocate_entries: Vec<RelocateEntry>,
}

impl RelocateListEntry {
    pub fn new(relocate_entries: Vec<RelocateEntry>) -> Self {
        Self { relocate_entries }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct RelocateEntry {
    // offset in functions
    // this 'code_offset' is different from the 'code_offset' in the FunctionItem, which
    // is the offset in the function bytecode area.
    pub code_offset: usize,
    pub relocate_type: RelocateType,
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
impl RelocateEntry {
    pub fn new(code_offset: usize, relocate_type: RelocateType) -> Self {
        Self {
            code_offset,
            relocate_type,
        }
    }

    // for instructions:
    // - data_load_*
    // - data_store_*
    // - host_addr_data
    // - data_load_extend_*
    // - data_store_extend_*
    // - host_addr_data_extend
    pub fn from_data_public_index(inst_addr: usize) -> Self {
        RelocateEntry::new(inst_addr + 4, RelocateType::DataPublicIndex)
    }

    // for instructions:
    // - call
    // - get_function
    // - host_addr_function
    pub fn from_function_public_index(inst_addr: usize) -> Self {
        RelocateEntry::new(inst_addr + 4, RelocateType::FunctionPublicIndex)
    }

    // for instruction:
    // - extcall
    pub fn from_external_function_index(inst_addr: usize) -> Self {
        RelocateEntry::new(inst_addr + 4, RelocateType::ExternalFunctionIndex)
    }

    // for instructions:
    // - block
    // - block_alt
    pub fn from_block_with_type_and_local_variables(inst_addr: usize) -> Vec<Self> {
        vec![
            RelocateEntry::new(inst_addr + 4, RelocateType::TypeIndex),
            RelocateEntry::new(inst_addr + 8, RelocateType::LocalVariableListIndex),
        ]
    }

    // for instruction:
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

/// internal entry point names:
///
/// - internal entry point name: "_start"
///   executes function: '{app_module_name}::_start' (the default entry point)
///   user CLI unit name: "" (empty string)
///
/// - internal entry point name: "{submodule_name}"
///   executes function: '{app_module_name}::app::{submodule_name}::_start' (the additional executable units)
///   user CLI unit name: ":{submodule_name}"
///
/// - internal entry point name: "{submodule_name}::test_*"
///   executes function: '{app_module_name}::tests::{submodule_name}::test_*' (unit tests)
///   user CLI unit name: name path prefix, e.g. "{submodule_name}", "{submodule_name}::test_get_"
#[derive(Debug, PartialEq)]
pub struct EntryPointEntry {
    /// The internal name of the entry points.
    pub unit_name: String,
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

#[derive(Debug)]
pub struct ImageCommonEntry {
    // Note that this is the name of module/package,
    // it CANNOT be the name of submodule (i.e. namespace) even if the current image is
    // a "object module", it also CANNOT be the full name or name path.
    //
    // about the "full_name" and "name_path"
    // -------------------------------------
    // - "full_name" = "module_name::name_path"
    // - "name_path" = "namespace::identifier"
    // - "namespace" = "sub_module_name"{0,N}
    //
    // e.g.
    // the name path of function "add" in submodule "myapp:utils" is "utils::add",
    // and the full name is "myapp::utils::add"
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

    // the dependencies
    pub import_module_entries: Vec<ImportModuleEntry>,

    // the following entries are used for linking:
    // - import_function_entries
    // - import_data_entries
    // - export_function_entries
    // - export_data_entries
    pub import_function_entries: Vec<ImportFunctionEntry>,
    pub import_data_entries: Vec<ImportDataEntry>,

    // the name path entries only contain the internal functions.
    pub export_function_entries: Vec<ExportFunctionEntry>,

    // the name path entries only contain the internal data items.
    pub export_data_entries: Vec<ExportDataEntry>,

    pub relocate_list_entries: Vec<RelocateListEntry>,

    // the dependencies
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
