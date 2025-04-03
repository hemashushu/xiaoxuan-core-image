// Copyright (c) 2025 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions.
// For more details, see the LICENSE, LICENSE.additional, and CONTRIBUTING files.

use std::io::Write;

use anc_isa::opcode::Opcode;

pub struct BytecodeWriter {
    buffer: Vec<u8>, // Implements the trait std::io::Write
}

// About the padding
// -----------------
// Instructions containing 'i32' parameters will insert padding automatically
// for 4-byte alignment.
//
// Summary:
//
// Without padding:
// - write_opcode
// - write_opcode_i16
// - write_opcode_i16_i16_i16
//
// With alignment check:
// - write_opcode_i32
// - write_opcode_i16_i32
// - write_opcode_i32_i32
// - write_opcode_i32_i32_i32
// - write_opcode_i64
// - write_opcode_f32
// - write_opcode_f64

// About the stubs
// ---------------
//
// The following instructions include the "next_inst_offset" parameter:
//
// - block_alt (param type_index: i32, next_inst_offset: i32)
// - block_nez (param local_variable_list_index: i32, next_inst_offset: i32)
// - break (param reversed_index: i16, next_inst_offset: i32)
// - break_alt (param next_inst_offset: i32)
//
// When generating bytecode for these instructions, the value of the
// "next_inst_offset" parameter is initially UNKNOWN and can only be determined
// when the "end" instruction is emitted.
//
// To handle this, the assembler first writes a placeholder value of `0`
// (referred to as a "stub") for the "next_inst_offset" parameter and records
// the positions of these instructions. Later, when the "end" instruction is
// generated, the placeholder `0` is replaced with the actual value.
//
// The "ControlFlowStack" structure is designed to facilitate this process.
//
// Notes:
//
// 1. The "recur" instruction does not require stubs because the value of the
//    "start_inst_offset" parameter can be determined immediately using the
//    "ControlFlowStack" structure.
//
// 2. If the target layer of a "break" instruction is "function", no stub is
//    needed, and the "ControlFlowStack" is unnecessary because the VM ignores
//    the "next_inst_offset" in this case.
//
// 3. Similarly, if the target layer of a "recur" instruction is "function",
//    no stub is needed, and the "ControlFlowStack" is unnecessary because the
//    VM ignores the "start_inst_offset" in this case.

/// Note: The word 'i32' in the function names below refers to a 32-bit integer,
/// equivalent to 'uint32_t' in C or 'u32' in Rust. Do not confuse it with Rust's 'i32',
/// which represents a signed 32-bit integer. The same applies to 'i8', 'i16', and 'i64'.
impl BytecodeWriter {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            buffer: Vec::<u8>::new(),
        }
    }

    // Writes a 16-bit unsigned integer to the buffer in little-endian format.
    fn put_i16(&mut self, value: u16) {
        let data = value.to_le_bytes();
        self.buffer.write_all(&data).unwrap();
    }

    // Writes a 32-bit unsigned integer to the buffer in little-endian format.
    fn put_i32(&mut self, value: u32) {
        let data = value.to_le_bytes();
        self.buffer.write_all(&data).unwrap();
    }

    // Writes a 64-bit unsigned integer to the buffer in little-endian format.
    fn put_i64(&mut self, value: u64) {
        let data = value.to_le_bytes();
        self.buffer.write_all(&data).unwrap();
    }

    // Writes a 32-bit floating-point number to the buffer in little-endian format.
    fn put_f32(&mut self, value: f32) {
        let data = value.to_le_bytes();
        self.buffer.write_all(&data).unwrap();
    }

    // Writes a 64-bit floating-point number to the buffer in little-endian format.
    fn put_f64(&mut self, value: f64) {
        let data = value.to_le_bytes();
        self.buffer.write_all(&data).unwrap();
    }

    // Writes an opcode to the buffer and returns the address of the instruction.
    fn put_opcode(&mut self, opcode: Opcode) -> usize {
        let addr = self.get_addr();
        self.put_i16(opcode as u16);
        addr
    }

    // Writes an opcode with padding to ensure alignment and returns the address.
    fn put_opcode_with_padding(&mut self, opcode: Opcode) -> usize {
        let addr = self.put_opcode(opcode);
        self.put_i16(0); // Adds padding
        addr
    }

    // Inserts a padding instruction ('nop') if the current buffer position is not 4-byte aligned.
    //
    // Note: Only instructions with 'i32' parameters require this alignment.
    fn insert_padding_if_necessary(&mut self) -> usize {
        let addr_of_next_inst = self.get_addr();

        if self.buffer.len() % 4 != 0 {
            self.put_i16(Opcode::nop as u16); // Inserts 'nop' for alignment
            addr_of_next_inst + 2
        } else {
            addr_of_next_inst
        }
    }

    /// Writes a 16-bit instruction and returns its address.
    pub fn write_opcode(&mut self, opcode: Opcode) -> usize {
        self.put_opcode(opcode)
    }

    /// Writes a 32-bit instruction (opcode + 16-bit parameter) and returns its address.
    pub fn write_opcode_i16(&mut self, opcode: Opcode, value: u16) -> usize {
        let addr = self.put_opcode(opcode);
        self.put_i16(value);
        addr
    }

    /// Writes a 64-bit instruction (opcode + padding + 32-bit parameter) and returns its address.
    pub fn write_opcode_i32(&mut self, opcode: Opcode, value: u32) -> usize {
        let addr = self.insert_padding_if_necessary();
        self.put_opcode_with_padding(opcode);
        self.put_i32(value);
        addr
    }

    /// Writes a 64-bit instruction (opcode + 16-bit parameter + 32-bit parameter) and returns its address.
    pub fn write_opcode_i16_i32(&mut self, opcode: Opcode, param0: u16, param1: u32) -> usize {
        let addr = self.insert_padding_if_necessary();
        self.put_opcode(opcode);
        self.put_i16(param0);
        self.put_i32(param1);
        addr
    }

    /// Writes a 64-bit instruction (opcode + three 16-bit parameters) and returns its address.
    pub fn write_opcode_i16_i16_i16(
        &mut self,
        opcode: Opcode,
        param0: u16,
        param1: u16,
        param2: u16,
    ) -> usize {
        let addr = self.put_opcode(opcode);
        self.put_i16(param0);
        self.put_i16(param1);
        self.put_i16(param2);
        addr
    }

    /// Writes a 96-bit instruction (opcode + padding + two 32-bit parameters) and returns its address.
    pub fn write_opcode_i32_i32(&mut self, opcode: Opcode, param0: u32, param1: u32) -> usize {
        let addr = self.insert_padding_if_necessary();
        self.put_opcode_with_padding(opcode);
        self.put_i32(param0);
        self.put_i32(param1);
        addr
    }

    /// Writes a 128-bit instruction (opcode + padding + three 32-bit parameters) and returns its address.
    pub fn write_opcode_i32_i32_i32(
        &mut self,
        opcode: Opcode,
        param0: u32,
        param1: u32,
        param2: u32,
    ) -> usize {
        let addr = self.insert_padding_if_necessary();
        self.put_opcode_with_padding(opcode);
        self.put_i32(param0);
        self.put_i32(param1);
        self.put_i32(param2);
        addr
    }

    // Pseudo-instructions for handling i64, f32, and f64 parameters.
    // These are not native to the ISA but are represented as combinations of smaller parameters.

    /// Writes a 96-bit pseudo-instruction (opcode + padding + 64-bit parameter) and returns its address.
    pub fn write_opcode_i64(&mut self, opcode: Opcode, value: u64) -> usize {
        let addr = self.insert_padding_if_necessary();
        self.put_opcode_with_padding(opcode);
        self.put_i64(value);
        addr
    }

    /// Writes a 64-bit pseudo-instruction (opcode + padding + 32-bit parameter) and returns its address.
    pub fn write_opcode_f32(&mut self, opcode: Opcode, value: f32) -> usize {
        let addr = self.insert_padding_if_necessary();
        self.put_opcode_with_padding(opcode);
        self.put_f32(value);
        addr
    }

    /// Writes a 96-bit pseudo-instruction (opcode + padding + 64-bit parameter) and returns its address.
    pub fn write_opcode_f64(&mut self, opcode: Opcode, value: f64) -> usize {
        let addr = self.insert_padding_if_necessary();
        self.put_opcode_with_padding(opcode);
        self.put_f64(value);
        addr
    }

    /// Converts the buffer into a byte vector.
    pub fn to_bytes(self) -> Vec<u8> {
        self.buffer
    }

    /// Writes the buffer to an external writer.
    pub fn write(&self, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        writer.write_all(&self.buffer)
    }
}

impl BytecodeWriter {
    // Rewrites a 32-bit value at a specific address in the buffer.
    fn rewrite_buffer(&mut self, addr: usize, value: u32) {
        self.buffer[addr..(addr + 4)].copy_from_slice(value.to_le_bytes().as_ref());
    }

    /// Returns the current address in the buffer.
    pub fn get_addr(&self) -> usize {
        self.buffer.len()
    }

    /// Returns the next aligned address in the buffer.
    pub fn get_addr_with_align(&self) -> usize {
        let addr_of_next_inst = self.get_addr();
        if addr_of_next_inst % 4 != 0 {
            addr_of_next_inst + 2
        } else {
            addr_of_next_inst
        }
    }

    pub fn fill_break_stub(&mut self, addr: usize, next_inst_offset: u32) {
        // (opcode:i16 reversed_index:i16, next_inst_offset:i32)
        // Also applies to the 'break_alt' instruction.
        self.rewrite_buffer(addr + 4, next_inst_offset);
    }

    pub fn fill_block_alt_stub(&mut self, addr: usize, next_inst_offset: u32) {
        // (opcode:i16 padding:i16 type_index:i32 local_variable_list_index:i32 next_inst_offset:i32)
        self.rewrite_buffer(addr + 12, next_inst_offset);
    }

    pub fn fill_block_nez_stub(&mut self, addr: usize, next_inst_offset: u32) {
        // (opcode:i16 padding:i16 local_variable_list_index:i32 next_inst_offset:i32)
        self.rewrite_buffer(addr + 8, next_inst_offset);
    }
}

pub struct BytecodeWriterHelper {
    writer: BytecodeWriter,
}

/// Chain calling style for appending opcodes.
impl BytecodeWriterHelper {
    pub fn new() -> Self {
        BytecodeWriterHelper {
            writer: BytecodeWriter::new(),
        }
    }

    pub fn append_opcode(mut self, opcode: Opcode) -> Self {
        self.writer.write_opcode(opcode);
        self
    }

    pub fn append_opcode_i16(mut self, opcode: Opcode, value: u16) -> Self {
        self.writer.write_opcode_i16(opcode, value);
        self
    }

    pub fn append_opcode_i32(mut self, opcode: Opcode, value: u32) -> Self {
        self.writer.write_opcode_i32(opcode, value);
        self
    }

    pub fn append_opcode_i16_i32(mut self, opcode: Opcode, param0: u16, param1: u32) -> Self {
        self.writer.write_opcode_i16_i32(opcode, param0, param1);
        self
    }

    pub fn append_opcode_i16_i16_i16(
        mut self,
        opcode: Opcode,
        param0: u16,
        param1: u16,
        param2: u16,
    ) -> Self {
        self.writer
            .write_opcode_i16_i16_i16(opcode, param0, param1, param2);
        self
    }

    pub fn append_opcode_i32_i32(mut self, opcode: Opcode, param0: u32, param1: u32) -> Self {
        self.writer.write_opcode_i32_i32(opcode, param0, param1);
        self
    }

    pub fn append_opcode_i32_i32_i32(
        mut self,
        opcode: Opcode,
        param0: u32,
        param1: u32,
        param2: u32,
    ) -> Self {
        self.writer
            .write_opcode_i32_i32_i32(opcode, param0, param1, param2);
        self
    }

    pub fn append_opcode_i64(mut self, opcode: Opcode, value: u64) -> Self {
        self.writer.write_opcode_i64(opcode, value);
        self
    }

    pub fn append_opcode_f32(mut self, opcode: Opcode, value: f32) -> Self {
        self.writer.write_opcode_f32(opcode, value);
        self
    }

    pub fn append_opcode_f64(mut self, opcode: Opcode, value: f64) -> Self {
        self.writer.write_opcode_f64(opcode, value);
        self
    }

    pub fn to_bytes(self) -> Vec<u8> {
        self.writer.to_bytes()
    }
}

impl Default for BytecodeWriterHelper {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use anc_isa::opcode::Opcode;
    use pretty_assertions::assert_eq;

    use crate::bytecode_writer::BytecodeWriterHelper;

    #[test]
    fn test_bytecode_writer() {
        // 16 bits
        let code0 = BytecodeWriterHelper::new()
            .append_opcode(Opcode::add_i32)
            .to_bytes();

        assert_eq!(code0, vec![0x00, 0x03]);

        // 32 bits
        let code1 = BytecodeWriterHelper::new()
            .append_opcode_i16(Opcode::add_imm_i32, 7)
            .to_bytes();

        assert_eq!(
            code1,
            vec![
                0x02, 0x03, // opcode
                7, 0, // param
            ]
        );

        // 64 bits - 1 param
        let code2 = BytecodeWriterHelper::new()
            .append_opcode_i32(Opcode::imm_i32, 11)
            .to_bytes();

        assert_eq!(
            code2,
            vec![
                0x40, 0x01, // opcode
                0, 0, // padding
                11, 0, 0, 0 // param
            ]
        );

        // 64 bits - 2 params
        let code3 = BytecodeWriterHelper::new()
            .append_opcode_i16_i32(Opcode::break_, 13, 17)
            .to_bytes();

        assert_eq!(
            code3,
            vec![
                0xc2, 0x03, // opcode
                13, 0, // param 0
                17, 0, 0, 0 // param 1
            ]
        );

        // 64 bits - 3 params
        let code4 = BytecodeWriterHelper::new()
            .append_opcode_i16_i16_i16(Opcode::local_load_i64, 19, 23, 29)
            .to_bytes();

        assert_eq!(
            code4,
            vec![
                0x80, 0x01, // opcode
                19, 0, // param 0
                23, 0, // param 1
                29, 0 // param 2
            ]
        );

        // 96 bits - 2 params
        let code5 = BytecodeWriterHelper::new()
            .append_opcode_i32_i32(Opcode::block, 31, 37)
            .to_bytes();

        assert_eq!(
            code5,
            vec![
                0xc1, 0x03, // opcode
                0, 0, // padding
                31, 0, 0, 0, // param 0
                37, 0, 0, 0 // param 1
            ]
        );

        // 128 bits - 3 params
        let code6 = BytecodeWriterHelper::new()
            .append_opcode_i32_i32_i32(Opcode::block_alt, 41, 73, 79)
            .to_bytes();

        assert_eq!(
            code6,
            vec![
                0xc4, 0x03, // opcode
                0, 0, // padding
                41, 0, 0, 0, // param 0
                73, 0, 0, 0, // param 1
                79, 0, 0, 0 // param 2
            ]
        );
    }

    #[test]
    fn test_bytecode_writer_with_i64_f32_f64_params() {
        // Pseudo f32
        let code0 = BytecodeWriterHelper::new()
            .append_opcode_f32(Opcode::imm_f32, std::f32::consts::PI)
            .to_bytes();

        // 3.1415927 -> 0x40490FDB
        assert_eq!(
            code0,
            vec![
                0x42, 0x01, // opcode
                0, 0, // padding
                0xdb, 0x0f, 0x49, 0x40, // param 0
            ]
        );

        let code1 = BytecodeWriterHelper::new()
            .append_opcode_i64(Opcode::imm_i64, 0x1122334455667788u64)
            .to_bytes();

        assert_eq!(
            code1,
            vec![
                0x41, 0x01, // opcode
                0, 0, // padding
                0x88, 0x77, 0x66, 0x55, // param 0
                0x44, 0x33, 0x22, 0x11 // param 1
            ]
        );

        let code2 = BytecodeWriterHelper::new()
            .append_opcode_f64(Opcode::imm_f64, 6.62607015e-34f64)
            .to_bytes();

        // 6.62607015e-34f64 (dec) -> 0x390B860B DE023111 (hex)

        assert_eq!(
            code2,
            vec![
                0x43, 0x01, // opcode
                0, 0, // padding
                0x11, 0x31, 0x02, 0xde, // param 0
                0x0b, 0x86, 0x0b, 0x39, // param 1
            ]
        );
    }

    #[test]
    fn test_bytecode_writer_auto_padding() {
        // Test
        // - write_opcode
        // - write_opcode_i16
        // - write_opcode_i16_i16_i16
        {
            let data = BytecodeWriterHelper::new()
                .append_opcode(Opcode::eqz_i32)
                .append_opcode_i16(Opcode::add_imm_i32, 0x2)
                .append_opcode_i16_i16_i16(Opcode::local_load_i64, 0x5, 0x7, 0x11)
                .to_bytes();

            assert_eq!(
                data,
                vec![
                    0xc0, 0x02, // eqz_i32
                    0x02, 0x03, 0x02, 0x00, // add_imm_i32
                    0x80, 0x01, 0x05, 0x00, 0x07, 0x00, 0x11, 0x00 // local_load_i64
                ]
            );
        }

        // Test
        // - write_opcode_i32
        // - write_opcode_i16_i32
        // - write_opcode_i32_i32
        // - write_opcode_i32_i32_i32
        {
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

            assert_eq!(
                data,
                vec![
                    0xc0, 0x02, // eqz_i32
                    0x00, 0x01, // NOP (auto padding)
                    0x40, 0x01, 0x00, 0x00, 0x13, 0x00, 0x00, 0x00, // imm_i32
                    0x02, 0x03, 0x02, 0x00, // add_imm_i32
                    0x40, 0x01, 0x00, 0x00, 0x13, 0x00, 0x00, 0x00, // imm_i32
                    //
                    0xc0, 0x02, // eqz_i32
                    0x00, 0x01, // NOP (auto padding)
                    0xc0, 0x01, 0x17, 0x00, 0x19, 0x00, 0x00, 0x00, // data_load_i64
                    0x02, 0x03, 0x02, 0x00, // add_imm_i32
                    0xc0, 0x01, 0x17, 0x00, 0x19, 0x00, 0x00, 0x00, // data_load_i64
                    //
                    0xc0, 0x02, // eqz_i32
                    0x00, 0x01, // NOP (auto padding)
                    0xc1, 0x03, 0x00, 0x00, 0x23, 0x00, 0x00, 0x00, 0x29, 0x00, 0x00,
                    0x00, // block
                    0x02, 0x03, 0x02, 0x00, // add_imm_i32
                    0xc1, 0x03, 0x00, 0x00, 0x23, 0x00, 0x00, 0x00, 0x29, 0x00, 0x00,
                    0x00, // block
                    //
                    0xc0, 0x02, // eqz_i32
                    0x00, 0x01, // NOP (auto padding)
                    0xc4, 0x03, 0x00, 0x00, 0x31, 0x00, 0x00, 0x00, 0x37, 0x00, 0x00, 0x00, 0x41,
                    0x00, 0x00, 0x00, // block_alt
                    0x02, 0x03, 0x02, 0x00, // add_imm_i32
                    0xc4, 0x03, 0x00, 0x00, 0x31, 0x00, 0x00, 0x00, 0x37, 0x00, 0x00, 0x00, 0x41,
                    0x00, 0x00, 0x00, // block_alt
                ]
            );
        }

        // Test
        // - write_opcode_i64
        // - write_opcode_f32
        // - write_opcode_f64
        {
            let data = BytecodeWriterHelper::new()
                .append_opcode(Opcode::eqz_i32)
                .append_opcode_i64(Opcode::imm_i64, 0x13)
                .append_opcode_i64(Opcode::imm_i64, 0x17)
                //
                .append_opcode(Opcode::eqz_i32)
                .append_opcode_f32(Opcode::imm_f32, std::f32::consts::E)
                .append_opcode_f32(Opcode::imm_f32, std::f32::consts::E)
                .append_opcode(Opcode::eqz_i32)
                .append_opcode_f64(Opcode::imm_f64, std::f64::consts::E)
                .append_opcode_f64(Opcode::imm_f64, std::f64::consts::E)
                .to_bytes();

            assert_eq!(
                data,
                vec![
                    0xc0, 0x02, // eqz_i32
                    0x00, 0x01, // NOP (auto padding)
                    0x41, 0x01, 0x00, 0x00, 0x13, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, // imm_i64
                    0x41, 0x01, 0x00, 0x00, 0x17, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, // imm_i64
                    //
                    0xc0, 0x02, // eqz_i32
                    0x00, 0x01, // NOP (auto padding)
                    0x42, 0x01, 0x00, 0x00, 0x54, 0xf8, 0x2d, 0x40, // imm_f32
                    0x42, 0x01, 0x00, 0x00, 0x54, 0xf8, 0x2d, 0x40, // imm_f32
                    0xc0, 0x02, // eqz_i32
                    0x00, 0x01, // NOP (auto padding)
                    0x43, 0x01, 0x00, 0x00, 0x69, 0x57, 0x14, 0x8b, 0x0a, 0xbf, 0x05,
                    0x40, // imm_f64
                    0x43, 0x01, 0x00, 0x00, 0x69, 0x57, 0x14, 0x8b, 0x0a, 0xbf, 0x05,
                    0x40, // imm_f64
                ]
            );
        }
    }
}
