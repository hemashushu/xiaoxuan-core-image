// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use std::io::Write;

use anc_isa::RUNTIME_EDITION;

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
    entry::{ImageCommonEntry, ImageIndexEntry},
    index_sections::{
        data_index_section::DataIndexSection, dynamic_link_module_section::DependentModuleSection,
        entry_point_section::EntryPointSection,
        external_function_index_section::ExternalFunctionIndexSection,
        external_function_section::UnifiedExternalFunctionSection,
        external_library_section::UnifiedExternalLibrarySection,
        external_type_section::UnifiedExternalTypeSection,
        function_index_section::FunctionIndexSection,
    },
    module_image::{ImageType, ModuleImage, SectionEntry},
};

pub fn write_object_file(
    image_common_entry: &ImageCommonEntry,
    generate_shared_module: bool,
    writer: &mut dyn Write,
) -> std::io::Result<()> {
    // property section
    let property_section = PropertySection::new(
        &image_common_entry.name,
        *RUNTIME_EDITION,
        image_common_entry.version.patch,
        image_common_entry.version.minor,
        image_common_entry.version.major,
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

    // export function section
    let (export_function_items, export_function_names_data) =
        ExportFunctionSection::convert_from_entries(&image_common_entry.export_function_entries);
    let export_function_section = ExportFunctionSection {
        items: &export_function_items,
        full_names_data: &export_function_names_data,
    };

    // export data section
    let (export_data_items, export_data_names_data) =
        ExportDataSection::convert_from_entries(&image_common_entry.export_data_entries);
    let export_data_section = ExportDataSection {
        items: &export_data_items,
        full_names_data: &export_data_names_data,
    };

    // relocate section
    let (relocate_lists, relocate_lists_data) =
        RelocateSection::convert_from_entries(&image_common_entry.relocate_list_entries);
    let relocate_section = RelocateSection {
        lists: &relocate_lists,
        list_data: &relocate_lists_data,
    };

    let image_type = if generate_shared_module {
        ImageType::SharedModule
    } else {
        ImageType::ObjectFile
    };

    let section_entries: Vec<&dyn SectionEntry> = vec![
        &property_section,
        //
        &type_section,
        &local_variable_section,
        &function_section,
        //
        &read_only_data_section,
        &read_write_data_section,
        &uninit_data_section,
        //
        &import_module_section,
        &import_function_section,
        &import_data_section,
        //
        &export_function_section,
        &export_data_section,
        &relocate_section,
        //
        &external_library_section,
        &external_function_section,
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

pub fn write_image_file(
    image_common_entry: &ImageCommonEntry,
    image_index_entry: &ImageIndexEntry,
    writer: &mut dyn Write,
) -> std::io::Result<()> {
    // property section
    let property_section = PropertySection::new(
        &image_common_entry.name,
        *RUNTIME_EDITION,
        image_common_entry.version.patch,
        image_common_entry.version.minor,
        image_common_entry.version.major,
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

    // export function section
    let (export_function_items, export_function_names_data) =
        ExportFunctionSection::convert_from_entries(&image_common_entry.export_function_entries);
    let export_function_section = ExportFunctionSection {
        items: &export_function_items,
        full_names_data: &export_function_names_data,
    };

    // export data section
    let (export_data_items, export_data_names_data) =
        ExportDataSection::convert_from_entries(&image_common_entry.export_data_entries);
    let export_data_section = ExportDataSection {
        items: &export_data_items,
        full_names_data: &export_data_names_data,
    };

    // relocate section
    let (relocate_lists, relocate_lists_data) =
        RelocateSection::convert_from_entries(&image_common_entry.relocate_list_entries);
    let relocate_section = RelocateSection {
        lists: &relocate_lists,
        list_data: &relocate_lists_data,
    };

    // function index section
    let (function_ranges, function_index_items) =
        FunctionIndexSection::convert_from_entries(&image_index_entry.function_index_list_entries);
    let function_index_section = FunctionIndexSection {
        ranges: &function_ranges,
        items: &function_index_items,
    };

    // data index section
    let (data_ranges, data_index_items) =
        DataIndexSection::convert_from_entries(&image_index_entry.data_index_list_entries);
    let data_index_section = DataIndexSection {
        ranges: &data_ranges,
        items: &data_index_items,
    };

    // external function index section
    let (external_function_ranges, external_function_index_items) =
        ExternalFunctionIndexSection::convert_from_entries(
            &image_index_entry.external_function_index_entries,
        );
    let external_function_index_section = ExternalFunctionIndexSection {
        ranges: &external_function_ranges,
        items: &external_function_index_items,
    };

    // unified external library section
    let (unified_external_library_items, unified_external_library_data) =
        UnifiedExternalLibrarySection::convert_from_entries(
            &image_index_entry.unified_external_library_entries,
        );
    let unified_external_library_section = UnifiedExternalLibrarySection {
        items: &unified_external_library_items,
        items_data: &unified_external_library_data,
    };

    // unified external type section
    let (unified_external_type_items, unified_external_type_data) =
        UnifiedExternalTypeSection::convert_from_entries(
            &image_index_entry.unified_external_type_entries,
        );
    let unified_external_type_section = UnifiedExternalTypeSection {
        items: &unified_external_type_items,
        types_data: &unified_external_type_data,
    };

    // unified external function section
    let (unified_external_function_items, unified_external_function_data) =
        UnifiedExternalFunctionSection::convert_from_entries(
            &image_index_entry.unified_external_function_entries,
        );
    let unified_external_function_section = UnifiedExternalFunctionSection {
        items: &unified_external_function_items,
        names_data: &unified_external_function_data,
    };

    // dependent module section
    let (dependent_module_items, dependent_module_data) =
        DependentModuleSection::convert_from_entries(&image_index_entry.dependent_module_entries);
    let dependent_module_section = DependentModuleSection {
        items: &dependent_module_items,
        items_data: &dependent_module_data,
    };

    // entry point section
    let (entry_point_items, unit_names_data) =
        EntryPointSection::convert_from_entries(&image_index_entry.entry_point_entries);
    let entry_point_section = EntryPointSection {
        items: &entry_point_items,
        unit_names_data: &unit_names_data,
    };

    let section_entries: Vec<&dyn SectionEntry> = vec![
        /*
         * common
         */
        &property_section,
        //
        &type_section,
        &local_variable_section,
        &function_section,
        //
        &read_only_data_section,
        &read_write_data_section,
        &uninit_data_section,
        //
        &import_module_section,
        &import_function_section,
        &import_data_section,
        //
        &export_function_section,
        &export_data_section,
        &relocate_section,
        //
        &external_library_section,
        &external_function_section,
        /*
         * index
         */
        &function_index_section,
        &data_index_section,
        &external_function_index_section,
        //
        &unified_external_library_section,
        &unified_external_type_section,
        &unified_external_function_section,
        //
        &dependent_module_section,
        &entry_point_section,
    ];

    // build object file binary
    let (section_items, sections_data) =
        ModuleImage::convert_from_section_entries(&section_entries);
    let module_image = ModuleImage {
        image_type: ImageType::Application,
        items: &section_items,
        sections_data: &sections_data,
    };

    // save
    module_image.write(writer)
}
