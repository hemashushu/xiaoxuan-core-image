// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use anc_isa::EffectiveVersion;

use crate::{
    entry::{ImageCommonEntry, ImageIndexEntry},
    module_image::ModuleImage,
    ImageError,
};

pub fn read_object_file(object_binary: &[u8]) -> Result<ImageCommonEntry, ImageError> {
    let module_image = ModuleImage::read(object_binary)?;

    let type_entries = module_image.get_type_section().convert_to_entries();
    let local_variable_list_entries = module_image
        .get_local_variable_section()
        .convert_to_entries();
    let function_entries = module_image.get_function_section().convert_to_entries();
    let read_only_data_entries = module_image
        .get_optional_read_only_data_section()
        .unwrap_or_default()
        .convert_to_entries();
    let read_write_data_entries = module_image
        .get_optional_read_write_data_section()
        .unwrap_or_default()
        .convert_to_entries();
    let uninit_data_entries = module_image
        .get_optional_uninit_data_section()
        .unwrap_or_default()
        .convert_to_entries();
    let external_library_entries = module_image
        .get_optional_external_library_section()
        .unwrap_or_default()
        .convert_to_entries();
    let external_function_entries = module_image
        .get_optional_external_function_section()
        .unwrap_or_default()
        .convert_to_entries();
    let import_module_entries = module_image
        .get_optional_import_module_section()
        .unwrap_or_default()
        .convert_to_entries();
    let import_function_entries = module_image
        .get_optional_import_function_section()
        .unwrap_or_default()
        .convert_to_entries();
    let import_data_entries = module_image
        .get_optional_import_data_section()
        .unwrap_or_default()
        .convert_to_entries();
    let export_function_entries = module_image
        .get_optional_export_function_section()
        .unwrap_or_default()
        .convert_to_entries();
    let export_data_entries = module_image
        .get_optional_export_data_section()
        .unwrap_or_default()
        .convert_to_entries();
    let relocate_list_entries = module_image
        .get_optional_relocate_section()
        .unwrap_or_default()
        .convert_to_entries();

    let property_section = module_image.get_property_section();

    let image_common_entry = ImageCommonEntry {
        name: property_section.get_module_name().to_owned(),
        version: EffectiveVersion::new(
            property_section.version_major,
            property_section.version_minor,
            property_section.version_patch,
        ),
        image_type: module_image.image_type,
        //
        type_entries,
        local_variable_list_entries,
        function_entries,
        //
        read_only_data_entries,
        read_write_data_entries,
        uninit_data_entries,
        //
        import_module_entries,
        import_function_entries,
        import_data_entries,
        //
        export_function_entries,
        export_data_entries,
        relocate_list_entries,
        //
        external_library_entries,
        external_function_entries,
    };

    Ok(image_common_entry)
}

pub fn read_image_file(
    image_binary: &[u8],
) -> Result<(ImageCommonEntry, ImageIndexEntry), ImageError> {
    let module_image = ModuleImage::read(image_binary)?;

    let type_entries = module_image.get_type_section().convert_to_entries();
    let local_variable_list_entries = module_image
        .get_local_variable_section()
        .convert_to_entries();
    let function_entries = module_image.get_function_section().convert_to_entries();
    let read_only_data_entries = module_image
        .get_optional_read_only_data_section()
        .unwrap_or_default()
        .convert_to_entries();
    let read_write_data_entries = module_image
        .get_optional_read_write_data_section()
        .unwrap_or_default()
        .convert_to_entries();
    let uninit_data_entries = module_image
        .get_optional_uninit_data_section()
        .unwrap_or_default()
        .convert_to_entries();
    let external_library_entries = module_image
        .get_optional_external_library_section()
        .unwrap_or_default()
        .convert_to_entries();
    let external_function_entries = module_image
        .get_optional_external_function_section()
        .unwrap_or_default()
        .convert_to_entries();
    let import_module_entries = module_image
        .get_optional_import_module_section()
        .unwrap_or_default()
        .convert_to_entries();
    let import_function_entries = module_image
        .get_optional_import_function_section()
        .unwrap_or_default()
        .convert_to_entries();
    let import_data_entries = module_image
        .get_optional_import_data_section()
        .unwrap_or_default()
        .convert_to_entries();
    let export_function_entries = module_image
        .get_optional_export_function_section()
        .unwrap_or_default()
        .convert_to_entries();
    let export_data_entries = module_image
        .get_optional_export_data_section()
        .unwrap_or_default()
        .convert_to_entries();
    let relocate_list_entries = module_image
        .get_optional_relocate_section()
        .unwrap_or_default()
        .convert_to_entries();

    let property_section = module_image.get_property_section();

    let image_common_entry = ImageCommonEntry {
        name: property_section.get_module_name().to_owned(),
        version: EffectiveVersion::new(
            property_section.version_major,
            property_section.version_minor,
            property_section.version_patch,
        ),
        image_type: module_image.image_type,
        //
        type_entries,
        local_variable_list_entries,
        function_entries,
        //
        read_only_data_entries,
        read_write_data_entries,
        uninit_data_entries,
        //
        import_module_entries,
        import_function_entries,
        import_data_entries,
        //
        export_function_entries,
        export_data_entries,
        relocate_list_entries,
        //
        external_library_entries,
        external_function_entries,
    };

    // function index section
    let function_index_list_entries = module_image
        .get_function_index_section()
        .convert_to_entries();

    // data index section
    let data_index_list_entries = module_image
        .get_optional_data_index_section()
        .unwrap_or_default()
        .convert_to_entries();

    // external function index section
    let external_function_index_entries = module_image
        .get_optional_external_function_index_section()
        .unwrap_or_default()
        .convert_to_entries();

    // unified external library section
    let unified_external_library_entries = module_image
        .get_optional_external_library_section()
        .unwrap_or_default()
        .convert_to_entries();

    // unified external type section
    let unified_external_type_entries = module_image
        .get_optional_unified_external_type_section()
        .unwrap_or_default()
        .convert_to_entries();

    // unified external function section
    let unified_external_function_entries = module_image
        .get_optional_external_function_section()
        .unwrap_or_default()
        .convert_to_entries();

    // dependent module section
    let dynamic_link_module_entries = module_image
        .get_dynamic_link_module_list_section()
        .convert_to_entries();

    // entry point section
    let entry_point_entries = module_image.get_entry_point_section().convert_to_entries();

    let image_index_entry = ImageIndexEntry {
        function_index_list_entries,
        data_index_list_entries,
        external_function_index_entries,
        unified_external_library_entries,
        unified_external_type_entries,
        unified_external_function_entries,
        dynamic_link_module_entries,
        entry_point_entries,
    };

    Ok((image_common_entry, image_index_entry))
}
