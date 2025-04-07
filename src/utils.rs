// Copyright (c) 2025 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions.
// For more details, see the LICENSE, LICENSE.additional, and CONTRIBUTING files.

// Import necessary modules and sections for handling module images and their components.
use crate::common_sections::data_name_section::DataNameSection;
use crate::common_sections::external_function_section::ExternalFunctionSection;
use crate::common_sections::external_library_section::ExternalLibrarySection;
use crate::common_sections::function_name_section::FunctionNameSection;
use crate::common_sections::function_section::FunctionSection;
use crate::common_sections::local_variable_section::LocalVariableSection;
use crate::common_sections::property_section::PropertySection;
use crate::common_sections::read_only_data_section::ReadOnlyDataSection;
use crate::common_sections::read_write_data_section::ReadWriteDataSection;
use crate::common_sections::type_section::TypeSection;
use crate::common_sections::uninit_data_section::UninitDataSection;
use crate::linking_sections::data_index_section::{DataIndexItem, DataIndexSection};
use crate::linking_sections::entry_point_section::EntryPointSection;
use crate::linking_sections::external_function_index_section::{
    ExternalFunctionIndexItem, ExternalFunctionIndexSection,
};
use crate::linking_sections::function_index_section::{FunctionIndexItem, FunctionIndexSection};
use crate::linking_sections::linking_module_section::LinkingModuleSection;
use crate::linking_sections::unified_external_function_section::UnifiedExternalFunctionSection;
use crate::linking_sections::unified_external_library_section::UnifiedExternalLibrarySection;
use crate::linking_sections::unified_external_type_section::UnifiedExternalTypeSection;
use crate::ImageError;

use anc_isa::{DataSectionType, OperandDataType, RUNTIME_EDITION};

use crate::entry::{
    DataNameEntry, EntryPointEntry, ExternalFunctionEntry, ExternalLibraryEntry, FunctionEntry,
    FunctionNameEntry, LinkingModuleEntry, LocalVariableEntry, LocalVariableListEntry,
    ModuleLocation, ReadOnlyDataEntry, ReadWriteDataEntry, TypeEntry, UninitDataEntry,
};

use crate::module_image::{ImageType, ModuleImage, RangeItem, SectionEntry, Visibility};

/// A helper object representing a function entry for unit tests.
pub struct HelperFunctionEntry {
    pub params: Vec<OperandDataType>,  // Function parameters.
    pub results: Vec<OperandDataType>, // Function results.
    pub local_variable_item_entries_without_args: Vec<LocalVariableEntry>, // Local variables excluding arguments.
    pub code: Vec<u8>, // Function code in binary format.
}

/// A helper object representing a block entry for unit tests.
pub struct HelperBlockEntry {
    pub params: Vec<OperandDataType>,  // Block parameters.
    pub results: Vec<OperandDataType>, // Block results.
    pub local_variable_item_entries_without_args: Vec<LocalVariableEntry>, // Local variables excluding arguments.
}

/// A helper object representing an external function entry for unit tests.
pub struct HelperExternalFunctionEntry {
    pub name: String,                    // Name of the external function.
    pub external_library_index: usize,   // Index of the external library.
    pub params: Vec<OperandDataType>,    // Parameters of the external function.
    pub result: Option<OperandDataType>, // Result type of the external function, if any.
}

/// Builds a module binary with a single function and no data sections.
/// This is a simplified helper function for unit tests.
pub fn helper_build_module_binary_with_single_function(
    param_datatypes: &[OperandDataType],
    result_datatypes: &[OperandDataType],
    local_variable_entries_without_functions_args: &[LocalVariableEntry],
    code: Vec<u8>,
) -> Vec<u8> {
    helper_build_module_binary_with_single_function_and_data(
        param_datatypes,
        result_datatypes,
        local_variable_entries_without_functions_args,
        code,
        &[], // No read-only data entries.
        &[], // No read-write data entries.
        &[], // No uninitialized data entries.
    )
}

/// Builds a module binary with a single function and data sections.
/// This helper function is used for unit tests.
pub fn helper_build_module_binary_with_single_function_and_data(
    param_datatypes: &[OperandDataType],
    result_datatypes: &[OperandDataType],
    local_variable_entries_without_function_args: &[LocalVariableEntry],
    code: Vec<u8>,
    read_only_data_entries: &[ReadOnlyDataEntry],
    read_write_data_entries: &[ReadWriteDataEntry],
    uninit_uninit_data_entries: &[UninitDataEntry],
) -> Vec<u8> {
    let type_entry = TypeEntry {
        params: param_datatypes.to_owned(),
        results: result_datatypes.to_owned(),
    };

    let params_as_local_variables = param_datatypes
        .iter()
        .map(|data_type| convert_operand_data_type_to_local_variable_entry(*data_type))
        .collect::<Vec<_>>();

    let mut local_variables = vec![];
    local_variables.extend_from_slice(&params_as_local_variables);
    local_variables.extend_from_slice(local_variable_entries_without_function_args);

    let local_list_entry = LocalVariableListEntry {
        local_variable_entries: local_variables,
    };

    let function_entry = FunctionEntry {
        type_index: 0,
        local_variable_list_index: 0,
        code,
    };

    helper_build_module_binary(
        "main",
        read_only_data_entries,
        read_write_data_entries,
        uninit_uninit_data_entries,
        &[type_entry],
        &[local_list_entry],
        &[function_entry],
        &[],
        &[],
        0,
    )
}

/// Builds a module binary with a single function and blocks.
/// This helper function is used for unit tests.
pub fn helper_build_module_binary_with_single_function_and_blocks(
    param_datatypes: Vec<OperandDataType>,
    result_datatypes: Vec<OperandDataType>,
    local_variable_item_entries_without_args: Vec<LocalVariableEntry>,
    code: Vec<u8>,
    helper_block_entries: Vec<HelperBlockEntry>,
) -> Vec<u8> {
    let helper_function_entry = HelperFunctionEntry {
        params: param_datatypes,
        results: result_datatypes,
        local_variable_item_entries_without_args,
        code,
    };

    helper_build_module_binary_with_functions_and_blocks(
        &[helper_function_entry],
        &helper_block_entries,
    )
}

/// Builds a module binary with multiple functions and blocks.
/// This helper function is used for unit tests.
pub fn helper_build_module_binary_with_functions_and_blocks(
    helper_function_entries: &[HelperFunctionEntry],
    helper_block_entries: &[HelperBlockEntry],
) -> Vec<u8> {
    // Build type entries.
    // Note: For simplicity, duplicate items are not merged.

    let function_type_entries = helper_function_entries
        .iter()
        .map(|entry| TypeEntry {
            params: entry.params.clone(),
            results: entry.results.clone(),
        })
        .collect::<Vec<_>>();

    let block_type_entries = helper_block_entries
        .iter()
        .map(|entry| TypeEntry {
            params: entry.params.clone(),
            results: entry.results.clone(),
        })
        .collect::<Vec<_>>();

    let mut type_entries = vec![];
    type_entries.extend_from_slice(&function_type_entries);
    type_entries.extend_from_slice(&block_type_entries);

    // Build local variable list entries.
    // Note: For simplicity, duplicate items are not merged.

    let local_list_entries_of_functions = helper_function_entries
        .iter()
        .map(|entry| {
            let params_as_local_variables = entry
                .params
                .iter()
                .map(|data_type| convert_operand_data_type_to_local_variable_entry(*data_type))
                .collect::<Vec<_>>();

            let mut local_variables = vec![];
            local_variables.extend_from_slice(&params_as_local_variables);
            local_variables.extend_from_slice(&entry.local_variable_item_entries_without_args);

            LocalVariableListEntry {
                local_variable_entries: local_variables,
            }
        })
        .collect::<Vec<_>>();

    let local_list_entries_of_blocks = helper_block_entries
        .iter()
        .map(|entry| {
            let params_as_local_variables = entry
                .params
                .iter()
                .map(|data_type| convert_operand_data_type_to_local_variable_entry(*data_type))
                .collect::<Vec<_>>();

            let mut local_variables = vec![];
            local_variables.extend_from_slice(&params_as_local_variables);
            local_variables.extend_from_slice(&entry.local_variable_item_entries_without_args);

            LocalVariableListEntry {
                local_variable_entries: local_variables,
            }
        })
        .collect::<Vec<_>>();

    let mut local_list_entries = vec![];
    local_list_entries.extend_from_slice(&local_list_entries_of_functions);
    local_list_entries.extend_from_slice(&local_list_entries_of_blocks);

    // Build function entries.
    let function_entries = helper_function_entries
        .iter()
        .enumerate()
        .map(|(idx, entry)| FunctionEntry {
            type_index: idx,
            local_variable_list_index: idx,
            code: entry.code.clone(),
        })
        .collect::<Vec<_>>();

    let entry_function_public_index = 0;

    helper_build_module_binary(
        "main",
        &[],
        &[],
        &[],
        &type_entries,
        &local_list_entries,
        &function_entries,
        &[],
        &[],
        entry_function_public_index,
    )
}

/// Builds a module binary with functions, data, and external functions.
/// This helper function is used for unit tests.
#[allow(clippy::too_many_arguments)]
pub fn helper_build_module_binary_with_functions_and_data_and_external_functions(
    helper_function_entries: &[HelperFunctionEntry],
    read_only_data_entries: &[ReadOnlyDataEntry],
    read_write_data_entries: &[ReadWriteDataEntry],
    uninit_uninit_data_entries: &[UninitDataEntry],
    external_library_entries: &[ExternalLibraryEntry],
    helper_external_function_entries: &[HelperExternalFunctionEntry],
) -> Vec<u8> {
    // Note: For simplicity, duplicate items are not merged.

    let function_type_entries = helper_function_entries
        .iter()
        .map(|entry| TypeEntry {
            params: entry.params.clone(),
            results: entry.results.clone(),
        })
        .collect::<Vec<_>>();

    let external_function_type_entries = helper_external_function_entries
        .iter()
        .map(|entry| TypeEntry {
            params: entry.params.clone(),
            results: if let Some(t) = entry.result {
                vec![t]
            } else {
                vec![]
            },
        })
        .collect::<Vec<_>>();

    let mut type_entries = vec![];
    type_entries.extend_from_slice(&function_type_entries);
    type_entries.extend_from_slice(&external_function_type_entries);

    // Build local variable list entries.
    // Note: For simplicity, duplicate items are not merged.

    let local_list_entries = helper_function_entries
        .iter()
        .map(|entry| {
            let params_as_local_variables = entry
                .params
                .iter()
                .map(|data_type| convert_operand_data_type_to_local_variable_entry(*data_type))
                .collect::<Vec<_>>();

            let mut local_variables = vec![];
            local_variables.extend_from_slice(&params_as_local_variables);
            local_variables.extend_from_slice(&entry.local_variable_item_entries_without_args);

            LocalVariableListEntry {
                local_variable_entries: local_variables,
            }
        })
        .collect::<Vec<_>>();

    // Build function entries.
    let function_entries = helper_function_entries
        .iter()
        .enumerate()
        .map(|(idx, entry)| FunctionEntry {
            type_index: idx,
            local_variable_list_index: idx,
            code: entry.code.clone(),
        })
        .collect::<Vec<_>>();

    let external_function_entries = helper_external_function_entries
        .iter()
        .enumerate()
        .map(|(idx, entry)| ExternalFunctionEntry {
            name: entry.name.clone(),
            external_library_index: entry.external_library_index,
            type_index: idx + function_entries.len(),
        })
        .collect::<Vec<_>>();

    helper_build_module_binary(
        "main",
        read_only_data_entries,
        read_write_data_entries,
        uninit_uninit_data_entries,
        &type_entries,
        &local_list_entries,
        &function_entries,
        external_library_entries,
        &external_function_entries,
        0,
    )
}

/// Builds a complete module binary with all sections.
/// This is a low-level helper function for unit tests.
#[allow(clippy::too_many_arguments)]
pub fn helper_build_module_binary(
    name: &str,
    read_only_data_entries: &[ReadOnlyDataEntry],
    read_write_data_entries: &[ReadWriteDataEntry],
    uninit_uninit_data_entries: &[UninitDataEntry],
    type_entries: &[TypeEntry],
    local_list_entries: &[LocalVariableListEntry],
    function_entries: &[FunctionEntry],
    external_library_entries: &[ExternalLibraryEntry],
    external_function_entries: &[ExternalFunctionEntry],
    entry_function_public_index: usize,
) -> Vec<u8> {
    // Type section.
    let (type_items, types_data) = TypeSection::convert_from_entries(type_entries);
    let type_section = TypeSection {
        items: &type_items,
        types_data: &types_data,
    };

    // Local variable section.
    let (local_lists, local_list_data) =
        LocalVariableSection::convert_from_entries(local_list_entries);
    let local_variable_section = LocalVariableSection {
        lists: &local_lists,
        list_data: &local_list_data,
    };

    // Function section.
    let (function_items, codes_data) = FunctionSection::convert_from_entries(function_entries);
    let function_section = FunctionSection {
        items: &function_items,
        codes_data: &codes_data,
    };

    // Read-only data section.
    let (ro_items, ro_data) = ReadOnlyDataSection::convert_from_entries(read_only_data_entries);
    let ro_data_section = ReadOnlyDataSection {
        items: &ro_items,
        datas_data: &ro_data,
    };

    // Read-write data section.
    let (rw_items, rw_data) = ReadWriteDataSection::convert_from_entries(read_write_data_entries);
    let rw_data_section = ReadWriteDataSection {
        items: &rw_items,
        datas_data: &rw_data,
    };

    // Uninitialized data section.
    let uninit_items = UninitDataSection::convert_from_entries(uninit_uninit_data_entries);
    let uninit_data_section = UninitDataSection {
        items: &uninit_items,
    };

    // Export function section.
    // For simplicity, these are arbitrary items.
    let (export_function_items, export_function_names_data) =
        FunctionNameSection::convert_from_entries(&[
            FunctionNameEntry::new("func0".to_owned(), Visibility::Public, 0),
            FunctionNameEntry::new("func1".to_owned(), Visibility::Public, 1),
        ]);

    let export_function_section = FunctionNameSection {
        items: &export_function_items,
        full_names_data: &export_function_names_data,
    };

    // Export data section.
    // For simplicity, these are arbitrary items.
    let (export_data_items, export_data_names_data) = DataNameSection::convert_from_entries(&[
        DataNameEntry::new(
            "data0".to_owned(),
            Visibility::Public,
            DataSectionType::ReadWrite,
            0,
        ),
        DataNameEntry::new(
            "data1".to_owned(),
            Visibility::Public,
            DataSectionType::ReadWrite,
            1,
        ),
    ]);

    let export_data_section = DataNameSection {
        items: &export_data_items,
        full_names_data: &export_data_names_data,
    };

    // External library section.
    let (external_library_items, external_library_data) =
        ExternalLibrarySection::convert_from_entries(external_library_entries);
    let external_library_section = ExternalLibrarySection {
        items: &external_library_items,
        items_data: &external_library_data,
    };

    // External function section.
    let (external_function_items, external_function_data) =
        ExternalFunctionSection::convert_from_entries(external_function_entries);
    let external_function_section = ExternalFunctionSection {
        items: &external_function_items,
        names_data: &external_function_data,
    };

    // Property section.
    let property_section = PropertySection::new(name, *RUNTIME_EDITION, 0, 0, 1 /* 0, 0 */);

    // Function index.
    let function_ranges: Vec<RangeItem> = vec![RangeItem {
        offset: 0,
        count: function_entries.len() as u32,
    }];

    let function_index_items: Vec<FunctionIndexItem> = (0..function_entries.len())
        .map(|idx| {
            let idx_u32 = idx as u32;
            FunctionIndexItem::new(0, idx_u32)
        })
        .collect::<Vec<_>>();

    let function_index_section = FunctionIndexSection {
        ranges: &function_ranges,
        items: &function_index_items,
    };

    // Data index.
    // The data index is ordered by:
    // 1. Imported read-only data.
    // 2. Imported read-write data.
    // 3. Imported uninitialized data.
    // 4. Read-only data.
    // 5. Read-write data.
    // 6. Uninitialized data.
    let data_ranges: Vec<RangeItem> = vec![RangeItem {
        offset: 0,
        count: (ro_items.len() + rw_items.len() + uninit_items.len()) as u32,
    }];

    let mut data_index_items: Vec<DataIndexItem> = vec![];

    let ro_iter = ro_items
        .iter()
        .enumerate()
        .map(|(idx, _item)| (idx, DataSectionType::ReadOnly));
    let rw_iter = rw_items
        .iter()
        .enumerate()
        .map(|(idx, _item)| (idx, DataSectionType::ReadWrite));
    let uninit_iter = uninit_items
        .iter()
        .enumerate()
        .map(|(idx, _item)| (idx, DataSectionType::Uninit));

    for (idx, data_section_type) in ro_iter.chain(rw_iter).chain(uninit_iter) {
        data_index_items.push(DataIndexItem::new(0, data_section_type, idx as u32));
    }

    let data_index_section = DataIndexSection {
        ranges: &data_ranges,
        items: &data_index_items,
    };

    // Unified external library section.
    // For simplicity, build 1:1 to the external_library_entries.
    let unified_external_library_entries = external_library_entries;
    let (unified_external_library_items, unified_external_library_data) =
        UnifiedExternalLibrarySection::convert_from_entries(unified_external_library_entries);
    let unified_external_library_section = UnifiedExternalLibrarySection {
        items: &unified_external_library_items,
        items_data: &unified_external_library_data,
    };

    // Unified external type section.
    // For simplicity, build 1:1 to type_entries.
    let unified_external_type_entries = type_entries;
    let (unified_external_type_items, unified_external_type_data) =
        UnifiedExternalTypeSection::convert_from_entries(unified_external_type_entries);
    let unified_external_type_section = UnifiedExternalTypeSection {
        items: &unified_external_type_items,
        types_data: &unified_external_type_data,
    };

    // Unified external function section.
    // For simplicity, build 1:1 to external_function_entries.
    let unified_external_function_entries = external_function_entries;
    let (unified_external_function_items, unified_external_function_data) =
        UnifiedExternalFunctionSection::convert_from_entries(unified_external_function_entries);
    let unified_external_function_section = UnifiedExternalFunctionSection {
        items: &unified_external_function_items,
        names_data: &unified_external_function_data,
    };

    // External function index section.
    let external_function_ranges: Vec<RangeItem> = vec![RangeItem {
        offset: 0,
        count: unified_external_function_entries.len() as u32,
    }];

    let external_function_index_items: Vec<ExternalFunctionIndexItem> = external_function_entries
        .iter()
        .enumerate()
        .map(|(idx, _)| ExternalFunctionIndexItem::new(idx as u32))
        .collect::<Vec<_>>();

    let external_function_index_section = ExternalFunctionIndexSection {
        ranges: &external_function_ranges,
        items: &external_function_index_items,
    };

    // Entry point section.
    let entry_point_entries = vec![EntryPointEntry::new(
        "".to_string(), // The name of the default entry point is an empty string.
        entry_function_public_index,
    )];
    let (entry_point_items, unit_names_data) =
        EntryPointSection::convert_from_entries(&entry_point_entries);
    let entry_point_section = EntryPointSection {
        items: &entry_point_items,
        unit_names_data: &unit_names_data,
    };

    // Dynamic link module list.
    let import_module_entry =
        LinkingModuleEntry::new(name.to_owned(), Box::new(ModuleLocation::Embed));
    let (module_list_items, module_list_data) =
        LinkingModuleSection::convert_from_entries(&[import_module_entry]);
    let module_list_section = LinkingModuleSection {
        items: &module_list_items,
        items_data: &module_list_data,
    };

    // Build module image.
    let section_entries: Vec<&dyn SectionEntry> = vec![
        /* The following are common sections. */
        &property_section,
        &type_section,
        &local_variable_section,
        &function_section,
        &ro_data_section,
        &rw_data_section,
        &uninit_data_section,
        &export_function_section,
        &export_data_section,
        /* Empty sections: import_module, import_function, import_data. */
        &external_library_section,
        &external_function_section,
        /* The following are index sections. */
        &entry_point_section,
        &module_list_section,
        &function_index_section,
        &data_index_section,
        &unified_external_type_section,
        &unified_external_library_section,
        &unified_external_function_section,
        &external_function_index_section,
    ];

    let (section_items, sections_data) =
        ModuleImage::convert_from_section_entries(&section_entries);
    let module_image = ModuleImage {
        image_type: ImageType::Application,
        items: &section_items,
        sections_data: &sections_data,
    };

    // Build module image binary.
    let mut image_binary: Vec<u8> = vec![];
    module_image.write(&mut image_binary).unwrap();
    image_binary
}

/// Loads modules from their binary representations.
/// This helper function is used for unit tests.
pub fn helper_load_modules_from_binaries<'a>(
    module_binaries: &[&'a [u8]],
) -> Result<Vec<ModuleImage<'a>>, ImageError> {
    let mut module_images: Vec<ModuleImage> = vec![];

    for binary in module_binaries {
        let module_image = ModuleImage::read(binary)?;
        module_images.push(module_image);
    }

    Ok(module_images)
}

/// Converts an operand data type to a local variable entry.
/// This is a utility function used internally.
fn convert_operand_data_type_to_local_variable_entry(
    operand_data_type: OperandDataType,
) -> LocalVariableEntry {
    match operand_data_type {
        OperandDataType::I32 => LocalVariableEntry::from_i32(),
        OperandDataType::I64 => LocalVariableEntry::from_i64(),
        OperandDataType::F32 => LocalVariableEntry::from_f32(),
        OperandDataType::F64 => LocalVariableEntry::from_f64(),
    }
}

#[cfg(test)]
mod tests {
    use core::str;
    use std::collections::HashMap;

    use anc_isa::{
        DataSectionType, DependencyCondition, DependencyLocal, DependencyShare,
        ExternalLibraryDependency, ExternalLibraryDependencyType, MemoryDataType, OperandDataType,
    };

    use crate::{
        common_sections::{
            self, local_variable_section::LocalVariableItem, read_only_data_section::DataItem,
        },
        entry::{
            ExternalLibraryEntry, LocalVariableEntry, ReadOnlyDataEntry, ReadWriteDataEntry,
            UninitDataEntry,
        },
        linking_sections::{
            data_index_section::DataIndexItem,
            external_function_index_section::ExternalFunctionIndexItem,
            function_index_section::FunctionIndexItem,
        },
        module_image::RangeItem,
        utils::{
            helper_build_module_binary_with_functions_and_data_and_external_functions,
            helper_build_module_binary_with_single_function_and_data,
            helper_load_modules_from_binaries, HelperExternalFunctionEntry, HelperFunctionEntry,
        },
    };

    #[test]
    fn test_build_module_binary_with_single_function_and_data_sections() {
        // Test building a module binary with a single function and data sections.
        let binary = helper_build_module_binary_with_single_function_and_data(
            &[OperandDataType::I64, OperandDataType::I64],
            &[OperandDataType::I32],
            &[LocalVariableEntry::from_i32()],
            vec![0u8],
            &[
                ReadOnlyDataEntry::from_i32(0x11),
                ReadOnlyDataEntry::from_i64(0x13),
            ],
            &[ReadWriteDataEntry::from_bytes(
                vec![0x17u8, 0x19, 0x23, 0x29, 0x31, 0x37],
                8,
            )],
            &[
                UninitDataEntry::from_i32(),
                UninitDataEntry::from_i64(),
                UninitDataEntry::from_i32(),
            ],
        );

        // Load module.
        let module_images = helper_load_modules_from_binaries(&[&binary]).unwrap();
        assert_eq!(module_images.len(), 1);

        // Check module image.
        let module_image = &module_images[0];

        // Check data index section.
        let data_index_section = module_image.get_optional_data_index_section().unwrap();
        assert_eq!(data_index_section.ranges.len(), 1);
        assert_eq!(data_index_section.items.len(), 6);

        assert_eq!(&data_index_section.ranges[0], &RangeItem::new(0, 6));

        assert_eq!(
            data_index_section.items,
            &[
                //
                DataIndexItem::new(0, DataSectionType::ReadOnly, 0,),
                DataIndexItem::new(0, DataSectionType::ReadOnly, 1,),
                //
                DataIndexItem::new(0, DataSectionType::ReadWrite, 0,),
                //
                DataIndexItem::new(0, DataSectionType::Uninit, 0,),
                DataIndexItem::new(0, DataSectionType::Uninit, 1,),
                DataIndexItem::new(0, DataSectionType::Uninit, 2,),
            ]
        );

        // Check function index section.
        let function_index_section = module_image.get_function_index_section();
        assert_eq!(function_index_section.ranges.len(), 1);
        assert_eq!(function_index_section.items.len(), 1);

        assert_eq!(&function_index_section.ranges[0], &RangeItem::new(0, 1));

        assert_eq!(
            function_index_section.items,
            &[FunctionIndexItem::new(0, 0)]
        );

        // Check data sections.
        let ro_section = module_image.get_optional_read_only_data_section().unwrap();
        assert_eq!(
            &ro_section.items[0],
            &DataItem::new(0, 4, MemoryDataType::I32, 4)
        );
        assert_eq!(
            &ro_section.items[1],
            &DataItem::new(8, 8, MemoryDataType::I64, 8)
        );
        assert_eq!(
            &ro_section.datas_data[ro_section.items[0].data_offset as usize..][0..4],
            [0x11, 0, 0, 0]
        );
        assert_eq!(
            &ro_section.datas_data[ro_section.items[1].data_offset as usize..][0..8],
            [0x13, 0, 0, 0, 0, 0, 0, 0]
        );

        let rw_section = module_image.get_optional_read_write_data_section().unwrap();
        assert_eq!(
            &rw_section.items[0],
            &common_sections::read_write_data_section::DataItem::new(
                0,
                6,
                MemoryDataType::Bytes,
                8
            )
        );
        assert_eq!(
            &rw_section.datas_data[rw_section.items[0].data_offset as usize..][0..6],
            &[0x17u8, 0x19, 0x23, 0x29, 0x31, 0x37]
        );

        let uninit_section = module_image.get_optional_uninit_data_section().unwrap();
        assert_eq!(
            &uninit_section.items[0],
            &common_sections::uninit_data_section::DataItem::new(0, 4, MemoryDataType::I32, 4)
        );
        assert_eq!(
            &uninit_section.items[1],
            &common_sections::uninit_data_section::DataItem::new(8, 8, MemoryDataType::I64, 8)
        );
        assert_eq!(
            &uninit_section.items[2],
            &common_sections::uninit_data_section::DataItem::new(16, 4, MemoryDataType::I32, 4)
        );

        // Check type section.
        let type_section = module_image.get_type_section();
        assert_eq!(type_section.items.len(), 1);
        assert_eq!(
            type_section.get_item_params_and_results(0),
            (
                &[OperandDataType::I64, OperandDataType::I64][..],
                &[OperandDataType::I32][..]
            )
        );

        // Check function section.
        let function_section = module_image.get_function_section();
        assert_eq!(function_section.items.len(), 1);

        assert_eq!(
            function_section.get_item_type_index_and_local_variable_list_index_and_code(0),
            (0, 0, vec![0u8].as_ref())
        );

        // Check local variable section.
        let local_variable_section = module_image.get_local_variable_section();
        assert_eq!(local_variable_section.lists.len(), 1);
        assert_eq!(
            local_variable_section.get_local_variable_list(0),
            &[
                LocalVariableItem::new(0, 8, MemoryDataType::I64, 8),
                LocalVariableItem::new(8, 8, MemoryDataType::I64, 8),
                LocalVariableItem::new(16, 4, MemoryDataType::I32, 4),
            ]
        );
    }

    #[test]
    fn test_build_module_binary_with_functions_and_blocks() {
        // TODO: Implement test for building a module binary with functions and blocks.
    }

    #[test]
    fn test_build_module_binary_with_single_function_and_external_functions() {
        // Test building a module binary with a single function and external functions.
        let binary = helper_build_module_binary_with_functions_and_data_and_external_functions(
            &[HelperFunctionEntry {
                local_variable_item_entries_without_args: vec![],
                code: vec![0u8],
                params: vec![],
                results: vec![],
            }],
            &[],
            &[],
            &[],
            &[
                ExternalLibraryEntry::new(
                    "libc".to_owned(),
                    Box::new(ExternalLibraryDependency::System("libc.so.1".to_owned())),
                ),
                ExternalLibraryEntry::new(
                    "libmagic".to_owned(),
                    Box::new(ExternalLibraryDependency::Share(Box::new(
                        DependencyShare {
                            version: "1.2".to_owned(),
                            condition: DependencyCondition::True,
                            parameters: HashMap::default(),
                        },
                    ))),
                ),
                ExternalLibraryEntry::new(
                    "zlib".to_owned(),
                    Box::new(ExternalLibraryDependency::Local(Box::new(
                        DependencyLocal {
                            path: "libz.so.1".to_owned(),
                            condition: DependencyCondition::True,
                            parameters: HashMap::default(),
                        },
                    ))),
                ),
            ],
            &[
                HelperExternalFunctionEntry {
                    name: "getuid".to_owned(),
                    external_library_index: 0,
                    params: vec![OperandDataType::I32],
                    result: None,
                },
                HelperExternalFunctionEntry {
                    name: "getenv".to_owned(),
                    external_library_index: 0,
                    params: vec![OperandDataType::I32, OperandDataType::I32],
                    result: Some(OperandDataType::I32),
                },
                HelperExternalFunctionEntry {
                    name: "magic_open".to_owned(),
                    external_library_index: 1,
                    params: vec![OperandDataType::I32, OperandDataType::I32],
                    result: Some(OperandDataType::I32),
                },
                HelperExternalFunctionEntry {
                    name: "inflate".to_owned(),
                    external_library_index: 2,
                    params: vec![OperandDataType::I32],
                    result: None,
                },
                HelperExternalFunctionEntry {
                    name: "fopen".to_owned(),
                    external_library_index: 0,
                    params: vec![],
                    result: None,
                },
                HelperExternalFunctionEntry {
                    name: "magic_file".to_owned(),
                    external_library_index: 1,
                    params: vec![OperandDataType::I32, OperandDataType::I32],
                    result: Some(OperandDataType::I32),
                },
            ],
        );

        // Load module.
        let module_images = helper_load_modules_from_binaries(&[&binary]).unwrap();
        assert_eq!(module_images.len(), 1);

        let module_image = &module_images[0];

        // Check unified external library section.
        let unified_external_library_section = module_image
            .get_optional_unified_external_library_section()
            .unwrap();

        assert_eq!(
            {
                let vv = unified_external_library_section
                    .get_item_name_and_external_library_dependent_type_and_value(0);
                let s = str::from_utf8(vv.2).unwrap();
                (
                    vv.0,
                    vv.1,
                    ason::from_str::<ExternalLibraryDependency>(s).unwrap(),
                )
            },
            (
                "libc",
                ExternalLibraryDependencyType::System,
                ExternalLibraryDependency::System("libc.so.1".to_owned(),)
            )
        );

        assert_eq!(
            {
                let vv = unified_external_library_section
                    .get_item_name_and_external_library_dependent_type_and_value(1);
                let s = str::from_utf8(vv.2).unwrap();
                (
                    vv.0,
                    vv.1,
                    ason::from_str::<ExternalLibraryDependency>(s).unwrap(),
                )
            },
            (
                "libmagic",
                ExternalLibraryDependencyType::Share,
                ExternalLibraryDependency::Share(Box::new(DependencyShare {
                    version: "1.2".to_owned(),
                    condition: DependencyCondition::True,
                    parameters: HashMap::default()
                },))
            )
        );

        assert_eq!(
            {
                let vv = unified_external_library_section
                    .get_item_name_and_external_library_dependent_type_and_value(2);
                let s = str::from_utf8(vv.2).unwrap();
                (
                    vv.0,
                    vv.1,
                    ason::from_str::<ExternalLibraryDependency>(s).unwrap(),
                )
            },
            (
                "zlib",
                ExternalLibraryDependencyType::Local,
                ExternalLibraryDependency::Local(Box::new(DependencyLocal {
                    path: "libz.so.1".to_owned(),
                    condition: DependencyCondition::True,
                    parameters: HashMap::default()
                }))
            )
        );

        // Check unified external function section.
        let unified_external_function_section = module_image
            .get_optional_unified_external_function_section()
            .unwrap();
        assert_eq!(
            unified_external_function_section
                .get_item_name_and_external_library_index_and_type_index(0),
            ("getuid", 0, 1)
        );
        assert_eq!(
            unified_external_function_section
                .get_item_name_and_external_library_index_and_type_index(1),
            ("getenv", 0, 2)
        );
        assert_eq!(
            unified_external_function_section
                .get_item_name_and_external_library_index_and_type_index(2),
            ("magic_open", 1, 3)
        );
        assert_eq!(
            unified_external_function_section
                .get_item_name_and_external_library_index_and_type_index(3),
            ("inflate", 2, 4)
        );
        assert_eq!(
            unified_external_function_section
                .get_item_name_and_external_library_index_and_type_index(4),
            ("fopen", 0, 5)
        );
        assert_eq!(
            unified_external_function_section
                .get_item_name_and_external_library_index_and_type_index(5),
            ("magic_file", 1, 6)
        );

        // Check external function index section.
        let external_function_index_section = module_image
            .get_optional_external_function_index_section()
            .unwrap();
        assert_eq!(external_function_index_section.ranges.len(), 1);
        assert_eq!(external_function_index_section.items.len(), 6);

        assert_eq!(
            &external_function_index_section.ranges[0],
            &RangeItem::new(0, 6)
        );

        assert_eq!(
            external_function_index_section.items,
            &[
                ExternalFunctionIndexItem::new(0),
                ExternalFunctionIndexItem::new(1),
                ExternalFunctionIndexItem::new(2),
                ExternalFunctionIndexItem::new(3),
                ExternalFunctionIndexItem::new(4),
                ExternalFunctionIndexItem::new(5),
            ]
        );

        // Check external library sections.
        let external_library_section = module_image
            .get_optional_external_library_section()
            .unwrap();

        assert_eq!(
            {
                let vv = external_library_section
                    .get_item_name_and_external_library_dependent_type_and_value(0);
                let s = str::from_utf8(vv.2).unwrap();
                (
                    vv.0,
                    vv.1,
                    ason::from_str::<ExternalLibraryDependency>(s).unwrap(),
                )
            },
            (
                "libc",
                ExternalLibraryDependencyType::System,
                ExternalLibraryDependency::System("libc.so.1".to_owned(),)
            )
        );

        assert_eq!(
            {
                let vv = external_library_section
                    .get_item_name_and_external_library_dependent_type_and_value(1);
                let s = str::from_utf8(vv.2).unwrap();
                (
                    vv.0,
                    vv.1,
                    ason::from_str::<ExternalLibraryDependency>(s).unwrap(),
                )
            },
            (
                "libmagic",
                ExternalLibraryDependencyType::Share,
                ExternalLibraryDependency::Share(Box::new(DependencyShare {
                    version: "1.2".to_owned(),
                    condition: DependencyCondition::True,
                    parameters: HashMap::default()
                },))
            )
        );

        assert_eq!(
            {
                let vv = external_library_section
                    .get_item_name_and_external_library_dependent_type_and_value(2);
                let s = str::from_utf8(vv.2).unwrap();
                (
                    vv.0,
                    vv.1,
                    ason::from_str::<ExternalLibraryDependency>(s).unwrap(),
                )
            },
            (
                "zlib",
                ExternalLibraryDependencyType::Local,
                ExternalLibraryDependency::Local(Box::new(DependencyLocal {
                    path: "libz.so.1".to_owned(),
                    condition: DependencyCondition::True,
                    parameters: HashMap::default()
                }))
            )
        );

        // Check external function section.
        let external_function_section = module_image
            .get_optional_external_function_section()
            .unwrap();
        assert_eq!(
            external_function_section.get_item_name_and_external_library_index_and_type_index(0),
            ("getuid", 0, 1)
        );
        assert_eq!(
            external_function_section.get_item_name_and_external_library_index_and_type_index(1),
            ("getenv", 0, 2)
        );
        assert_eq!(
            external_function_section.get_item_name_and_external_library_index_and_type_index(2),
            ("magic_open", 1, 3)
        );
        assert_eq!(
            external_function_section.get_item_name_and_external_library_index_and_type_index(3),
            ("inflate", 2, 4)
        );
        assert_eq!(
            external_function_section.get_item_name_and_external_library_index_and_type_index(4),
            ("fopen", 0, 5)
        );
        assert_eq!(
            external_function_section.get_item_name_and_external_library_index_and_type_index(5),
            ("magic_file", 1, 6)
        );
    }
}
