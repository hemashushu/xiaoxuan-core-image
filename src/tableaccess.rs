// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use std::ptr::slice_from_raw_parts;

use crate::module_image::DATA_ALIGN_BYTES;

/// load a section that contains two tables.
///
/// ```text
/// |----------------------------------------------|
/// | table 0 item count (u32) | padding (4 bytes) |
/// |----------------------------------------------|
/// | table 0 record 0                             | <-- record length must be a multiple of 0x4
/// | table 0 record 1                             |
/// | ...                                          |
/// |----------------------------------------------|
/// | table 1 record 0                             | <-- record length must be a multiple of 0x4
/// | table 1 record 1                             |
/// |----------------------------------------------|
/// ```
///
/// note that the items count of table 1 is calculated by:
/// (table 1 data length) / (one record length)
pub fn load_section_with_two_tables<T0, T1>(section_data: &[u8]) -> (&[T0], &[T1]) {
    let ptr = section_data.as_ptr();
    let item_count0 = unsafe { std::ptr::read(ptr as *const u32) } as usize;

    // there is a "safe" approach to read a number from pointer, e.g.
    //
    // ```rust
    //     let mut buf = [0u8; 4];
    //     let data = &binary[0..4];
    //     buf.clone_from_slice(data);
    //     let module_count =  u32::from_le_bytes(buf);
    // ```

    let one_record_length_in_bytes0 = size_of::<T0>();
    let total_length_in_bytes0 = one_record_length_in_bytes0 * item_count0;

    // 8 bytes is the length of header, i.e.
    // 4 bytes `item_count` + 4 bytes padding.
    let items0_data = &section_data[8..(8 + total_length_in_bytes0)];
    let items1_data = &section_data[(8 + total_length_in_bytes0)..];

    // there is another method to get the `items_data`, e.g.
    // ```rust
    //     let ptr_items = unsafe {
    //         ptr.offset(8)
    //     } as *const DataIndexOffset;
    // ```

    let one_record_length_in_bytes1 = size_of::<T1>();
    let item_count1 = items1_data.len() / one_record_length_in_bytes1;
    let items0 = load_items::<T0>(items0_data, item_count0);
    let items1 = load_items::<T1>(items1_data, item_count1);

    (items0, items1)
}

/// save a section that contains two tables.
///
/// ```text
/// |----------------------------------------------|
/// | table 0 item count (u32) | padding (4 bytes) |
/// |----------------------------------------------|
/// | table 0 record 0                             | <-- record length must be a multiple of 0x4
/// | table 0 record 1                             |
/// | ...                                          |
/// |----------------------------------------------|
/// | table 1 record 0                             | <-- record length must be a multiple of 0x4
/// | table 1 record 1                             |
/// |----------------------------------------------|
/// ```
pub fn save_section_with_two_tables<T0, T1>(
    items0: &[T0],
    items1: &[T1],
    writer: &mut dyn std::io::Write,
) -> std::io::Result<()> {
    // write header
    let item_count0 = items0.len();
    writer.write_all(&(item_count0 as u32).to_le_bytes())?; // item count
    writer.write_all(&[0u8; 4])?; // 4 bytes padding

    save_items(items0, writer)?;
    save_items(items1, writer)?;
    // save_offsets(offsets, writer)?;
    // save_index_items(items, writer)?;

    Ok(())
}

/// load a section that contains a table and a variable-length data area.
///
/// ```text
/// |--------------------------------------|
/// | item count (u32) | padding (4 bytes) |
/// |--------------------------------------|
/// | record 0                             | <-- record length must be a multiple of 0x4
/// | record 1                             |
/// | ...                                  |
/// |--------------------------------------|
/// | variable length data area            | <-- data length must be a multiple of 0x4
/// | ...                                  |
/// |--------------------------------------|
/// ```
pub fn load_section_with_table_and_data_area<T>(section_data: &[u8]) -> (&[T], &[u8]) {
    let ptr = section_data.as_ptr();
    let item_count = unsafe { std::ptr::read(ptr as *const u32) } as usize;

    let one_record_length_in_bytes = size_of::<T>();
    let total_length_in_bytes = one_record_length_in_bytes * item_count as usize;

    // 8 bytes is the length of header,
    // 4 bytes `item_count` + 4 bytes padding.
    let items_data = &section_data[8..(8 + total_length_in_bytes)];
    let additional_data = &section_data[(8 + total_length_in_bytes)..];

    let items = load_items::<T>(items_data, item_count);

    (items, additional_data)
}

/// save a section that contains a table and a variable-length data area.
///
/// ```text
/// |--------------------------------------|
/// | item count (u32) | padding (4 bytes) |
/// |--------------------------------------|
/// | record 0                             | <-- record length must be a multiple of 0x4
/// | record 1                             |
/// | ...                                  |
/// |--------------------------------------|
/// | variable length data area            | <-- data length must be a multiple of 0x4
/// | ...                                  |     if the length is not 4x, byte '\0' will
/// |--------------------------------------|     be appended automatically by this function.
/// ```
pub fn save_section_with_table_and_data_area<T>(
    items: &[T],
    additional_data: &[u8],
    writer: &mut dyn std::io::Write,
) -> std::io::Result<()> {
    // write header
    let item_count = items.len();
    writer.write_all(&(item_count as u32).to_le_bytes())?; // item count
    writer.write_all(&[0u8; 4])?; // 4 bytes padding

    save_items::<T>(items, writer)?;
    writer.write_all(additional_data)?;

    let remainder = additional_data.len() % DATA_ALIGN_BYTES; // remainder

    if remainder != 0 {
        let padding = DATA_ALIGN_BYTES - remainder;
        for _count in 0..padding {
            // writer.write(b"\0")?;
            writer.write_all(b"\0")?;
        }
    }

    Ok(())
}

/// load a section that contains only one table.
///
/// ```text
/// |--------------------------------------|
/// | item count (u32) | padding (4 bytes) |
/// |--------------------------------------|
/// | record 0                             | <-- record length must be a multiple of 0x4
/// | record 1                             |
/// | ...                                  |
/// |--------------------------------------|
/// ```
pub fn load_section_with_one_table<T>(section_data: &[u8]) -> &[T] {
    let ptr = section_data.as_ptr();
    let item_count = unsafe { std::ptr::read(ptr as *const u32) } as usize;

    let one_record_length_in_bytes = size_of::<T>();
    let total_length_in_bytes = one_record_length_in_bytes * item_count as usize;

    // 8 bytes is the length of header,
    // 4 bytes `item_count` + 4 bytes padding.
    let items_data = &section_data[8..(8 + total_length_in_bytes)];
    let items = load_items::<T>(items_data, item_count);

    items
}

/// save a section that contains only one table.
///
/// ```text
/// |--------------------------------------|
/// | item count (u32) | padding (4 bytes) |
/// |--------------------------------------|
/// | record 0                             | <-- record length must be a multiple of 0x4
/// | record 1                             |
/// | ...                                  |
/// |--------------------------------------|
/// ```
pub fn save_section_with_one_table<T>(
    items: &[T],
    writer: &mut dyn std::io::Write,
) -> std::io::Result<()> {
    // write header
    let item_count = items.len();
    writer.write_all(&(item_count as u32).to_le_bytes())?; // item count
    writer.write_all(&[0u8; 4])?; // 4 bytes padding

    save_items::<T>(items, writer)?;
    Ok(())
}

/// load a table
/// note that record length must be a multiple of 0x4
pub fn load_items<T>(items_data: &[u8], item_count: usize) -> &[T] {
    let items_ptr = items_data.as_ptr() as *const T;
    // https://doc.rust-lang.org/std/ptr/fn.slice_from_raw_parts.html
    let items_slice = std::ptr::slice_from_raw_parts(items_ptr, item_count);
    unsafe { &*items_slice }
}

/// save a table
/// note that record length must be a multiple of 0x4
pub fn save_items<T>(items: &[T], writer: &mut dyn std::io::Write) -> std::io::Result<()> {
    // let item_count = items.len();
    // let one_record_length_in_bytes = size_of::<T>();
    // let total_length_in_bytes = one_record_length_in_bytes * item_count;

    let total_length_in_bytes = std::mem::size_of_val(items);

    let ptr = items.as_ptr() as *const u8;
    let slice = slice_from_raw_parts(ptr, total_length_in_bytes);
    writer.write_all(unsafe { &*slice })?;

    // an example of writing a slice to Vec<u8>
    //
    // ```rust
    //     let one_record_length_in_bytes = size_of::<T>();
    //     let total_length_in_bytes = one_record_length_in_bytes * item_count;
    //
    //     let mut buf: Vec<u8> = Vec::with_capacity(total_length_in_bytes);
    //     let dst = buf.as_mut_ptr() as *mut u8;
    //     let src = items.as_ptr() as *const u8;
    //
    //     unsafe {
    //         std::ptr::copy(src, dst, total_length_in_bytes);
    //         buf.set_len(total_length_in_bytes);
    //     }
    // ```

    Ok(())
}
