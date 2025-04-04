// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use crate::common_sections::data_section::{
    ReadOnlyDataSection, ReadWriteDataSection, UninitDataSection,
};
use crate::common_sections::export_data_section::ExportDataSection;
use crate::common_sections::export_function_section::ExportFunctionSection;
use crate::common_sections::external_function_section::ExternalFunctionSection;
use crate::common_sections::external_library_section::ExternalLibrarySection;
use crate::common_sections::function_section::FunctionSection;
use crate::common_sections::local_variable_section::LocalVariableSection;
use crate::common_sections::property_section::PropertySection;
use crate::common_sections::type_section::TypeSection;
use crate::index_sections::data_index_section::{DataIndexItem, DataIndexSection};
use crate::index_sections::dynamic_link_module_section::DynamicLinkModuleSection;
use crate::index_sections::entry_point_section::EntryPointSection;
use crate::index_sections::external_function_index_section::{
    ExternalFunctionIndexItem, ExternalFunctionIndexSection,
};
use crate::index_sections::external_function_section::UnifiedExternalFunctionSection;
use crate::index_sections::external_library_section::UnifiedExternalLibrarySection;
use crate::index_sections::external_type_section::UnifiedExternalTypeSection;
use crate::index_sections::function_index_section::{FunctionIndexItem, FunctionIndexSection};
use crate::ImageError;

use anc_isa::{DataSectionType, OperandDataType, RUNTIME_EDITION};

use crate::entry::{
    DynamicLinkModuleEntry, EntryPointEntry, ExportDataEntry, ExportFunctionEntry,
    ExternalFunctionEntry, ExternalLibraryEntry, FunctionEntry, InitedDataEntry,
    LocalVariableEntry, LocalVariableListEntry, ModuleLocation, TypeEntry, UninitDataEntry,
};

use crate::module_image::{ImageType, ModuleImage, RangeItem, SectionEntry, Visibility};

/// helper object for unit test
pub struct HelperFunctionEntry {
    pub params: Vec<OperandDataType>,
    pub results: Vec<OperandDataType>,
    pub local_variable_item_entries_without_args: Vec<LocalVariableEntry>,
    pub code: Vec<u8>,
}

/// helper object for unit test
pub struct HelperBlockEntry {
    pub params: Vec<OperandDataType>,
    pub results: Vec<OperandDataType>,
    pub local_variable_item_entries_without_args: Vec<LocalVariableEntry>,
}

/// helper object for unit test
pub struct HelperExternalFunctionEntry {
    pub name: String,
    pub external_library_index: usize,
    pub params: Vec<OperandDataType>,
    pub result: Option<OperandDataType>,
}

/// helper function for unit test
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
        &[],
        &[],
        &[],
    )
}

/// helper function for unit test
pub fn helper_build_module_binary_with_single_function_and_data(
    param_datatypes: &[OperandDataType],
    result_datatypes: &[OperandDataType],
    local_variable_entries_without_function_args: &[LocalVariableEntry],
    code: Vec<u8>,
    read_only_data_entries: &[InitedDataEntry],
    read_write_data_entries: &[InitedDataEntry],
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

/// helper function for unit test
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

/// helper function for unit test
pub fn helper_build_module_binary_with_functions_and_blocks(
    helper_function_entries: &[HelperFunctionEntry],
    helper_block_entries: &[HelperBlockEntry],
) -> Vec<u8> {
    // build type entries

    // note:
    // for simplicity, duplicate items would not be merged.

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

    // build local variables list entries

    // note:
    // for simplicity, duplicate items would be be merged.

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

    // build function entries
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

/// helper function for unit test
#[allow(clippy::too_many_arguments)]
pub fn helper_build_module_binary_with_functions_and_data_and_external_functions(
    // type_entries: Vec<TypeEntry>,
    helper_function_entries: &[HelperFunctionEntry],
    read_only_data_entries: &[InitedDataEntry],
    read_write_data_entries: &[InitedDataEntry],
    uninit_uninit_data_entries: &[UninitDataEntry],
    external_library_entries: &[ExternalLibraryEntry],
    helper_external_function_entries: &[HelperExternalFunctionEntry],
) -> Vec<u8> {
    // note:
    // for simplicity, duplicate items would not be merged.

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

    // build local variables list entries

    // note:
    // for simplicity, duplicate items would be be merged.

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

    // build function entries
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
        // helper_external_function_entries,
        external_library_entries,
        &external_function_entries,
        0,
    )
}

/// helper function for unit test
#[allow(clippy::too_many_arguments)]
pub fn helper_build_module_binary(
    name: &str,
    read_only_data_entries: &[InitedDataEntry],
    read_write_data_entries: &[InitedDataEntry],
    uninit_uninit_data_entries: &[UninitDataEntry],
    type_entries: &[TypeEntry],
    local_list_entries: &[LocalVariableListEntry], // this local list includes function/block args
    function_entries: &[FunctionEntry],
    external_library_entries: &[ExternalLibraryEntry],
    external_function_entries: &[ExternalFunctionEntry],
    entry_function_public_index: usize,
) -> Vec<u8> {
    // type section
    let (type_items, types_data) = TypeSection::convert_from_entries(type_entries);
    let type_section = TypeSection {
        items: &type_items,
        types_data: &types_data,
    };

    // local variable section
    let (local_lists, local_list_data) =
        LocalVariableSection::convert_from_entries(local_list_entries);
    let local_variable_section = LocalVariableSection {
        lists: &local_lists,
        list_data: &local_list_data,
    };

    // function section
    let (function_items, codes_data) = FunctionSection::convert_from_entries(function_entries);
    let function_section = FunctionSection {
        items: &function_items,
        codes_data: &codes_data,
    };

    // read-only data section
    let (ro_items, ro_data) = ReadOnlyDataSection::convert_from_entries(read_only_data_entries);
    let ro_data_section = ReadOnlyDataSection {
        items: &ro_items,
        datas_data: &ro_data,
    };

    // read-write data section
    let (rw_items, rw_data) = ReadWriteDataSection::convert_from_entries(read_write_data_entries);
    let rw_data_section = ReadWriteDataSection {
        items: &rw_items,
        datas_data: &rw_data,
    };

    // uninitilized data section
    let uninit_items = UninitDataSection::convert_from_entries(uninit_uninit_data_entries);
    let uninit_data_section = UninitDataSection {
        items: &uninit_items,
    };

    // export function section
    // for simplicity, these are abitray items
    let (export_function_items, export_function_names_data) =
        ExportFunctionSection::convert_from_entries(&[
            ExportFunctionEntry::new("func0".to_owned(), Visibility::Public),
            ExportFunctionEntry::new("func1".to_owned(), Visibility::Public),
        ]);

    let export_function_section = ExportFunctionSection {
        items: &export_function_items,
        full_names_data: &export_function_names_data,
    };

    // export data section
    // for simplicity, these are abitray items
    let (export_data_items, export_data_names_data) = ExportDataSection::convert_from_entries(&[
        ExportDataEntry::new(
            "data0".to_owned(),
            Visibility::Public,
            DataSectionType::ReadWrite,
        ),
        ExportDataEntry::new(
            "data1".to_owned(),
            Visibility::Public,
            DataSectionType::ReadWrite,
        ),
    ]);

    let export_data_section = ExportDataSection {
        items: &export_data_items,
        full_names_data: &export_data_names_data,
    };

    // external library section
    let (external_library_items, external_library_data) =
        ExternalLibrarySection::convert_from_entries(external_library_entries);
    let external_library_section = ExternalLibrarySection {
        items: &external_library_items,
        items_data: &external_library_data,
    };

    // external function section
    let (external_function_items, external_function_data) =
        ExternalFunctionSection::convert_from_entries(external_function_entries);
    let external_function_section = ExternalFunctionSection {
        items: &external_function_items,
        names_data: &external_function_data,
    };

    // property section
    let property_section = PropertySection::new(name, *RUNTIME_EDITION, 0, 0, 1, 0, 0);

    // function index
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

    // data index

    // the data index is ordered by:
    // 1. imported ro data
    // 2. imported rw data
    // 3. imported uninit data
    // 4. ro data
    // 5. rw data
    // 6. uninit data
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
        data_index_items.push(DataIndexItem::new(0, idx as u32, data_section_type));
    }

    let data_index_section = DataIndexSection {
        ranges: &data_ranges,
        items: &data_index_items,
    };

    // unified external library section
    // for simplicity, build 1:1 to the external_library_entries
    let unified_external_library_entries = external_library_entries;
    let (unified_external_library_items, unified_external_library_data) =
        UnifiedExternalLibrarySection::convert_from_entries(unified_external_library_entries);
    let unified_external_library_section = UnifiedExternalLibrarySection {
        items: &unified_external_library_items,
        items_data: &unified_external_library_data,
    };

    // unified external type section
    // for simplicity, build 1:1 to type_entries
    let unified_external_type_entries = type_entries;
    let (unified_external_type_items, unified_external_type_data) =
        UnifiedExternalTypeSection::convert_from_entries(unified_external_type_entries);
    let unified_external_type_section = UnifiedExternalTypeSection {
        items: &unified_external_type_items,
        types_data: &unified_external_type_data,
    };

    // unified external function section
    // for simplicity, build 1:1 to external_function_entries
    let unified_external_function_entries = external_function_entries;
    let (unified_external_function_items, unified_external_function_data) =
        UnifiedExternalFunctionSection::convert_from_entries(unified_external_function_entries);
    let unified_external_function_section = UnifiedExternalFunctionSection {
        items: &unified_external_function_items,
        names_data: &unified_external_function_data,
    };

    // external function index section
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

    // entry point section
    let entry_point_entries = vec![EntryPointEntry::new(
        "".to_string(), // the name of default entry point is empty string
        entry_function_public_index,
    )];
    let (entry_point_items, unit_names_data) =
        EntryPointSection::convert_from_entries(&entry_point_entries);
    let entry_point_section = EntryPointSection {
        items: &entry_point_items,
        unit_names_data: &unit_names_data,
    };

    // dynamic link module list
    let import_module_entry =
        DynamicLinkModuleEntry::new(name.to_owned(), Box::new(ModuleLocation::Embed));
    let (module_list_items, module_list_data) =
        DynamicLinkModuleSection::convert_from_entries(&[import_module_entry]);
    let module_list_section = DynamicLinkModuleSection {
        items: &module_list_items,
        items_data: &module_list_data,
    };

    // build module image
    let section_entries: Vec<&dyn SectionEntry> = vec![
        /* the following are common sections */
        &property_section,
        &type_section,
        &local_variable_section,
        &function_section,
        &ro_data_section,
        &rw_data_section,
        &uninit_data_section,
        &export_function_section,
        &export_data_section,
        /* empty sections: import_module, import_function, import_data */
        &external_library_section,
        &external_function_section,
        /* the following are index sections */
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

    // build module image binary
    let mut image_binary: Vec<u8> = vec![];
    module_image.write(&mut image_binary).unwrap();
    image_binary
}

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
        common_sections::{data_section::DataItem, local_variable_section::LocalVariableItem},
        entry::{ExternalLibraryEntry, InitedDataEntry, LocalVariableEntry, UninitDataEntry},
        index_sections::{
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
        let binary = helper_build_module_binary_with_single_function_and_data(
            &[OperandDataType::I64, OperandDataType::I64],
            &[OperandDataType::I32],
            &[LocalVariableEntry::from_i32()],
            vec![0u8],
            &[
                InitedDataEntry::from_i32(0x11),
                InitedDataEntry::from_i64(0x13),
            ],
            &[InitedDataEntry::from_bytes(
                vec![0x17u8, 0x19, 0x23, 0x29, 0x31, 0x37],
                8,
            )],
            &[
                UninitDataEntry::from_i32(),
                UninitDataEntry::from_i64(),
                UninitDataEntry::from_i32(),
            ],
        );

        // load module
        let module_images = helper_load_modules_from_binaries(&[&binary]).unwrap();
        assert_eq!(module_images.len(), 1);

        // check module image
        let module_image = &module_images[0];

        // check data index section
        let data_index_section = module_image.get_optional_data_index_section().unwrap();
        assert_eq!(data_index_section.ranges.len(), 1);
        assert_eq!(data_index_section.items.len(), 6);

        assert_eq!(&data_index_section.ranges[0], &RangeItem::new(0, 6));

        assert_eq!(
            data_index_section.items,
            &[
                //
                DataIndexItem::new(0, 0, DataSectionType::ReadOnly,),
                DataIndexItem::new(0, 1, DataSectionType::ReadOnly,),
                //
                DataIndexItem::new(0, 0, DataSectionType::ReadWrite,),
                //
                DataIndexItem::new(0, 0, DataSectionType::Uninit,),
                DataIndexItem::new(0, 1, DataSectionType::Uninit,),
                DataIndexItem::new(0, 2, DataSectionType::Uninit,),
            ]
        );

        // check function index section
        let function_index_section = module_image.get_function_index_section();
        assert_eq!(function_index_section.ranges.len(), 1);
        assert_eq!(function_index_section.items.len(), 1);

        assert_eq!(&function_index_section.ranges[0], &RangeItem::new(0, 1));

        assert_eq!(
            function_index_section.items,
            &[FunctionIndexItem::new(0, 0)]
        );

        // check data sections
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
            &DataItem::new(0, 6, MemoryDataType::Bytes, 8)
        );
        assert_eq!(
            &rw_section.datas_data[rw_section.items[0].data_offset as usize..][0..6],
            &[0x17u8, 0x19, 0x23, 0x29, 0x31, 0x37]
        );

        let uninit_section = module_image.get_optional_uninit_data_section().unwrap();
        assert_eq!(
            &uninit_section.items[0],
            &DataItem::new(0, 4, MemoryDataType::I32, 4)
        );
        assert_eq!(
            &uninit_section.items[1],
            &DataItem::new(8, 8, MemoryDataType::I64, 8)
        );
        assert_eq!(
            &uninit_section.items[2],
            &DataItem::new(16, 4, MemoryDataType::I32, 4)
        );

        // check type section
        let type_section = module_image.get_type_section();
        assert_eq!(type_section.items.len(), 1);
        assert_eq!(
            type_section.get_item_params_and_results(0),
            (
                &[OperandDataType::I64, OperandDataType::I64][..],
                &[OperandDataType::I32][..]
            )
        );

        // check function section
        let function_section = module_image.get_function_section();
        assert_eq!(function_section.items.len(), 1);

        assert_eq!(
            function_section.get_item_type_index_and_local_variable_list_index_and_code(0),
            (0, 0, vec![0u8].as_ref())
        );

        // check local variable section
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
        // TODO
    }

    #[test]
    fn test_build_module_binary_with_single_function_and_external_functions() {
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

        // load module
        let module_images = helper_load_modules_from_binaries(&[&binary]).unwrap();
        assert_eq!(module_images.len(), 1);

        let module_image = &module_images[0];

        // check unified external library section
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

        // check unified external function section
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

        // check external function index section
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

        // check external library sections
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

        // check external function section
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
