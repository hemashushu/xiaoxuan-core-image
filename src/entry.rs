// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

//! Entries are used to simplify the creation and parsing of
//! sections.
//!
//! Sections are based on binary, and Entries are based
//! on general data types. Compiler and unit tests
//! access Sections through Entries, but Entries are not need
//! at runtime, which accesses the binary image directly.
//!
//! about the "full_name" and "name_path"
//! -------------------------------------
//! - "full_name" = "module_name::name_path"
//! - "name_path" = "namespace::identifier"
//! - "namespace" = "sub_module_name"{0,N}

use anc_isa::{
    DataSectionType, ExternalLibraryDependency, MemoryDataType, ModuleDependency, OperandDataType,
};

#[derive(Debug, PartialEq, Clone)]
pub struct TypeEntry {
    pub params: Vec<OperandDataType>,
    pub results: Vec<OperandDataType>,
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

#[derive(Debug, PartialEq)]
pub struct FunctionEntry {
    pub type_index: usize,
    pub local_list_index: usize,
    pub code: Vec<u8>,
}

#[derive(Debug, PartialEq)]
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

#[derive(Debug, PartialEq)]
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
    // it CANNOT be the sub-module name even if the current image is
    // the object file of a sub-module.
    // it CANNOT be a name path either.
    //
    // about the "full_name" and "name_path"
    // -------------------------------------
    // - "full_name" = "module_name::name_path"
    // - "name_path" = "namespace::identifier"
    // - "namespace" = "sub_module_name"{0,N}
    pub name: String,
    pub value: Box<ModuleDependency>,
}

impl ImportModuleEntry {
    pub fn new(name: String, value: Box<ModuleDependency>) -> Self {
        Self { name, value }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ImportFunctionEntry {
    // the exported name path,
    // name path includes the submodule name path, but does not include the module name.
    //
    // e.g.
    // the name path of functon 'add' in module 'myapp' is 'add',
    // the name path of function 'add' in submodule 'myapp:utils' is 'utils::add'.
    //
    // about the "full_name" and "name_path"
    // -------------------------------------
    // - "full_name" = "module_name::name_path"
    // - "name_path" = "namespace::identifier"
    // - "namespace" = "sub_module_name"{0,N}
    pub name_path: String,
    pub import_module_index: usize,
    pub type_index: usize, // used for validation when linking
}

impl ImportFunctionEntry {
    pub fn new(name_path: String, import_module_index: usize, type_index: usize) -> Self {
        Self {
            name_path,
            import_module_index,
            type_index,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ImportDataEntry {
    // the exported name path,
    // name path includes the submodule name path, but does not include the module name.
    //
    // e.g.
    // the name path of data 'buf' in module 'myapp' is 'buf',
    // the name path of data 'buf' in submodule 'myapp:utils' is 'utils::buf'.
    //
    // about the "full_name" and "name_path"
    // -------------------------------------
    // - "full_name" = "module_name::name_path"
    // - "name_path" = "namespace::identifier"
    // - "namespace" = "sub_module_name"{0,N}
    pub name_path: String,
    pub import_module_index: usize,
    pub data_section_type: DataSectionType, // for validation when linking
    pub memory_data_type: MemoryDataType,   // for validation when linking
}

impl ImportDataEntry {
    pub fn new(
        name_path: String,
        import_module_index: usize,
        data_section_type: DataSectionType,
        memory_data_type: MemoryDataType,
    ) -> Self {
        Self {
            name_path,
            import_module_index,
            data_section_type,
            memory_data_type,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct FunctionNamePathEntry {
    // the exported name path,
    // name path includes the submodule name path, but does not include the module name.
    //
    // e.g.
    // the name path of functon 'add' in module 'myapp' is 'add',
    // the name path of function 'add' in submodule 'myapp:utils' is 'utils::add'.
    //
    // about the "full_name" and "name_path"
    // -------------------------------------
    // - "full_name" = "module_name::name_path"
    // - "name_path" = "namespace::identifier"
    // - "namespace" = "sub_module_name"{0,N}
    pub name_path: String,
    // pub function_public_index: usize, // this field is used for bridge function call
    pub export: bool,
}

impl FunctionNamePathEntry {
    pub fn new(name_path: String, /* function_public_index: usize,*/ export: bool) -> Self {
        Self {
            name_path,
            // function_public_index,
            export,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct DataNamePathEntry {
    // the exported name path,
    // name path includes the submodule name path, but does not include the module name.
    //
    // e.g.
    // the name path of data 'buf' in module 'myapp' is 'buf',
    // the name path of data 'buf' in submodule 'myapp:utils' is 'utils::buf'.
    //
    // about the "full_name" and "name_path"
    // -------------------------------------
    // - "full_name" = "module_name::name_path"
    // - "name_path" = "namespace::identifier"
    // - "namespace" = "sub_module_name"{0,N}
    pub name_path: String,
    // pub data_public_index: usize, // this field is used for bridge function call
    pub export: bool,
}

impl DataNamePathEntry {
    pub fn new(name_path: String, /* data_public_index: usize, */ export: bool) -> Self {
        Self {
            name_path,
            // data_public_index,
            export,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct FunctionIndexEntry {
    // pub function_public_index: usize,
    pub target_module_index: usize,
    pub function_internal_index: usize,
}

impl FunctionIndexEntry {
    pub fn new(
        // function_public_index: usize,
        target_module_index: usize,
        function_internal_index: usize,
    ) -> Self {
        Self {
            // function_public_index,
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
    // pub data_public_index: usize,
    pub target_module_index: usize,
    pub data_internal_index: usize,
    pub target_data_section_type: DataSectionType,
}

impl DataIndexEntry {
    pub fn new(
        // data_public_index: usize,
        target_module_index: usize,
        data_internal_index: usize,
        target_data_section_type: DataSectionType,
    ) -> Self {
        Self {
            // data_public_index,
            target_module_index,
            data_internal_index,
            target_data_section_type,
        }
    }
}

/// DataIndexListEntry per Module
#[derive(Debug)]
pub struct DataIndexListEntry {
    pub index_entries: Vec<DataIndexEntry>,
}

impl DataIndexListEntry {
    pub fn new(index_entries: Vec<DataIndexEntry>) -> Self {
        Self { index_entries }
    }
}

// #[derive(Debug, PartialEq)]
// pub struct UnifiedExternalLibraryEntry {
//     pub name: String,
//     pub value: Box<ExternalLibraryDependency>,
//     pub external_library_dependent_type: ExternalLibraryDependencyType,
// }
//
// impl UnifiedExternalLibraryEntry {
//     pub fn new(
//         name: String,
//         value: Box<ExternalLibraryDependency>,
//         external_library_dependent_type: ExternalLibraryDependencyType,
//     ) -> Self {
//         Self {
//             name,
//             value,
//             external_library_dependent_type,
//         }
//     }
// }

// #[derive(Debug, PartialEq)]
// pub struct UnifiedExternalFunctionEntry {
//     pub name: String,
//     pub unified_external_library_index: usize,
// }
//
// impl UnifiedExternalFunctionEntry {
//     pub fn new(name: String, unified_external_library_index: usize) -> Self {
//         Self {
//             name,
//             unified_external_library_index,
//         }
//     }
// }

#[derive(Debug)]
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
    // pub external_function_index: usize,
    pub unified_external_function_index: usize,
    // // copy the type_index from ExternalFuncSection of the specific module,
    // // so that the ExternalFuncSection can be omitted at runtime.
    // pub type_index: usize,
}

impl ExternalFunctionIndexEntry {
    pub fn new(
        // external_function_index: usize,
        unified_external_function_index: usize,
        // type_index: usize,
    ) -> Self {
        Self {
            // external_function_index,
            unified_external_function_index,
            // type_index,
        }
    }
}
