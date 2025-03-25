// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use crate::module_image::{ModuleSectionId, SectionEntry};

pub const MODULE_NAME_BUFFER_LENGTH: usize = 256;

#[repr(C)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct PropertySection {
    pub edition: [u8; 8],

    // note:
    // avoid using the u64 integer, it is because
    // both instructions and image data are 4-byte aligned.
    pub version_patch: u16,
    pub version_minor: u16,
    pub version_major: u16,
    _padding0: [u8; 2], // for 4-byte align

    // the "module name", "import data count" and "import function count" are
    // used for find the public index of function and data in
    // the bridge function call.
    //
    // it's also possible to get these information from the `import*`
    // sections, but they are optional in the runtime.
    pub import_data_count: u32,
    pub import_function_count: u32,

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
    // Note that only [a-zA-Z0-9_] and unicode chars are allowed for the (sub)module(/source file) name.
    pub module_name_length: u32,

    pub module_name_buffer: [u8; 256],
}

impl PropertySection {
    pub fn new(
        module_name: &str,
        edition: [u8; 8],
        version_patch: u16,
        version_minor: u16,
        version_major: u16,
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
            edition,
            version_patch,
            version_minor,
            version_major,
            _padding0: [0u8; 2],
            import_data_count,
            import_function_count,
            module_name_length: module_name_src.len() as u32,
            module_name_buffer: module_name_dest,
        }
    }

    pub fn get_module_name(&self) -> &str {
        std::str::from_utf8(&self.module_name_buffer[..(self.module_name_length as usize)]).unwrap()
    }
}

impl<'a> SectionEntry<'a> for PropertySection {
    fn read(section_data: &'a [u8]) -> Self {
        let property_section_ptr = unsafe {
            std::mem::transmute::<*const u8, *const PropertySection>(section_data.as_ptr())
        };

        unsafe { *property_section_ptr }
    }

    fn write(&'a self, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        let mut section_data = [0u8; std::mem::size_of::<PropertySection>()];
        let src = self as *const PropertySection as *const u8;
        let dst = section_data.as_mut_ptr();
        unsafe { std::ptr::copy(src, dst, section_data.len()) };

        writer.write_all(&section_data)
    }

    fn id(&'a self) -> ModuleSectionId {
        ModuleSectionId::Property
    }
}

#[cfg(test)]
mod tests {
    use anc_isa::RUNTIME_EDITION;

    use crate::module_image::SectionEntry;

    use super::PropertySection;

    #[test]
    fn test_write_section() {
        let section = PropertySection::new("bar", *RUNTIME_EDITION, 7, 11, 13, 17, 19);

        let mut section_data: Vec<u8> = vec![];
        section.write(&mut section_data).unwrap();

        let mut expect_data = vec![];

        expect_data.append(&mut RUNTIME_EDITION.to_vec());
        expect_data.append(&mut vec![
            7, 0, // version patch
            11, 0, // version minor
            13, 0, // version major
            0, 0, // version padding
            //
            17, 0, 0, 0, // import data count
            19, 0, 0, 0, // import function count
            //
            3, 0, 0, 0, // name length
            0x62, 0x61, 0x72, // name buffer
        ]);

        // extend the data
        expect_data.resize(std::mem::size_of::<PropertySection>(), 0);

        assert_eq!(section_data, expect_data);
    }

    #[test]
    fn test_read_section() {
        let mut section_data = vec![];
        section_data.append(&mut RUNTIME_EDITION.to_vec());
        section_data.append(&mut vec![
            7, 0, // version patch
            11, 0, // version minor
            13, 0, // version major
            0, 0, // version padding
            //
            17, 0, 0, 0, // import data count
            19, 0, 0, 0, // import function count
            //
            3, 0, 0, 0, // name length
            0x62, 0x61, 0x72, // name buffer
        ]);

        // extend the data
        section_data.resize(std::mem::size_of::<PropertySection>(), 0);

        let section = PropertySection::read(&section_data);
        assert_eq!(&section.edition, RUNTIME_EDITION);
        assert_eq!(section.version_patch, 7);
        assert_eq!(section.version_minor, 11);
        assert_eq!(section.version_major, 13);
        assert_eq!(section.import_data_count, 17);
        assert_eq!(section.import_function_count, 19);
        assert_eq!(section.module_name_length, 3);

        assert_eq!(section.get_module_name(), "bar");
    }
}
