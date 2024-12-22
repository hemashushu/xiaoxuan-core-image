// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

// "relocate section" binary layout
//
//              |-----------------------------------------------|
//              | item count (u32) | (4 bytes padding)          |
//              |-----------------------------------------------|
//  item 0 -->  | list offset 0 (u32) | list item count 0 (u32) | <-- table
//  item 1 -->  | list offset 1       | list item count 1       |
//              | ...                                           |
//              |-----------------------------------------------|
// offset 0 --> | list data 0                                   | <-- data area
// offset 1 --> | list data 1                                   |
//              | ...                                           |
//              |-----------------------------------------------|
//
//
// the "list" is also a table, the layout of "list":
//
//          |--------|     |-------------------------------------------------------|
// list     | item 0 | --> | stub offset 0 (u32) | stub type 0 (u8) | pad (3 byte) |
// data0 -> | item 1 | --> | stub offset 1       | stub type 1      |              |
//          | ...    |     | ...                                                   |
//          |--------|     |-------------------------------------------------------|
//