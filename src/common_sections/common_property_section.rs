// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use crate::module_image::{ImageType, ModuleSectionId, SectionEntry};

#[repr(C)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct CommonPropertySection {
    pub image_type: ImageType,

    // // for linking
    // pub constructor_function_public_index: u32, // u32::max for None
    // pub destructor_function_public_index: u32,  // u32::max for None

    // the "module name", "import data count" and "import function count" are
    // used for find the public index of function and data in
    // the bridge function call.
    //
    // it's also possible to get these information from the `import*`
    // sections, but they are optional in the runtime.
    pub import_data_count: u32,
    pub import_function_count: u32,

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
    pub module_name_length: u32,
    pub module_name_buffer: [u8; 256],
}

impl<'a> SectionEntry<'a> for CommonPropertySection {
    fn load(section_data: &'a [u8]) -> Self {
        let property_section_ptr = unsafe {
            std::mem::transmute::<*const u8, *const CommonPropertySection>(section_data.as_ptr())
        };

        unsafe { *property_section_ptr }
    }

    fn save(&'a self, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        let mut data = [0u8; std::mem::size_of::<CommonPropertySection>()];
        let src = self as *const CommonPropertySection as *const u8;
        let dst = data.as_mut_ptr();
        unsafe { std::ptr::copy(src, dst, data.len()) };

        writer.write_all(&data)
    }

    fn id(&'a self) -> ModuleSectionId {
        ModuleSectionId::CommonProperty
    }
}

#[cfg(test)]
mod tests {
    use crate::module_image::{ImageType, SectionEntry};

    use super::CommonPropertySection;

    #[test]
    fn test_save_section() {
        let mut module_name_buffer = [0u8; 256];
        module_name_buffer[0] = 29;
        module_name_buffer[1] = 31;
        module_name_buffer[2] = 37;

        let section = CommonPropertySection {
            image_type: ImageType::SharedModule,
            // constructor_function_public_index: 11,
            // destructor_function_public_index: 13,
            import_data_count: 17,
            import_function_count: 19,
            module_name_length: 3,
            module_name_buffer,
        };

        let mut section_data: Vec<u8> = Vec::new();
        section.save(&mut section_data).unwrap();

        let mut expect_data = vec![
            1, 0, 0, 0, // image type
            // 11, 0, 0, 0, // constructor function public index
            // 13, 0, 0, 0, // destructor function public index
            17, 0, 0, 0, // import data count
            19, 0, 0, 0, // import function count
            3, 0, 0, 0, // name length
            29, 31, 37, // name buffer
        ];

        expect_data.resize(std::mem::size_of::<CommonPropertySection>(), 0);

        assert_eq!(section_data, expect_data);
    }

    #[test]
    fn test_load_section() {
        let mut section_data = vec![
            1, 0, 0, 0, // image type
            // 11, 0, 0, 0, // constructor function public index
            // 13, 0, 0, 0, // destructor function public index
            17, 0, 0, 0, // import data count
            19, 0, 0, 0, // import function count
            3, 0, 0, 0, // name length
            29, 31, 37, 0, // name buffer
        ];

        section_data.resize(std::mem::size_of::<CommonPropertySection>(), 0);

        let section = CommonPropertySection::load(&section_data);
        assert_eq!(section.image_type, ImageType::SharedModule);
        // assert_eq!(section.constructor_function_public_index, 11);
        // assert_eq!(section.destructor_function_public_index, 13);
        assert_eq!(section.import_data_count, 17);
        assert_eq!(section.import_function_count, 19);
        assert_eq!(section.module_name_length, 3);

        let mut module_name_buffer = [0u8; 256];
        module_name_buffer[0] = 29;
        module_name_buffer[1] = 31;
        module_name_buffer[2] = 37;

        assert_eq!(section.module_name_buffer, module_name_buffer);
    }
}
