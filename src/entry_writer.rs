// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use std::io::Write;

use anc_isa::{RUNTIME_MAJOR_VERSION, RUNTIME_MINOR_VERSION};

use crate::{
    common_sections::{
        common_property_section::CommonPropertySection,
        data_name_section::DataNameSection,
        data_section::{ReadOnlyDataSection, ReadWriteDataSection, UninitDataSection},
        external_function_section::ExternalFunctionSection,
        external_library_section::ExternalLibrarySection,
        function_name_section::FunctionNameSection,
        function_section::FunctionSection,
        import_data_section::ImportDataSection,
        import_function_section::ImportFunctionSection,
        import_module_section::ImportModuleSection,
        local_variable_section::LocalVariableSection,
        type_section::TypeSection,
    },
    entry::{ImageCommonEntry, ImageIndexEntry},
    index_sections::{
        data_index_section::DataIndexSection,
        external_function_index_section::ExternalFunctionIndexSection,
        external_function_section::UnifiedExternalFunctionSection,
        external_library_section::UnifiedExternalLibrarySection,
        external_type_section::UnifiedExternalTypeSection,
        function_index_section::FunctionIndexSection, index_property_section::IndexPropertySection,
        module_list_section::ModuleListSection,
    },
    module_image::{ImageType, ModuleImage, SectionEntry},
};

pub fn write_object_file(
    image_common_entry: &ImageCommonEntry,
    generate_shared_module: bool,
    writer: &mut dyn Write,
) -> std::io::Result<()> {
    let image_type = if generate_shared_module {
        ImageType::SharedModule
    } else {
        ImageType::ObjectFile
    };

    write_image_file(image_common_entry, None, image_type, writer)
}

pub fn write_image_file(
    image_common_entry: &ImageCommonEntry,
    image_index_entry_opt: Option<(
        &ImageIndexEntry,
        /* entry_function_public_index */ usize,
    )>,
    image_type: ImageType,
    writer: &mut dyn Write,
) -> std::io::Result<()> {
    // property section
    let common_property_section = CommonPropertySection::new(
        &image_common_entry.name,
        image_common_entry.import_data_entries.len() as u32,
        image_common_entry.import_function_entries.len() as u32,
    );

    // type section
    let (type_items, types_data) =
        TypeSection::convert_from_entries(&image_common_entry.type_entries);
    let type_section = TypeSection {
        items: &type_items,
        types_data: &types_data,
    };

    // local variable section
    let (local_lists, local_list_data) =
        LocalVariableSection::convert_from_entries(&image_common_entry.local_variable_list_entries);
    let local_variable_section = LocalVariableSection {
        lists: &local_lists,
        list_data: &local_list_data,
    };

    // function section
    let (function_items, function_codes_data) =
        FunctionSection::convert_from_entries(&image_common_entry.function_entries);
    let function_section = FunctionSection {
        items: &function_items,
        codes_data: &function_codes_data,
    };

    // ro data section
    let (read_only_data_items, read_only_data) =
        ReadOnlyDataSection::convert_from_entries(&image_common_entry.read_only_data_entries);
    let read_only_data_section = ReadOnlyDataSection {
        items: &read_only_data_items,
        datas_data: &read_only_data,
    };

    // rw data section
    let (read_write_data_items, read_write_data) =
        ReadWriteDataSection::convert_from_entries(&image_common_entry.read_write_data_entries);
    let read_write_data_section = ReadWriteDataSection {
        items: &read_write_data_items,
        datas_data: &read_write_data,
    };

    // uninitialized data section
    let uninit_data_items =
        UninitDataSection::convert_from_entries(&image_common_entry.uninit_data_entries);
    let uninit_data_section = UninitDataSection {
        items: &uninit_data_items,
    };

    // external library section
    let (external_library_items, external_library_names_data) =
        ExternalLibrarySection::convert_from_entries(&image_common_entry.external_library_entries);
    let external_library_section = ExternalLibrarySection {
        items: &external_library_items,
        items_data: &external_library_names_data,
    };

    // external function section
    let (external_function_items, external_function_names_data) =
        ExternalFunctionSection::convert_from_entries(
            &image_common_entry.external_function_entries,
        );
    let external_function_section = ExternalFunctionSection {
        items: &external_function_items,
        names_data: &external_function_names_data,
    };

    // import module section
    let (import_module_items, import_module_data) =
        ImportModuleSection::convert_from_entries(&image_common_entry.import_module_entries);
    let import_module_section = ImportModuleSection {
        items: &import_module_items,
        items_data: &import_module_data,
    };

    // import function section
    let (import_function_items, import_function_data) =
        ImportFunctionSection::convert_from_entries(&image_common_entry.import_function_entries);
    let import_function_section = ImportFunctionSection {
        items: &import_function_items,
        full_names_data: &import_function_data,
    };

    // import data entries
    let (import_data_items, import_data) =
        ImportDataSection::convert_from_entries(&image_common_entry.import_data_entries);
    let import_data_section = ImportDataSection {
        items: &import_data_items,
        full_names_data: &import_data,
    };

    // func name section
    let (function_name_items, function_name_data) =
        FunctionNameSection::convert_from_entries(&image_common_entry.function_name_entries);
    let function_name_section = FunctionNameSection {
        items: &function_name_items,
        full_names_data: &function_name_data,
    };

    // data name section
    let (data_name_items, data_name_data) =
        DataNameSection::convert_from_entries(&image_common_entry.data_name_entries);
    let data_name_section = DataNameSection {
        items: &data_name_items,
        full_names_data: &data_name_data,
    };

    if let Some((image_index_entry, entry_function_public_index)) = image_index_entry_opt {
        let (function_ranges, function_index_items) =
            FunctionIndexSection::convert_from_entries(&image_index_entry.function_index_entries);
        let function_index_section = FunctionIndexSection {
            ranges: &function_ranges,
            items: &function_index_items,
        };

        let (data_ranges, data_index_items) =
            DataIndexSection::convert_from_entries(&image_index_entry.data_index_entries);
        let data_index_section = DataIndexSection {
            ranges: &data_ranges,
            items: &data_index_items,
        };

        let (module_list_items, module_list_data) =
            ModuleListSection::convert_from_entries(&image_index_entry.module_entries);
        let module_list_section = ModuleListSection {
            items: &module_list_items,
            items_data: &module_list_data,
        };

        let (unified_external_library_items, unified_external_library_data) =
            UnifiedExternalLibrarySection::convert_from_entries(
                &image_index_entry.external_library_entries,
            );
        let unified_external_library_section = UnifiedExternalLibrarySection {
            items: &unified_external_library_items,
            items_data: &unified_external_library_data,
        };

        let (unified_external_type_items, unified_external_type_data) =
            UnifiedExternalTypeSection::convert_from_entries(
                &image_index_entry.external_type_entries,
            );
        let external_type_section = UnifiedExternalTypeSection {
            items: &unified_external_type_items,
            types_data: &unified_external_type_data,
        };

        let (unified_external_function_items, unified_external_function_data) =
            UnifiedExternalFunctionSection::convert_from_entries(
                &image_index_entry.external_function_entries,
            );
        let unified_external_function_section = UnifiedExternalFunctionSection {
            items: &unified_external_function_items,
            names_data: &unified_external_function_data,
        };

        let (external_function_ranges, external_function_index_items) =
            ExternalFunctionIndexSection::convert_from_entries(
                &image_index_entry.external_function_index_entries,
            );
        let external_function_index_section = ExternalFunctionIndexSection {
            ranges: &external_function_ranges,
            items: &external_function_index_items,
        };

        let index_property_section = IndexPropertySection {
            entry_function_public_index: entry_function_public_index as u32,
            runtime_major_version: RUNTIME_MAJOR_VERSION,
            runtime_minor_version: RUNTIME_MINOR_VERSION,
        };

        let section_entries: Vec<&dyn SectionEntry> = vec![
            // common
            &type_section,
            &local_variable_section,
            &function_section,
            &read_only_data_section,
            &read_write_data_section,
            &uninit_data_section,
            &external_library_section,
            &external_function_section,
            &import_module_section,
            &import_function_section,
            &import_data_section,
            &function_name_section,
            &data_name_section,
            &common_property_section,
            // index
            &function_index_section,
            &data_index_section,
            &module_list_section,
            &unified_external_library_section,
            &external_type_section,
            &unified_external_function_section,
            &external_function_index_section,
            &index_property_section,
        ];

        // build object file binary
        let (section_items, sections_data) =
            ModuleImage::convert_from_section_entries(&section_entries);
        let module_image = ModuleImage {
            image_type,
            items: &section_items,
            sections_data: &sections_data,
        };

        // save
        module_image.write(writer)
    } else {
        let section_entries: Vec<&dyn SectionEntry> = vec![
            &type_section,
            &local_variable_section,
            &function_section,
            &read_only_data_section,
            &read_write_data_section,
            &uninit_data_section,
            &external_library_section,
            &external_function_section,
            &import_module_section,
            &import_function_section,
            &import_data_section,
            &function_name_section,
            &data_name_section,
            &common_property_section,
        ];

        // build object file binary
        let (section_items, sections_data) =
            ModuleImage::convert_from_section_entries(&section_entries);
        let module_image = ModuleImage {
            image_type,
            items: &section_items,
            sections_data: &sections_data,
        };

        // save
        module_image.write(writer)
    }
}
