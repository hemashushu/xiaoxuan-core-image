// Copyright (c) 2025 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use std::ptr::slice_from_raw_parts;

use crate::module_image::{BASE_SECTION_HEADER_LENGTH, TABLE_RECORD_ALIGN_BYTES};

/// Reads a section containing two tables.
///
/// ```text
/// |-------------------------------------------------------|
/// | table 0 item count (u32) | extra header len (4 bytes) |
/// |-------------------------------------------------------|
/// | table 0 record 0                                      | <-- record length must be a multiple of 4 bytes
/// | table 0 record 1                                      |
/// | ...                                                   |
/// |-------------------------------------------------------|
/// | table 1 record 0                                      | <-- record length must be a multiple of 4 bytes
/// | table 1 record 1                                      |
/// |-------------------------------------------------------|
/// ```
///
/// Note:
/// - The item count of table 1 is calculated as `(table 1 data length) / (one record length)`.
pub fn read_section_with_two_tables<T0, T1>(section_data: &[u8]) -> (&[T0], &[T1]) {
    let ptr = section_data.as_ptr();
    let item_count0 = unsafe { std::ptr::read(ptr as *const u32) } as usize;

    // Alternative safe approach to read a number from a pointer:
    // ```rust
    // let mut buf = [0u8; 4];
    // buf.clone_from_slice(&section_data[0..4]);
    // let item_count0 = u32::from_le_bytes(buf) as usize;
    // ```

    let one_record_length_in_bytes0 = size_of::<T0>();
    let total_length_in_bytes0 = one_record_length_in_bytes0 * item_count0;

    // The base section header length is 8 bytes:
    // - 4 bytes for `item_count`
    // - 4 bytes for "extra header length".
    let items0_data = &section_data
        [BASE_SECTION_HEADER_LENGTH..(BASE_SECTION_HEADER_LENGTH + total_length_in_bytes0)];
    let items1_data = &section_data[(BASE_SECTION_HEADER_LENGTH + total_length_in_bytes0)..];

    let one_record_length_in_bytes1 = size_of::<T1>();
    let item_count1 = items1_data.len() / one_record_length_in_bytes1;

    let items0 = read_items::<T0>(items0_data, item_count0);
    let items1 = read_items::<T1>(items1_data, item_count1);

    (items0, items1)
}

/// Writes a section containing two tables.
///
/// ```text
/// |-------------------------------------------------------|
/// | table 0 item count (u32) | extra header len (4 bytes) |
/// |-------------------------------------------------------|
/// | table 0 record 0                                      | <-- record length must be a multiple of 4 bytes
/// | table 0 record 1                                      |
/// | ...                                                   |
/// |-------------------------------------------------------|
/// | table 1 record 0                                      | <-- record length must be a multiple of 4 bytes
/// | table 1 record 1                                      |
/// |-------------------------------------------------------|
/// ```
pub fn write_section_with_two_tables<T0, T1>(
    items0: &[T0],
    items1: &[T1],
    writer: &mut dyn std::io::Write,
) -> std::io::Result<()> {
    // Write header
    let item_count0 = items0.len();
    writer.write_all(&(item_count0 as u32).to_le_bytes())?; // Item count
    writer.write_all(&[0u8; 4])?; // 4 bytes for extra header length

    write_items(items0, writer)?;
    write_items(items1, writer)?;
    Ok(())
}

/// Reads a section containing a table and a variable-length data area.
///
/// ```text
/// |-----------------------------------------------|
/// | item count (u32) | extra header len (4 bytes) |
/// |-----------------------------------------------|
/// | record 0                                      | <-- record length must be a multiple of 4 bytes
/// | record 1                                      |
/// | ...                                           |
/// |-----------------------------------------------|
/// | variable-length data area                     | <-- data length must be a multiple of 4 bytes
/// | ...                                           |
/// |-----------------------------------------------|
/// ```
pub fn read_section_with_table_and_data_area<T>(section_data: &[u8]) -> (&[T], &[u8]) {
    let ptr = section_data.as_ptr();
    let item_count = unsafe { std::ptr::read(ptr as *const u32) } as usize;

    let one_record_length_in_bytes = size_of::<T>();
    let total_length_in_bytes = one_record_length_in_bytes * item_count;

    // The base section header length is 8 bytes:
    // - 4 bytes for `item_count`
    // - 4 bytes for "extra header length".
    let items_data = &section_data
        [BASE_SECTION_HEADER_LENGTH..(BASE_SECTION_HEADER_LENGTH + total_length_in_bytes)];
    let additional_data = &section_data[(BASE_SECTION_HEADER_LENGTH + total_length_in_bytes)..];

    let items = read_items::<T>(items_data, item_count);

    (items, additional_data)
}

/// Writes a section containing a table and a variable-length data area.
///
/// ```text
/// |-----------------------------------------------|
/// | item count (u32) | extra header len (4 bytes) |
/// |-----------------------------------------------|
/// | record 0                                      | <-- record length must be a multiple of 4 bytes
/// | record 1                                      |
/// | ...                                           |
/// |-----------------------------------------------|
/// | variable-length data area                     | <-- Total length must be a multiple of 4 bytes.
/// | ...                                           |     If not, padding with '\0' bytes is added.
/// |-----------------------------------------------|
/// ```
pub fn write_section_with_table_and_data_area<T>(
    items: &[T],
    additional_data: &[u8],
    writer: &mut dyn std::io::Write,
) -> std::io::Result<()> {
    // Write header
    let item_count = items.len();
    writer.write_all(&(item_count as u32).to_le_bytes())?; // Item count
    writer.write_all(&[0u8; 4])?; // 4 bytes for extra header length

    write_items::<T>(items, writer)?;
    writer.write_all(additional_data)?;

    // Pad the data area to make its length a multiple of 4 bytes
    let remainder = additional_data.len() % TABLE_RECORD_ALIGN_BYTES;
    if remainder != 0 {
        let padding = TABLE_RECORD_ALIGN_BYTES - remainder;
        writer.write_all(&vec![0u8; padding])?;
    }

    Ok(())
}

/// Reads a section containing only one table.
///
/// ```text
/// |-----------------------------------------------|
/// | item count (u32) | extra header len (4 bytes) |
/// |-----------------------------------------------|
/// | record 0                                      | <-- record length must be a multiple of 4 bytes
/// | record 1                                      |
/// | ...                                           |
/// |-----------------------------------------------|
/// ```
pub fn read_section_with_one_table<T>(section_data: &[u8]) -> &[T] {
    let ptr = section_data.as_ptr();
    let item_count = unsafe { std::ptr::read(ptr as *const u32) } as usize;

    let one_record_length_in_bytes = size_of::<T>();
    let total_length_in_bytes = one_record_length_in_bytes * item_count;

    // The base section header length is 8 bytes:
    // - 4 bytes for `item_count`
    // - 4 bytes for "extra header length".
    let items_data = &section_data
        [BASE_SECTION_HEADER_LENGTH..(BASE_SECTION_HEADER_LENGTH + total_length_in_bytes)];
    read_items::<T>(items_data, item_count)
}

/// Writes a section containing only one table.
///
/// ```text
/// |-----------------------------------------------|
/// | item count (u32) | extra header len (4 bytes) |
/// |-----------------------------------------------|
/// | record 0                                      | <-- record length must be a multiple of 4 bytes
/// | record 1                                      |
/// | ...                                           |
/// |-----------------------------------------------|
/// ```
pub fn write_section_with_one_table<T>(
    items: &[T],
    writer: &mut dyn std::io::Write,
) -> std::io::Result<()> {
    // Write header
    let item_count = items.len();
    writer.write_all(&(item_count as u32).to_le_bytes())?; // Item count
    writer.write_all(&[0u8; 4])?; // 4 bytes for extra header length

    write_items::<T>(items, writer)?;
    Ok(())
}

/// Reads a table from the given data.
///
/// Note: The record length must be a multiple of 4 bytes.
pub fn read_items<T>(items_data: &[u8], item_count: usize) -> &[T] {
    let items_ptr = items_data.as_ptr() as *const T;
    let items_slice = std::ptr::slice_from_raw_parts(items_ptr, item_count);
    unsafe { &*items_slice }
}

/// Writes a table to the given writer.
///
/// Note: The record length must be a multiple of 4 bytes.
pub fn write_items<T>(items: &[T], writer: &mut dyn std::io::Write) -> std::io::Result<()> {
    let total_length_in_bytes = std::mem::size_of_val(items);

    let ptr = items.as_ptr() as *const u8;
    let slice = slice_from_raw_parts(ptr, total_length_in_bytes);
    writer.write_all(unsafe { &*slice })?;

    Ok(())
}
