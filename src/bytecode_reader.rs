// Copyright (c) 2025 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions.
// For more details, see the LICENSE, LICENSE.additional, and CONTRIBUTING files.

use anc_isa::opcode::Opcode;

/// Formats the bytecode as binary with fixed-length hexadecimal representation.
///
/// Example output:
/// ```text
/// 0x0000  00 11 22 33  44 55 66 77
/// 0x0008  88 99 aa bb  cc dd ee ff
/// ```
pub fn format_bytecode_as_binary(codes: &[u8]) -> String {
    codes
        .chunks(8)
        .enumerate()
        .map(|(chunk_addr, chunk)| {
            let binary = chunk
                .iter()
                .enumerate()
                .map(|(idx, byte)| {
                    // Formats bytes as:
                    // 00 11 22 33  44 55 66 77
                    if idx == 4 {
                        format!("  {:02x}", byte)
                    } else if idx == 0 {
                        format!("{:02x}", byte)
                    } else {
                        format!(" {:02x}", byte)
                    }
                })
                .collect::<Vec<String>>()
                .join("");

            format!("0x{:04x}  {}", chunk_addr * 8, binary)
        })
        .collect::<Vec<String>>()
        .join("\n")
}

/// Formats the bytecode as text with instruction hex and corresponding instruction names.
///
/// Example output:
/// ```text
/// 0x0000  00 07                       i32.add
/// 0x0002  00 04 02 00                 heap.load       off:0x02
/// 0x0006  08 04 03 00                 heap.store      off:0x03
/// ```
pub fn format_bytecode_as_text(codes: &[u8]) -> String {
    let mut lines: Vec<String> = vec![];

    let code_length = codes.len(); // Total bytecode length
    let mut offset = 0; // Current offset in the bytecode

    while offset < code_length {
        let (offset_param, opcode) = read_opcode(codes, offset);

        let (offset_next, param_text) = match opcode {
            // Fundamental instructions
            Opcode::nop => (offset_param, String::new()),
            Opcode::terminate => {
                let (offset_next, code) = continue_read_param_i32(codes, offset_param);
                (offset_next, format!("code:{}", code))
            }
            Opcode::imm_i32 | Opcode::imm_f32 => {
                let (offset_next, v) = continue_read_param_i32(codes, offset_param);
                (offset_next, format!("0x{:08x}", v))
            }
            Opcode::imm_i64 | Opcode::imm_f64 => {
                let (offset_next, v_low, v_high) = continue_read_param_i32_i32(codes, offset_param);
                (
                    offset_next,
                    format!("low:0x{:08x}  high:0x{:08x}", v_low, v_high),
                )
            }
            // Local load/store instructions
            Opcode::local_load_i64
            | Opcode::local_load_i32_s
            | Opcode::local_load_i32_u
            | Opcode::local_load_i16_s
            | Opcode::local_load_i16_u
            | Opcode::local_load_i8_s
            | Opcode::local_load_i8_u
            | Opcode::local_load_f64
            | Opcode::local_load_f32
            | Opcode::local_store_i64
            | Opcode::local_store_i32
            | Opcode::local_store_i16
            | Opcode::local_store_i8
            | Opcode::local_store_f64
            | Opcode::local_store_f32 => {
                let (offset_next, reversed_index, offset, index) =
                    continue_read_param_i16_i16_i16(codes, offset_param);
                (
                    offset_next,
                    format!(
                        "rev:{:<2}  off:0x{:02x}  idx:{}",
                        reversed_index, offset, index,
                    ),
                )
            }
            //
            Opcode::local_load_extend_i64
            | Opcode::local_load_extend_i32_s
            | Opcode::local_load_extend_i32_u
            | Opcode::local_load_extend_i16_s
            | Opcode::local_load_extend_i16_u
            | Opcode::local_load_extend_i8_s
            | Opcode::local_load_extend_i8_u
            | Opcode::local_load_extend_f64
            | Opcode::local_load_extend_f32
            | Opcode::local_store_extend_i64
            | Opcode::local_store_extend_i32
            | Opcode::local_store_extend_i16
            | Opcode::local_store_extend_i8
            | Opcode::local_store_extend_f64
            | Opcode::local_store_extend_f32 => {
                let (offset_next, reversed_index, index) =
                    continue_read_param_i16_i32(codes, offset_param);
                (
                    offset_next,
                    format!("rev:{:<2}  idx:{}", reversed_index, index),
                )
            }
            // Data load/store instructions
            Opcode::data_load_i64
            | Opcode::data_load_i32_s
            | Opcode::data_load_i32_u
            | Opcode::data_load_i16_s
            | Opcode::data_load_i16_u
            | Opcode::data_load_i8_s
            | Opcode::data_load_i8_u
            | Opcode::data_load_f64
            | Opcode::data_load_f32
            | Opcode::data_store_i64
            | Opcode::data_store_i32
            | Opcode::data_store_i16
            | Opcode::data_store_i8
            | Opcode::data_store_f64
            | Opcode::data_store_f32 => {
                let (offset_next, offset, index) = continue_read_param_i16_i32(codes, offset_param);
                (offset_next, format!("off:0x{:02x}  idx:{}", offset, index))
            }
            //
            Opcode::data_load_extend_i64
            | Opcode::data_load_extend_i32_s
            | Opcode::data_load_extend_i32_u
            | Opcode::data_load_extend_i16_s
            | Opcode::data_load_extend_i16_u
            | Opcode::data_load_extend_i8_s
            | Opcode::data_load_extend_i8_u
            | Opcode::data_load_extend_f64
            | Opcode::data_load_extend_f32
            | Opcode::data_store_extend_i64
            | Opcode::data_store_extend_i32
            | Opcode::data_store_extend_i16
            | Opcode::data_store_extend_i8
            | Opcode::data_store_extend_f64
            | Opcode::data_store_extend_f32 => {
                let (offset_next, index) = continue_read_param_i32(codes, offset_param);
                (offset_next, format!("idx:{}", index))
            }
            // Data dynamic instructions
            Opcode::data_load_dynamic_i64
            | Opcode::data_load_dynamic_i32_s
            | Opcode::data_load_dynamic_i32_u
            | Opcode::data_load_dynamic_i16_s
            | Opcode::data_load_dynamic_i16_u
            | Opcode::data_load_dynamic_i8_s
            | Opcode::data_load_dynamic_i8_u
            | Opcode::data_load_dynamic_f64
            | Opcode::data_load_dynamic_f32
            | Opcode::data_store_dynamic_i64
            | Opcode::data_store_dynamic_i32
            | Opcode::data_store_dynamic_i16
            | Opcode::data_store_dynamic_i8
            | Opcode::data_store_dynamic_f64
            | Opcode::data_store_dynamic_f32 => (offset_param, String::new()),
            // Heap memory instructions
            Opcode::memory_allocate
            | Opcode::memory_resize
            | Opcode::memory_free
            | Opcode::memory_fill
            | Opcode::memory_copy => (offset_param, String::new()),

            // Conversion instructions
            Opcode::truncate_i64_to_i32
            | Opcode::extend_i32_s_to_i64
            | Opcode::extend_i32_u_to_i64
            | Opcode::demote_f64_to_f32
            | Opcode::promote_f32_to_f64
            | Opcode::convert_f32_to_i32_s
            | Opcode::convert_f32_to_i32_u
            | Opcode::convert_f64_to_i32_s
            | Opcode::convert_f64_to_i32_u
            | Opcode::convert_f32_to_i64_s
            | Opcode::convert_f32_to_i64_u
            | Opcode::convert_f64_to_i64_s
            | Opcode::convert_f64_to_i64_u
            | Opcode::convert_i32_s_to_f32
            | Opcode::convert_i32_u_to_f32
            | Opcode::convert_i64_s_to_f32
            | Opcode::convert_i64_u_to_f32
            | Opcode::convert_i32_s_to_f64
            | Opcode::convert_i32_u_to_f64
            | Opcode::convert_i64_s_to_f64
            | Opcode::convert_i64_u_to_f64 => (offset_param, String::new()),
            // Comparison instructions
            Opcode::eqz_i32
            | Opcode::nez_i32
            | Opcode::eq_i32
            | Opcode::ne_i32
            | Opcode::lt_i32_s
            | Opcode::lt_i32_u
            | Opcode::gt_i32_s
            | Opcode::gt_i32_u
            | Opcode::le_i32_s
            | Opcode::le_i32_u
            | Opcode::ge_i32_s
            | Opcode::ge_i32_u
            | Opcode::eqz_i64
            | Opcode::nez_i64
            | Opcode::eq_i64
            | Opcode::ne_i64
            | Opcode::lt_i64_s
            | Opcode::lt_i64_u
            | Opcode::gt_i64_s
            | Opcode::gt_i64_u
            | Opcode::le_i64_s
            | Opcode::le_i64_u
            | Opcode::ge_i64_s
            | Opcode::ge_i64_u
            | Opcode::eq_f32
            | Opcode::ne_f32
            | Opcode::lt_f32
            | Opcode::gt_f32
            | Opcode::le_f32
            | Opcode::ge_f32
            | Opcode::eq_f64
            | Opcode::ne_f64
            | Opcode::lt_f64
            | Opcode::gt_f64
            | Opcode::le_f64
            | Opcode::ge_f64 => (offset_param, String::new()),
            // arithmetic
            Opcode::add_i32
            | Opcode::sub_i32
            | Opcode::mul_i32
            | Opcode::div_i32_s
            | Opcode::div_i32_u
            | Opcode::rem_i32_s
            | Opcode::rem_i32_u => (offset_param, String::new()),
            Opcode::add_imm_i32 | Opcode::sub_imm_i32 => {
                let (offset_next, amount) = continue_read_param_i16(codes, offset_param);
                (offset_next, format!("{}", amount))
            }
            Opcode::add_i64
            | Opcode::sub_i64
            | Opcode::mul_i64
            | Opcode::div_i64_s
            | Opcode::div_i64_u
            | Opcode::rem_i64_s
            | Opcode::rem_i64_u => (offset_param, String::new()),
            Opcode::add_imm_i64 | Opcode::sub_imm_i64 => {
                let (offset_next, amount) = continue_read_param_i16(codes, offset_param);
                (offset_next, format!("{}", amount))
            }
            Opcode::add_f32
            | Opcode::sub_f32
            | Opcode::mul_f32
            | Opcode::div_f32
            | Opcode::add_f64
            | Opcode::sub_f64
            | Opcode::mul_f64
            | Opcode::div_f64 => (offset_param, String::new()),
            // bitwise
            Opcode::and
            | Opcode::or
            | Opcode::xor
            | Opcode::not
            | Opcode::count_leading_zeros_i32
            | Opcode::count_leading_ones_i32
            | Opcode::count_trailing_zeros_i32
            | Opcode::count_ones_i32
            | Opcode::shift_left_i32
            | Opcode::shift_right_i32_s
            | Opcode::shift_right_i32_u
            | Opcode::rotate_left_i32
            | Opcode::rotate_right_i32
            | Opcode::count_leading_zeros_i64
            | Opcode::count_leading_ones_i64
            | Opcode::count_trailing_zeros_i64
            | Opcode::count_ones_i64
            | Opcode::shift_left_i64
            | Opcode::shift_right_i64_s
            | Opcode::shift_right_i64_u
            | Opcode::rotate_left_i64
            | Opcode::rotate_right_i64 => (offset_param, String::new()),
            // math
            Opcode::abs_i32
            | Opcode::neg_i32
            | Opcode::abs_i64
            | Opcode::neg_i64
            | Opcode::abs_f32
            | Opcode::neg_f32
            | Opcode::ceil_f32
            | Opcode::floor_f32
            | Opcode::round_half_away_from_zero_f32
            | Opcode::round_half_to_even_f32
            | Opcode::trunc_f32
            | Opcode::fract_f32
            | Opcode::sqrt_f32
            | Opcode::cbrt_f32
            | Opcode::exp_f32
            | Opcode::exp2_f32
            | Opcode::ln_f32
            | Opcode::log2_f32
            | Opcode::log10_f32
            | Opcode::sin_f32
            | Opcode::cos_f32
            | Opcode::tan_f32
            | Opcode::asin_f32
            | Opcode::acos_f32
            | Opcode::atan_f32
            | Opcode::copysign_f32
            | Opcode::pow_f32
            | Opcode::log_f32
            | Opcode::min_f32
            | Opcode::max_f32
            | Opcode::abs_f64
            | Opcode::neg_f64
            | Opcode::ceil_f64
            | Opcode::floor_f64
            | Opcode::round_half_away_from_zero_f64
            | Opcode::round_half_to_even_f64
            | Opcode::trunc_f64
            | Opcode::fract_f64
            | Opcode::sqrt_f64
            | Opcode::cbrt_f64
            | Opcode::exp_f64
            | Opcode::exp2_f64
            | Opcode::ln_f64
            | Opcode::log2_f64
            | Opcode::log10_f64
            | Opcode::sin_f64
            | Opcode::cos_f64
            | Opcode::tan_f64
            | Opcode::asin_f64
            | Opcode::acos_f64
            | Opcode::atan_f64
            | Opcode::copysign_f64
            | Opcode::pow_f64
            | Opcode::log_f64
            | Opcode::min_f64
            | Opcode::max_f64 => (offset_param, String::new()),
            // control flow
            Opcode::end => (offset_param, String::new()),
            Opcode::block => {
                let (offset_next, type_idx, local_variable_list_index) =
                    continue_read_param_i32_i32(codes, offset_param);
                (
                    offset_next,
                    format!("type:{:<2}  local:{}", type_idx, local_variable_list_index),
                )
            }
            Opcode::block_alt => {
                let (offset_next, type_idx, local_variable_list_index, offset) =
                    continue_read_param_i32_i32_i32(codes, offset_param);
                (
                    offset_next,
                    format!(
                        "type:{:<2}  local:{:<2}  off:0x{:02x}",
                        type_idx, local_variable_list_index, offset
                    ),
                )
            }
            Opcode::break_alt => {
                let (offset_next, offset) = continue_read_param_i32(codes, offset_param);
                (offset_next, format!("off:0x{:02x}", offset))
            }
            Opcode::block_nez => {
                let (offset_next, local_variable_list_index, offset) =
                    continue_read_param_i32_i32(codes, offset_param);
                (
                    offset_next,
                    format!(
                        "local:{:<2}  off:0x{:02x}",
                        local_variable_list_index, offset
                    ),
                )
            }
            Opcode::break_ | Opcode::recur => {
                let (offset_next, reversed_index, offset) =
                    continue_read_param_i16_i32(codes, offset_param);
                (
                    offset_next,
                    format!("rev:{:<2}  off:0x{:02x}", reversed_index, offset),
                )
            }
            Opcode::call | Opcode::extcall | Opcode::envcall => {
                let (offset_next, idx) = continue_read_param_i32(codes, offset_param);
                (offset_next, format!("idx:{}", idx))
            }
            Opcode::call_dynamic | Opcode::syscall => (offset_param, String::new()),
            // machine
            Opcode::get_function | Opcode::get_data => {
                let (offset_next, idx) = continue_read_param_i32(codes, offset_param);
                (offset_next, format!("idx:{}", idx))
            }
            Opcode::host_addr_local => {
                let (offset_next, reversed_idx, offset, idx) =
                    continue_read_param_i16_i16_i16(codes, offset_param);
                (
                    offset_next,
                    format!("rev:{:<2}  off:0x{:02x}  idx:{}", reversed_idx, offset, idx,),
                )
            }
            Opcode::host_addr_local_extend => {
                let (offset_next, reversed_idx, idx) =
                    continue_read_param_i16_i32(codes, offset_param);
                (offset_next, format!("rev:{:<2}  idx:{}", reversed_idx, idx))
            }
            Opcode::host_addr_data => {
                let (offset_next, offset, idx) = continue_read_param_i16_i32(codes, offset_param);
                (offset_next, format!("off:0x{:02x}  idx:{}", offset, idx))
            }
            Opcode::host_addr_data_extend => {
                let (offset_next, idx) = continue_read_param_i32(codes, offset_param);
                (offset_next, format!("idx:{}", idx))
            }
            Opcode::host_addr_data_dynamic => (offset_param, String::new()),
            Opcode::host_addr_function => {
                let (offset_next, idx) = continue_read_param_i32(codes, offset_param);
                (offset_next, format!("idx:{}", idx))
            }
            Opcode::host_copy_from_data
            | Opcode::host_copy_to_data
            | Opcode::host_external_memory_copy => (offset_param, String::new()),
        };

        // format!(...)
        // https://doc.rust-lang.org/std/fmt/

        let mut line = format!("0x{:04x}  ", offset);
        let addr_width = line.len();

        let inst_data = &codes[offset..offset_next];
        let mut chunks = inst_data.chunks(8);

        // format the bytes as the following text:
        //
        // 0x0006  08 04 03 00
        // 0x000a  00 02 05 00  07 00 11 00
        let print_binary = |data: &[u8]| {
            data.iter()
                .enumerate()
                .map(|(idx, byte)| {
                    if idx == 4 {
                        format!("  {:02x}", byte)
                    } else if idx == 0 {
                        format!("{:02x}", byte)
                    } else {
                        format!(" {:02x}", byte)
                    }
                })
                .collect::<Vec<String>>()
                .join("")
        };

        if param_text.is_empty() {
            line.push_str(&format!(
                "{:28}{}",
                print_binary(chunks.next().unwrap()),
                opcode.get_name()
            ));
        } else {
            line.push_str(&format!(
                "{:28}{:16}  {}",
                print_binary(chunks.next().unwrap()),
                opcode.get_name(),
                param_text
            ));
        }

        lines.push(line);

        let indent_text = " ".repeat(addr_width);
        for chunk in chunks {
            lines.push(format!("{}{}", indent_text, print_binary(chunk)));
        }

        // move on
        offset = offset_next;
    }

    lines.join("\n")
}

// opcode, or
// 16 bits instruction
// [opcode]
fn read_opcode(codes: &[u8], offset: usize) -> (usize, Opcode) {
    let opcode_data = &codes[offset..offset + 2];
    let opcode_u16 = u16::from_le_bytes(opcode_data.try_into().unwrap());

    (offset + 2, unsafe {
        std::mem::transmute::<u16, Opcode>(opcode_u16)
    })
}

// 32 bits instruction parameters
// [opcode + i16]
fn continue_read_param_i16(codes: &[u8], offset: usize) -> (usize, u16) {
    let param_data0 = &codes[offset..offset + 2];
    (
        offset + 2,
        u16::from_le_bytes(param_data0.try_into().unwrap()),
    )
}

// 64 bits instruction parameters
// [opcode + padding + i32]
//
// note that 'i32' in function name means a 32-bit integer, which is equivalent to
// the 'uint32_t' in C or 'u32' in Rust. do not confuse it with 'i32' in Rust.
// the same applies to the i8, i16 and i64.
fn continue_read_param_i32(codes: &[u8], offset: usize) -> (usize, u32) {
    let param_data0 = &codes[offset + 2..offset + 6];

    (
        offset + 6,
        u32::from_le_bytes(param_data0.try_into().unwrap()),
    )
}

// 64 bits instruction parameters
// [opcode + i16 + i32]
fn continue_read_param_i16_i32(codes: &[u8], offset: usize) -> (usize, u16, u32) {
    let param_data0 = &codes[offset..offset + 2];
    let param_data1 = &codes[offset + 2..offset + 6];

    (
        offset + 6,
        u16::from_le_bytes(param_data0.try_into().unwrap()),
        u32::from_le_bytes(param_data1.try_into().unwrap()),
    )
}

// 64 bits instruction parameters
// [opcode + i16 + i16 + i16]
fn continue_read_param_i16_i16_i16(codes: &[u8], offset: usize) -> (usize, u16, u16, u16) {
    let param_data0 = &codes[offset..offset + 2];
    let param_data1 = &codes[offset + 2..offset + 4];
    let param_data2 = &codes[offset + 4..offset + 6];

    (
        offset + 6,
        u16::from_le_bytes(param_data0.try_into().unwrap()),
        u16::from_le_bytes(param_data1.try_into().unwrap()),
        u16::from_le_bytes(param_data2.try_into().unwrap()),
    )
}

// 96 bits instruction parameters
// [opcode + padding + i32 + i32]
fn continue_read_param_i32_i32(codes: &[u8], offset: usize) -> (usize, u32, u32) {
    let param_data0 = &codes[offset + 2..offset + 6];
    let param_data1 = &codes[offset + 6..offset + 10];

    (
        offset + 10,
        u32::from_le_bytes(param_data0.try_into().unwrap()),
        u32::from_le_bytes(param_data1.try_into().unwrap()),
    )
}

// 128 bits instruction parameters
// [opcode + padding + i32 + i32 + i32]
fn continue_read_param_i32_i32_i32(codes: &[u8], offset: usize) -> (usize, u32, u32, u32) {
    let param_data0 = &codes[offset + 2..offset + 6];
    let param_data1 = &codes[offset + 6..offset + 10];
    let param_data2 = &codes[offset + 10..offset + 14];

    (
        offset + 14,
        u32::from_le_bytes(param_data0.try_into().unwrap()),
        u32::from_le_bytes(param_data1.try_into().unwrap()),
        u32::from_le_bytes(param_data2.try_into().unwrap()),
    )
}

#[cfg(test)]
mod tests {
    use anc_isa::opcode::Opcode;
    use pretty_assertions::assert_eq;

    use crate::{
        bytecode_reader::{format_bytecode_as_binary, format_bytecode_as_text},
        bytecode_writer::BytecodeWriterHelper,
    };

    #[test]
    fn test_print_bytecodes_as_binary() {
        let data = BytecodeWriterHelper::new()
            .append_opcode(Opcode::eqz_i32)
            .append_opcode_i32(Opcode::imm_i32, 0x13)
            .append_opcode_i16(Opcode::add_imm_i32, 0x2)
            .append_opcode_i32(Opcode::imm_i32, 0x13)
            //
            .append_opcode(Opcode::eqz_i32)
            .append_opcode_i16_i32(Opcode::data_load_i64, 0x17, 0x19)
            .append_opcode_i16(Opcode::add_imm_i32, 0x2)
            .append_opcode_i16_i32(Opcode::data_load_i64, 0x17, 0x19)
            //
            .append_opcode(Opcode::eqz_i32)
            .append_opcode_i32_i32(Opcode::block, 0x23, 0x29)
            .append_opcode_i16(Opcode::add_imm_i32, 0x2)
            .append_opcode_i32_i32(Opcode::block, 0x23, 0x29)
            //
            .append_opcode(Opcode::eqz_i32)
            .append_opcode_i32_i32_i32(Opcode::block_alt, 0x31, 0x37, 0x41)
            .append_opcode_i16(Opcode::add_imm_i32, 0x2)
            .append_opcode_i32_i32_i32(Opcode::block_alt, 0x31, 0x37, 0x41)
            .to_bytes();

        let text = format_bytecode_as_binary(&data);

        assert_eq!(
            text,
            "\
0x0000  c0 02 00 01  40 01 00 00
0x0008  13 00 00 00  02 03 02 00
0x0010  40 01 00 00  13 00 00 00
0x0018  c0 02 00 01  c0 01 17 00
0x0020  19 00 00 00  02 03 02 00
0x0028  c0 01 17 00  19 00 00 00
0x0030  c0 02 00 01  c1 03 00 00
0x0038  23 00 00 00  29 00 00 00
0x0040  02 03 02 00  c1 03 00 00
0x0048  23 00 00 00  29 00 00 00
0x0050  c0 02 00 01  c4 03 00 00
0x0058  31 00 00 00  37 00 00 00
0x0060  41 00 00 00  02 03 02 00
0x0068  c4 03 00 00  31 00 00 00
0x0070  37 00 00 00  41 00 00 00"
        );
    }

    #[test]
    fn test_print_bytecodes_as_text() {
        let data = BytecodeWriterHelper::new()
            .append_opcode(Opcode::eqz_i32)
            .append_opcode_i32(Opcode::imm_i32, 0x13)
            .append_opcode_i16(Opcode::add_imm_i32, 0x2)
            .append_opcode_i32(Opcode::imm_i32, 0x13)
            //
            .append_opcode(Opcode::eqz_i32)
            .append_opcode_i16_i32(Opcode::data_load_i64, 0x17, 0x19)
            .append_opcode_i16(Opcode::add_imm_i32, 0x2)
            .append_opcode_i16_i32(Opcode::data_load_i64, 0x17, 0x19)
            //
            .append_opcode(Opcode::eqz_i32)
            .append_opcode_i32_i32(Opcode::block, 0x23, 0x29)
            .append_opcode_i16(Opcode::add_imm_i32, 0x2)
            .append_opcode_i32_i32(Opcode::block, 0x23, 0x29)
            //
            .append_opcode(Opcode::eqz_i32)
            .append_opcode_i32_i32_i32(Opcode::block_alt, 0x31, 0x37, 0x41)
            .append_opcode_i16(Opcode::add_imm_i32, 0x2)
            .append_opcode_i32_i32_i32(Opcode::block_alt, 0x31, 0x37, 0x41)
            .to_bytes();

        let text = format_bytecode_as_text(&data);

        assert_eq!(
            text,
            "\
0x0000  c0 02                       eqz_i32
0x0002  00 01                       nop
0x0004  40 01 00 00  13 00 00 00    imm_i32           0x00000013
0x000c  02 03 02 00                 add_imm_i32       2
0x0010  40 01 00 00  13 00 00 00    imm_i32           0x00000013
0x0018  c0 02                       eqz_i32
0x001a  00 01                       nop
0x001c  c0 01 17 00  19 00 00 00    data_load_i64     off:0x17  idx:25
0x0024  02 03 02 00                 add_imm_i32       2
0x0028  c0 01 17 00  19 00 00 00    data_load_i64     off:0x17  idx:25
0x0030  c0 02                       eqz_i32
0x0032  00 01                       nop
0x0034  c1 03 00 00  23 00 00 00    block             type:35  local:41
        29 00 00 00
0x0040  02 03 02 00                 add_imm_i32       2
0x0044  c1 03 00 00  23 00 00 00    block             type:35  local:41
        29 00 00 00
0x0050  c0 02                       eqz_i32
0x0052  00 01                       nop
0x0054  c4 03 00 00  31 00 00 00    block_alt         type:49  local:55  off:0x41
        37 00 00 00  41 00 00 00
0x0064  02 03 02 00                 add_imm_i32       2
0x0068  c4 03 00 00  31 00 00 00    block_alt         type:49  local:55  off:0x41
        37 00 00 00  41 00 00 00"
        )
    }
}
