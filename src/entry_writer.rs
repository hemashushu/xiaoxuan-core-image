// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of the Mozilla Public License version 2.0 and additional exceptions.
// More details can be found in the LICENSE, LICENSE.additional, and CONTRIBUTING files.

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
        data_index_section::DataIndexSection,
        dynamic_link_module_section::DynamicLinkModuleSection,
        entry_point_section::EntryPointSection,
        external_function_index_section::ExternalFunctionIndexSection,
        external_function_section::UnifiedExternalFunctionSection,
        external_library_section::UnifiedExternalLibrarySection,
        external_type_section::UnifiedExternalTypeSection,
        function_index_section::FunctionIndexSection,
    },
    module_image::{ImageType, ModuleImage, SectionEntry},
};

// Writes an object file based on the provided ImageCommonEntry.
// If `generate_shared_module` is true, the output will be a shared module; otherwise, it will be an object file.
pub fn write_object_file(
    image_common_entry: &ImageCommonEntry,
    generate_shared_module: bool,
    writer: &mut dyn Write,
) -> std::io::Result<()> {
    // Create the property section with metadata about the image.
    let property_section = PropertySection::new(
        &image_common_entry.name,
        *RUNTIME_EDITION,
        image_common_entry.version.patch,
        image_common_entry.version.minor,
        image_common_entry.version.major,
        image_common_entry.import_data_entries.len() as u32,
        image_common_entry.import_function_entries.len() as u32,
    );

    // Convert and prepare all sections from the ImageCommonEntry.
    // Type section
    let (type_items, types_data) =
        TypeSection::convert_from_entries(&image_common_entry.type_entries);
    let type_section = TypeSection {
        items: &type_items,
        types_data: &types_data,
    };

    // Local variable section
    let (local_lists, local_list_data) =
        LocalVariableSection::convert_from_entries(&image_common_entry.local_variable_list_entries);
    let local_variable_section = LocalVariableSection {
        lists: &local_lists,
        list_data: &local_list_data,
    };

    // Function section
    let (function_items, function_codes_data) =
        FunctionSection::convert_from_entries(&image_common_entry.function_entries);
    let function_section = FunctionSection {
        items: &function_items,
        codes_data: &function_codes_data,
    };

    // Read-only data section
    let (read_only_data_items, read_only_data) =
        ReadOnlyDataSection::convert_from_entries(&image_common_entry.read_only_data_entries);
    let read_only_data_section = ReadOnlyDataSection {
        items: &read_only_data_items,
        datas_data: &read_only_data,
    };

    // Read-write data section
    let (read_write_data_items, read_write_data) =
        ReadWriteDataSection::convert_from_entries(&image_common_entry.read_write_data_entries);
    let read_write_data_section = ReadWriteDataSection {
        items: &read_write_data_items,
        datas_data: &read_write_data,
    };

    // Uninitialized data section
    let uninit_data_items =
        UninitDataSection::convert_from_entries(&image_common_entry.uninit_data_entries);
    let uninit_data_section = UninitDataSection {
        items: &uninit_data_items,
    };

    // External library section
    let (external_library_items, external_library_names_data) =
        ExternalLibrarySection::convert_from_entries(&image_common_entry.external_library_entries);
    let external_library_section = ExternalLibrarySection {
        items: &external_library_items,
        items_data: &external_library_names_data,
    };

    // External function section
    let (external_function_items, external_function_names_data) =
        ExternalFunctionSection::convert_from_entries(
            &image_common_entry.external_function_entries,
        );
    let external_function_section = ExternalFunctionSection {
        items: &external_function_items,
        names_data: &external_function_names_data,
    };

    // Import module section
    let (import_module_items, import_module_data) =
        ImportModuleSection::convert_from_entries(&image_common_entry.import_module_entries);
    let import_module_section = ImportModuleSection {
        items: &import_module_items,
        items_data: &import_module_data,
    };

    // Import function section
    let (import_function_items, import_function_data) =
        ImportFunctionSection::convert_from_entries(&image_common_entry.import_function_entries);
    let import_function_section = ImportFunctionSection {
        items: &import_function_items,
        full_names_data: &import_function_data,
    };

    // Import data section
    let (import_data_items, import_data) =
        ImportDataSection::convert_from_entries(&image_common_entry.import_data_entries);
    let import_data_section = ImportDataSection {
        items: &import_data_items,
        full_names_data: &import_data,
    };

    // Export function section
    let (export_function_items, export_function_names_data) =
        ExportFunctionSection::convert_from_entries(&image_common_entry.export_function_entries);
    let export_function_section = ExportFunctionSection {
        items: &export_function_items,
        full_names_data: &export_function_names_data,
    };

    // Export data section
    let (export_data_items, export_data_names_data) =
        ExportDataSection::convert_from_entries(&image_common_entry.export_data_entries);
    let export_data_section = ExportDataSection {
        items: &export_data_items,
        full_names_data: &export_data_names_data,
    };

    // Relocate section
    let (relocate_lists, relocate_lists_data) =
        RelocateSection::convert_from_entries(&image_common_entry.relocate_list_entries);
    let relocate_section = RelocateSection {
        lists: &relocate_lists,
        list_data: &relocate_lists_data,
    };

    // Determine the image type based on the `generate_shared_module` flag.
    let image_type = if generate_shared_module {
        ImageType::SharedModule
    } else {
        ImageType::ObjectFile
    };

    // Collect all section entries into a vector.
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

    // Build the object file binary from the section entries.
    let (section_items, sections_data) =
        ModuleImage::convert_from_section_entries(&section_entries);
    let module_image = ModuleImage {
        image_type,
        items: &section_items,
        sections_data: &sections_data,
    };

    // Write the binary data to the provided writer.
    module_image.write(writer)
}

// Writes an image file based on the provided ImageCommonEntry and ImageIndexEntry.
// This function generates a complete application image.
pub fn write_image_file(
    image_common_entry: &ImageCommonEntry,
    image_index_entry: &ImageIndexEntry,
    writer: &mut dyn Write,
) -> std::io::Result<()> {
    // Create the property section with metadata about the image.
    let property_section = PropertySection::new(
        &image_common_entry.name,
        *RUNTIME_EDITION,
        image_common_entry.version.patch,
        image_common_entry.version.minor,
        image_common_entry.version.major,
        image_common_entry.import_data_entries.len() as u32,
        image_common_entry.import_function_entries.len() as u32,
    );

    // Convert and prepare all sections from the ImageCommonEntry.
    // Type section
    let (type_items, types_data) =
        TypeSection::convert_from_entries(&image_common_entry.type_entries);
    let type_section = TypeSection {
        items: &type_items,
        types_data: &types_data,
    };

    // Local variable section
    let (local_lists, local_list_data) =
        LocalVariableSection::convert_from_entries(&image_common_entry.local_variable_list_entries);
    let local_variable_section = LocalVariableSection {
        lists: &local_lists,
        list_data: &local_list_data,
    };

    // Function section
    let (function_items, function_codes_data) =
        FunctionSection::convert_from_entries(&image_common_entry.function_entries);
    let function_section = FunctionSection {
        items: &function_items,
        codes_data: &function_codes_data,
    };

    // Read-only data section
    let (read_only_data_items, read_only_data) =
        ReadOnlyDataSection::convert_from_entries(&image_common_entry.read_only_data_entries);
    let read_only_data_section = ReadOnlyDataSection {
        items: &read_only_data_items,
        datas_data: &read_only_data,
    };

    // Read-write data section
    let (read_write_data_items, read_write_data) =
        ReadWriteDataSection::convert_from_entries(&image_common_entry.read_write_data_entries);
    let read_write_data_section = ReadWriteDataSection {
        items: &read_write_data_items,
        datas_data: &read_write_data,
    };

    // Uninitialized data section
    let uninit_data_items =
        UninitDataSection::convert_from_entries(&image_common_entry.uninit_data_entries);
    let uninit_data_section = UninitDataSection {
        items: &uninit_data_items,
    };

    // External library section
    let (external_library_items, external_library_names_data) =
        ExternalLibrarySection::convert_from_entries(&image_common_entry.external_library_entries);
    let external_library_section = ExternalLibrarySection {
        items: &external_library_items,
        items_data: &external_library_names_data,
    };

    // External function section
    let (external_function_items, external_function_names_data) =
        ExternalFunctionSection::convert_from_entries(
            &image_common_entry.external_function_entries,
        );
    let external_function_section = ExternalFunctionSection {
        items: &external_function_items,
        names_data: &external_function_names_data,
    };

    // Import module section
    let (import_module_items, import_module_data) =
        ImportModuleSection::convert_from_entries(&image_common_entry.import_module_entries);
    let import_module_section = ImportModuleSection {
        items: &import_module_items,
        items_data: &import_module_data,
    };

    // Import function section
    let (import_function_items, import_function_data) =
        ImportFunctionSection::convert_from_entries(&image_common_entry.import_function_entries);
    let import_function_section = ImportFunctionSection {
        items: &import_function_items,
        full_names_data: &import_function_data,
    };

    // Import data section
    let (import_data_items, import_data) =
        ImportDataSection::convert_from_entries(&image_common_entry.import_data_entries);
    let import_data_section = ImportDataSection {
        items: &import_data_items,
        full_names_data: &import_data,
    };

    // Export function section
    let (export_function_items, export_function_names_data) =
        ExportFunctionSection::convert_from_entries(&image_common_entry.export_function_entries);
    let export_function_section = ExportFunctionSection {
        items: &export_function_items,
        full_names_data: &export_function_names_data,
    };

    // Export data section
    let (export_data_items, export_data_names_data) =
        ExportDataSection::convert_from_entries(&image_common_entry.export_data_entries);
    let export_data_section = ExportDataSection {
        items: &export_data_items,
        full_names_data: &export_data_names_data,
    };

    // Relocate section
    let (relocate_lists, relocate_lists_data) =
        RelocateSection::convert_from_entries(&image_common_entry.relocate_list_entries);
    let relocate_section = RelocateSection {
        lists: &relocate_lists,
        list_data: &relocate_lists_data,
    };

    // Convert and prepare all index-specific sections from the ImageIndexEntry.
    // Function index section
    let (function_ranges, function_index_items) =
        FunctionIndexSection::convert_from_entries(&image_index_entry.function_index_list_entries);
    let function_index_section = FunctionIndexSection {
        ranges: &function_ranges,
        items: &function_index_items,
    };

    // Data index section
    let (data_ranges, data_index_items) =
        DataIndexSection::convert_from_entries(&image_index_entry.data_index_list_entries);
    let data_index_section = DataIndexSection {
        ranges: &data_ranges,
        items: &data_index_items,
    };

    // External function index section
    let (external_function_ranges, external_function_index_items) =
        ExternalFunctionIndexSection::convert_from_entries(
            &image_index_entry.external_function_index_entries,
        );
    let external_function_index_section = ExternalFunctionIndexSection {
        ranges: &external_function_ranges,
        items: &external_function_index_items,
    };

    // Unified external library section
    let (unified_external_library_items, unified_external_library_data) =
        UnifiedExternalLibrarySection::convert_from_entries(
            &image_index_entry.unified_external_library_entries,
        );
    let unified_external_library_section = UnifiedExternalLibrarySection {
        items: &unified_external_library_items,
        items_data: &unified_external_library_data,
    };

    // Unified external type section
    let (unified_external_type_items, unified_external_type_data) =
        UnifiedExternalTypeSection::convert_from_entries(
            &image_index_entry.unified_external_type_entries,
        );
    let unified_external_type_section = UnifiedExternalTypeSection {
        items: &unified_external_type_items,
        types_data: &unified_external_type_data,
    };

    // Unified external function section
    let (unified_external_function_items, unified_external_function_data) =
        UnifiedExternalFunctionSection::convert_from_entries(
            &image_index_entry.unified_external_function_entries,
        );
    let unified_external_function_section = UnifiedExternalFunctionSection {
        items: &unified_external_function_items,
        names_data: &unified_external_function_data,
    };

    // Dynamic link module section
    let (dynamic_link_module_items, dynamic_link_module_data) =
        DynamicLinkModuleSection::convert_from_entries(
            &image_index_entry.dynamic_link_module_entries,
        );
    let dynamic_link_module_section = DynamicLinkModuleSection {
        items: &dynamic_link_module_items,
        items_data: &dynamic_link_module_data,
    };

    // Entry point section
    let (entry_point_items, unit_names_data) =
        EntryPointSection::convert_from_entries(&image_index_entry.entry_point_entries);
    let entry_point_section = EntryPointSection {
        items: &entry_point_items,
        unit_names_data: &unit_names_data,
    };

    // Collect all section entries, including both common and index-specific sections.
    let section_entries: Vec<&dyn SectionEntry> = vec![
        /*
         * Common sections
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
         * Index-specific sections
         */
        &function_index_section,
        &data_index_section,
        &external_function_index_section,
        //
        &unified_external_library_section,
        &unified_external_type_section,
        &unified_external_function_section,
        //
        &dynamic_link_module_section,
        &entry_point_section,
    ];

    // Build the application image binary from the section entries.
    let (section_items, sections_data) =
        ModuleImage::convert_from_section_entries(&section_entries);
    let module_image = ModuleImage {
        image_type: ImageType::Application,
        items: &section_items,
        sections_data: &sections_data,
    };

    // Write the binary data to the provided writer.
    module_image.write(writer)
}
