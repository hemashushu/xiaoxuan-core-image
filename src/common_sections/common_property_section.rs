// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use crate::module_image::{ModuleSectionId, SectionEntry};

pub const MODULE_NAME_BUFFER_LENGTH: usize = 256;

#[repr(C)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct CommonPropertySection {
    // pub image_type: ImageType,

    // the "module name", "import data count" and "import function count" are
    // used for find the public index of function and data in
    // the bridge function call.
    //
    // it's also possible to get these information from the `import*`
    // sections, but they are optional in the runtime.
    pub import_data_count: u32,
    pub import_function_count: u32,

    // Note that this is the name of module/package,
    // it CANNOT be the name of submodule even if the current image is
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
    pub module_name_length: u32,
    pub module_name_buffer: [u8; 256],
}

impl CommonPropertySection {
    pub fn new(
        module_name: &str,
        // image_type: ImageType,
        import_data_count: u32,
        import_function_count: u32,
    ) -> Self {
        let module_name_src = module_name.as_bytes();
        let mut module_name_dest = [0u8; MODULE_NAME_BUFFER_LENGTH];

        unsafe {
            std::ptr::copy(
                module_name_src.as_ptr(),
                module_name_dest.as_mut_ptr(),
                module_name_src.len(),
            )
        };

        Self {
            // image_type,
            import_data_count,
            import_function_count,
            module_name_length: module_name_src.len() as u32,
            module_name_buffer: module_name_dest,
        }
    }

    pub fn get_module_name(&self) -> &str {
        unsafe {
            std::str::from_utf8_unchecked(
                &self.module_name_buffer[..(self.module_name_length as usize)],
            )
        }
    }
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
    use crate::module_image::SectionEntry;

    use super::CommonPropertySection;

    #[test]
    fn test_save_section() {
        let section = CommonPropertySection::new("bar", 17, 19);

        let mut section_data: Vec<u8> = Vec::new();
        section.save(&mut section_data).unwrap();

        let mut expect_data = vec![
            // 1, 0, 0, 0, // image type
            17, 0, 0, 0, // import data count
            19, 0, 0, 0, // import function count
            3, 0, 0, 0, // name length
            0x62, 0x61, 0x72, // name buffer
        ];

        expect_data.resize(std::mem::size_of::<CommonPropertySection>(), 0);

        assert_eq!(section_data, expect_data);
    }

    #[test]
    fn test_load_section() {
        let mut section_data = vec![
            // 1, 0, 0, 0, // image type
            17, 0, 0, 0, // import data count
            19, 0, 0, 0, // import function count
            3, 0, 0, 0, // name length
            0x62, 0x61, 0x72, // name buffer, "bar"
        ];

        section_data.resize(std::mem::size_of::<CommonPropertySection>(), 0);

        let section = CommonPropertySection::load(&section_data);
        // assert_eq!(section.image_type, ImageType::SharedModule);
        assert_eq!(section.import_data_count, 17);
        assert_eq!(section.import_function_count, 19);
        assert_eq!(section.module_name_length, 3);

        assert_eq!(section.get_module_name(), "bar");
    }
}
