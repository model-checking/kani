// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! GOTO binary serializer.
use crate::irep::{Irep, IrepId, Symbol, SymbolTable};
use crate::{InternString, InternedString};
use std::collections::HashMap;
use std::fs::File;
use std::hash::Hash;
use std::io::{self, BufReader};
use std::io::{BufWriter, Bytes, Error, ErrorKind, Read, Write};
use std::path::PathBuf;

/// Writes a symbol table to a file in goto binary format in version 5.
pub fn write_goto_binary_file(
    symbol_table: &crate::goto_program::SymbolTable,
    filename: &PathBuf,
) -> io::Result<()> {
    let out_file = File::create(filename).expect("could not create output file {filename}");
    let mut writer = BufWriter::new(out_file);
    let mut serializer = GotoBinarySerializer::new(&mut writer);
    let irep_symbol_table = &symbol_table.to_irep();
    serializer.write_file(irep_symbol_table)
}

/// A numbered InternedString. The number is guaranteed to be in [0,N].
/// Had to introduce this indirection because InternedString does not let you access
/// its unique id, so we have to build one ourselves.
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
struct NumberedString {
    number: usize,
    string: InternedString,
}

/// A key representing an Irep as the vector of unique numbers describing its contents.
/// If:
/// - `#sub` is the number of sub
/// - `#named_sub` is the number of named sub
/// Then:
/// - the size of the key is `3 + #sub + 2 * #named_sub`.
/// - the unique numbers must be pushed in the following order:
/// ```
/// number(id)
/// #sub
/// number(sub[0])
/// ...
/// number(sub[#sub-1])
/// #named_sub
/// number(named_sub[0].key)
/// number(named_sub[0].value)
/// ...
/// number(named_sub[#named_sub-1].key)
/// number(named_sub[#named_sub-1].value)
/// ```
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
struct IrepKey {
    numbers: Vec<usize>,
}

impl IrepKey {
    fn new(id: usize, sub: &[usize], named_sub: &[(usize, usize)]) -> Self {
        let mut vec: Vec<usize> = Vec::new();
        let size = sub.len() + 2 * named_sub.len() + 3;
        vec.reserve_exact(size);
        vec.push(id);
        vec.push(sub.len());
        vec.extend_from_slice(sub);
        vec.push(named_sub.len());
        for (k, v) in named_sub {
            vec.push(*k);
            vec.push(*v);
        }
        IrepKey { numbers: vec }
    }
}

/// Inverse cache of unique NumberedIrep objects.
struct IrepNumberingInv {
    // Maps irep numbers to numbered ireps
    index: Vec<NumberedIrep>,

    // Stores the concactenation of all irep keys seen by the IrepNumbering owning this object
    keys: Vec<usize>,
}

impl IrepNumberingInv {
    fn new() -> Self {
        IrepNumberingInv { index: Vec::new(), keys: Vec::new() }
    }

    fn add_key(&mut self, key: &IrepKey, number: usize) {
        assert_eq!(number, self.index.len());
        self.index.push(NumberedIrep { number: number, start_index: self.keys.len() });
        self.keys.extend(&key.numbers);
    }

    fn numbered_irep_from_number(&self, irep_number: usize) -> Option<NumberedIrep> {
        self.index.get(irep_number).map(|result| *result)
    }
}

/// Unique numbering of InternedString, IrepId and Irep based on their contents.
struct IrepNumbering {
    /// Maps each key to its unique number
    string_cache: HashMap<InternedString, usize>,

    /// Inverse string cache
    inv_string_cache: Vec<NumberedString>,

    /// Maps each irep key to its unique number
    cache: HashMap<IrepKey, usize>,

    /// Inverse cache
    inv_cache: IrepNumberingInv,
}

impl IrepNumbering {
    fn new() -> Self {
        IrepNumbering {
            string_cache: HashMap::new(),
            inv_string_cache: Vec::new(),
            cache: HashMap::new(),
            inv_cache: IrepNumberingInv::new(),
        }
    }

    fn numbered_string_from_number(&mut self, string_number: usize) -> Option<NumberedString> {
        self.inv_string_cache.get(string_number).map(|result| *result)
    }

    fn numbered_irep_from_number(&mut self, irep_number: usize) -> Option<NumberedIrep> {
        self.inv_cache.numbered_irep_from_number(irep_number)
    }

    /// Translates an InternedString to a NumberedString.
    fn number_string(&mut self, string: &InternedString) -> NumberedString {
        let len = self.string_cache.len();
        let entry = self.string_cache.entry(*string);
        let number = *entry.or_insert_with(|| {
            self.inv_string_cache.push(NumberedString { number: len, string: *string });
            len
        });
        self.inv_string_cache[number]
    }

    /// Translates an IrepId to a NumberedString. The IrepId get the number of their
    /// string representation.
    fn number_irep_id(&mut self, irep_id: &IrepId) -> NumberedString {
        self.number_string(&irep_id.to_string().intern())
    }

    /// Translates an Irep to a NumberedIrep. The irep is recursively traversed
    /// and numbered in a bottom-up fashion. Structurally identical Irep
    /// result in the same NumberedIrep.
    fn number_irep(&mut self, irep: &Irep) -> NumberedIrep {
        // build the key
        let id = self.number_irep_id(&irep.id).number;
        let sub: Vec<usize> = irep.sub.iter().map(|sub| self.number_irep(sub).number).collect();
        let named_sub: Vec<(usize, usize)> = irep
            .named_sub
            .iter()
            .map(|(key, value)| (self.number_irep_id(key).number, self.number_irep(value).number))
            .collect();
        let key = IrepKey::new(id, &sub, &named_sub);
        self.get_or_insert(&key)
    }

    /// Gets the existing NumberedIrep from the Irepkey or inserts a fresh
    /// one and returns it.
    fn get_or_insert(&mut self, key: &IrepKey) -> NumberedIrep {
        if let Some(number) = self.cache.get(key) {
            // Return the NumberedIrep from the inverse cache
            return self.inv_cache.index[*number];
        }
        let next_number = self.cache.len();
        self.inv_cache.add_key(&key, next_number);
        self.cache.insert(key.clone(), next_number);
        return self.inv_cache.index[next_number];
    }
}

/// A uniquely numbered Irep. Its meaning can on be correctly interpreted with
/// respect to the Numbering instance that produced it.
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
struct NumberedIrep {
    // The unique number of this NumberedIrep.
    number: usize,
    // Start index of the IrepKey of this NumberedIrep in the IrepNumbering cache.
    start_index: usize,
}

impl NumberedIrep {
    /// Returns the unique number of the `id` field of this NumberedIrep.
    /// It is found at index `start_index` in the inverse cache.
    fn id(&self, cache: &IrepNumbering) -> NumberedString {
        cache.inv_string_cache[cache.inv_cache.keys[self.start_index]]
    }

    /// Returns `#sub`, the number of `sub` ireps of this NumberedIrep.
    /// It is found at `start_index + 1` in the inverse cache.
    fn nof_sub(&self, cache: &IrepNumbering) -> usize {
        cache.inv_cache.keys[self.start_index + 1]
    }

    /// Returns the numbered irep for the ith `sub` of this numbered irep.
    fn sub(&self, cache: &IrepNumbering, sub_idx: usize) -> NumberedIrep {
        let sub_number = cache.inv_cache.keys[self.start_index + sub_idx + 2];
        cache.inv_cache.index[sub_number]
    }

    /// Returns `#named_sub`, the number of named subs of this numbered irep.
    /// It is found at `start_index + #sub + 2` in the inverse cache.
    fn nof_named_sub(&self, cache: &IrepNumbering) -> usize {
        cache.inv_cache.keys[self.start_index + self.nof_sub(cache) + 2]
    }

    /// Returns a pair formed of a NumberedString and NumberedIrep describing the named
    /// sub number `i` of this numbered irep.
    fn named_sub(
        &self,
        cache: &IrepNumbering,
        named_sub_idx: usize,
    ) -> (NumberedString, NumberedIrep) {
        let start_index = self.start_index + self.nof_sub(cache) + 2 * named_sub_idx + 3;
        (
            cache.inv_string_cache[cache.inv_cache.keys[start_index]],
            cache.inv_cache.index[cache.inv_cache.keys[start_index + 1]],
        )
    }
}
/// GOTO binary serializer. Uses an IrepNumbering to implement sharing during
/// serialization.
struct GotoBinarySerializer<'a, W>
where
    W: Write,
{
    writer: &'a mut W,

    /// In-memory temporary buffer, contents get flushed after each object
    buf: Vec<u8>,

    /// Numbering used for structural sharing.
    numbering: IrepNumbering,

    /// Counts how many times a given irep was written.
    irep_count: Vec<usize>,

    /// Counts how many times a given string was written.
    string_count: Vec<usize>,
}

impl<'a, W> GotoBinarySerializer<'a, W>
where
    W: Write,
{
    /// Constructor.
    fn new(writer: &'a mut W) -> Self {
        GotoBinarySerializer {
            writer,
            buf: Vec::new(),
            numbering: IrepNumbering::new(),
            irep_count: Vec::new(),
            string_count: Vec::new(),
        }
    }

    /// Adds an InternedString uid to the "written" cache, returns true iff was never written before.
    fn is_first_write_string(&mut self, u: usize) -> bool {
        if u >= self.string_count.len() {
            self.string_count.resize(u + 1, 0);
        }
        let count = self.string_count[u];
        self.string_count[u] = count.checked_add(1).unwrap();
        count == 0
    }

    /// Adds an Irep uid to the "written" cache, returns true iff it was never written before.
    fn is_first_write_irep(&mut self, u: usize) -> bool {
        if u >= self.irep_count.len() {
            self.irep_count.resize(u + 1, 0);
        }
        let count = self.irep_count[u];
        self.irep_count[u] = count.checked_add(1).unwrap();
        count == 0
    }

    /// Flushes the temporary buffer to the external writer,
    /// flushes the writer and clears the temporary buffer.
    fn flush(&mut self) -> io::Result<()> {
        self.writer.write_all(&self.buf)?;
        self.buf.clear();
        Ok(())
    }

    /// Writes a single byte to the temporary buffer.
    fn write_u8(&mut self, u: u8) -> io::Result<()> {
        self.buf.push(u);
        Ok(())
    }

    /// Writes a usize to the temporary buffer using 7-bit variable length encoding.
    fn write_usize_varenc(&mut self, mut u: usize) -> io::Result<()> {
        loop {
            let mut v: u8 = (u & 0x7f) as u8;
            u = u >> 7;
            if u == 0 {
                // all remaining bits of u are zero
                self.buf.push(v);
                break;
            }
            // there are more bits in u, set the 8th bit to indicate continuation
            v = v | 0x80;
            self.buf.push(v);
        }
        Ok(())
    }

    /// Writes a numbered string to the buffer. Writes the unique number of the string,
    /// and writes the actual string only if was never written before.
    fn write_numbered_string_ref(&mut self, numbered_string: &NumberedString) -> io::Result<()> {
        let num = numbered_string.number;
        self.write_usize_varenc(num)?;
        if self.is_first_write_string(num) {
            // first occurrence
            numbered_string.string.map(|raw_str| {
                for c in raw_str.chars() {
                    if c.is_ascii() {
                        if c == '0' || c == '\\' {
                            self.buf.push('\\' as u8);
                        }
                        self.buf.push(c as u8);
                    } else {
                        let mut buf = [0; 4];
                        c.encode_utf8(&mut buf);
                        for u in buf {
                            if u == 0 {
                                break;
                            }
                            self.buf.push(u);
                        }
                    }
                }
                // write terminator
                self.buf.push(0u8);
            });
        }
        self.flush()?;
        Ok(())
    }

    /// Writes a numbered irep to the buffer. Writes the unique number of the irep,
    /// and writes the actual irep contents only if was never written before.
    fn write_numbered_irep_ref(&mut self, irep: &NumberedIrep) -> io::Result<()> {
        let num = irep.number;
        self.write_usize_varenc(num)?;

        if self.is_first_write_irep(num) {
            let id = &irep.id(&self.numbering);
            self.write_numbered_string_ref(id)?;

            for sub_idx in 0..(irep.nof_sub(&self.numbering)) {
                self.write_u8(b'S')?;
                self.write_numbered_irep_ref(&irep.sub(&self.numbering, sub_idx))?;
            }

            for named_sub_idx in 0..(irep.nof_named_sub(&self.numbering)) {
                self.write_u8(b'N')?;
                let (k, v) = irep.named_sub(&self.numbering, named_sub_idx);
                self.write_numbered_string_ref(&k)?;
                self.write_numbered_irep_ref(&v)?;
            }

            self.write_u8(0)?; // terminator
        }
        self.flush()?;
        Ok(())
    }

    /// Translates the string to its numbered version and serializes it.
    fn write_string_ref(&mut self, str: &InternedString) -> io::Result<()> {
        let numbered_string = &self.numbering.number_string(str);
        self.write_numbered_string_ref(numbered_string)
    }

    /// Translates the irep to its numbered version and serializes it.
    fn write_irep_ref(&mut self, irep: &Irep) -> io::Result<()> {
        let numbered_irep = self.numbering.number_irep(irep);
        self.write_numbered_irep_ref(&numbered_irep)
    }

    /// Writes a symbol to the byte stream.
    fn write_symbol(&mut self, symbol: &Symbol) -> io::Result<()> {
        self.write_irep_ref(&symbol.typ)?;
        self.write_irep_ref(&symbol.value)?;
        self.write_irep_ref(&symbol.location)?;
        self.write_string_ref(&symbol.name)?;
        self.write_string_ref(&symbol.module)?;
        self.write_string_ref(&symbol.base_name)?;
        self.write_string_ref(&symbol.mode)?;
        self.write_string_ref(&symbol.pretty_name)?;
        self.write_u8(0)?;

        let mut flags: usize = 0;
        flags = (flags << 1) | (symbol.is_weak) as usize;
        flags = (flags << 1) | (symbol.is_type) as usize;
        flags = (flags << 1) | (symbol.is_property) as usize;
        flags = (flags << 1) | (symbol.is_macro) as usize;
        flags = (flags << 1) | (symbol.is_exported) as usize;
        flags = (flags << 1) | (symbol.is_input) as usize;
        flags = (flags << 1) | (symbol.is_output) as usize;
        flags = (flags << 1) | (symbol.is_state_var) as usize;
        flags = (flags << 1) | (symbol.is_parameter) as usize;
        flags = (flags << 1) | (symbol.is_auxiliary) as usize;
        flags = (flags << 1) | (false) as usize; // sym.binding;
        flags = (flags << 1) | (symbol.is_lvalue) as usize;
        flags = (flags << 1) | (symbol.is_static_lifetime) as usize;
        flags = (flags << 1) | (symbol.is_thread_local) as usize;
        flags = (flags << 1) | (symbol.is_file_local) as usize;
        flags = (flags << 1) | (symbol.is_extern) as usize;
        flags = (flags << 1) | (symbol.is_volatile) as usize;

        self.write_usize_varenc(flags)?;
        self.flush()?;
        Ok(())
    }

    /// Writes a symbol table to the byte stream.
    fn write_symbol_table(&mut self, symbol_table: &SymbolTable) -> io::Result<()> {
        // Write symbol table size
        self.write_usize_varenc(symbol_table.symbol_table.len())?;

        // Write symbols
        for symbol in symbol_table.symbol_table.values() {
            self.write_symbol(symbol)?;
        }

        self.flush()?;
        Ok(())
    }

    /// Writes an empty function map to the byte stream.
    fn write_function_map(&mut self) -> io::Result<()> {
        // Write empty GOTO functions map
        self.write_usize_varenc(0)?;
        self.flush()?;
        Ok(())
    }

    /// Writes a GOTO binary file header to the byte stream.
    fn write_header(&mut self) -> io::Result<()> {
        // Write header
        self.write_u8(0x7f)?;
        self.write_u8(b'G')?;
        self.write_u8(b'B')?;
        self.write_u8(b'F')?;

        // Write goto binary version
        self.write_usize_varenc(5)?;
        self.flush()?;
        Ok(())
    }

    /// Writes the symbol table using the GOTO binary file format to the byte stream.
    fn write_file(&mut self, symbol_table: &SymbolTable) -> io::Result<()> {
        self.write_header()?;
        self.write_symbol_table(symbol_table)?;
        self.write_function_map()?;
        self.flush()?;
        Ok(())
    }
}
