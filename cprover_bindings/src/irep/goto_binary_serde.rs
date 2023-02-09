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

/// Reads a symbol table from a file expected to be in goto binary format in version 5.
pub fn read_goto_binary_file(filename: &PathBuf) -> io::Result<()> {
    let file = File::open(filename).expect("could not open input file {filename}");
    let reader = BufReader::new(file);
    let mut deserializer = GotoBinaryDeserializer::new(reader);
    deserializer.read_file()
}

/// GOTO binary deserializer. Reads GOTO constructs from the byte stream of a reader.
struct GotoBinaryDeserializer<R>
where
    R: Read,
{
    /// Stream of bytes from which GOTO objects are read.
    bytes: Bytes<R>,

    /// Numbering for ireps
    numbering: IrepNumbering,

    /// Counts how many times a given irep was read.
    irep_count: Vec<usize>,

    /// Maps the irep number used in the binary stream to the new one generated by our own numbering.
    irep_map: Vec<Option<usize>>,

    /// Counts how many times a given string was read.
    string_count: Vec<usize>,

    /// Maps the string number used in the binary stream to the new one generated by our own numbering.
    string_map: Vec<Option<usize>>,
}

impl<R> GotoBinaryDeserializer<R>
where
    R: Read,
{
    /// Constructor. The reader is moved into this object and cannot be used
    /// afterwards.
    fn new(reader: R) -> Self {
        GotoBinaryDeserializer {
            bytes: reader.bytes(),
            numbering: IrepNumbering::new(),
            string_count: Vec::new(),
            string_map: Vec::new(),
            irep_count: Vec::new(),
            irep_map: Vec::new(),
        }
    }

    /// Returns Err if the found value is not the expected value.
    fn expect<T: Eq + std::fmt::Display>(found: T, expected: T) -> io::Result<T> {
        if found != expected {
            return Err(Error::new(
                ErrorKind::Other,
                format!("expected {} in byte stream, found {} instead)", expected, found),
            ));
        }
        Ok(found)
    }

    /// Adds an InternedString unique number to the "read" cache, returns true iff was never read before.
    fn is_first_read_string(&mut self, u: usize) -> bool {
        if u >= self.string_count.len() {
            self.string_count.resize(u + 1, 0);
        }
        let count = self.string_count[u];
        self.string_count[u] = count.checked_add(1).unwrap();
        count == 0
    }

    /// Maps a string number used in the byte stream to the number generated by our own numbering for that string.
    fn add_string_mapping(&mut self, num_binary: usize, num: usize) {
        if num_binary >= self.string_map.len() {
            self.string_map.resize(num_binary + 1, None);
        }
        let old = self.string_map[num_binary];
        if !old.is_none() {
            panic!("string number already mapped");
        }
        self.string_map[num_binary] = Some(num);
    }

    /// Adds an Irep unique number to the "read" cache, returns true iff it was never read before.
    fn is_first_read_irep(&mut self, u: usize) -> bool {
        if u >= self.irep_count.len() {
            self.irep_count.resize(u + 1, 0);
        }
        let count = self.irep_count[u];
        self.irep_count[u] = count.checked_add(1).unwrap();
        count == 0
    }

    /// Maps an Irep number used in the byte stream to the number generated by our own numbering for that Irep.
    fn add_irep_mapping(&mut self, num_binary: usize, num: usize) {
        if num_binary >= self.irep_map.len() {
            self.irep_map.resize(num_binary + 1, None);
        }
        let old = self.irep_map[num_binary];
        if !old.is_none() {
            panic!("irep number already mapped");
        }
        self.irep_map[num_binary] = Some(num);
    }

    /// Reads a u8 from the byte stream.
    fn read_u8(&mut self) -> io::Result<u8> {
        match self.bytes.next() {
            Some(Ok(u)) => {
                return Ok(u);
            }
            Some(Err(error)) => {
                return Err(error);
            }
            None => {
                return Err(Error::new(ErrorKind::Other, "unexpected end of input"));
            }
        }
    }

    /// Reads a usize from the byte stream assuming 7-bit variable length encoding.
    fn read_usize_varenc(&mut self) -> io::Result<usize> {
        let mut result: usize = 0;
        let mut shift: usize = 0;
        let max_shift: usize = std::mem::size_of::<usize>() * std::mem::size_of::<u8>() * 8;
        loop {
            match self.bytes.next() {
                Some(Ok(u)) => {
                    if shift >= max_shift {
                        return Err(Error::new(
                            ErrorKind::Other,
                            "serialized value is too large to fit in usize",
                        ));
                    };
                    result = result | (((u & 0x7f) as usize) << shift);
                    shift = shift.checked_add(7).unwrap();
                    if u & (0x80 as u8) == 0 {
                        return Ok(result);
                    }
                }
                Some(Err(error)) => {
                    return Err(error);
                }
                None => {
                    return Err(Error::new(ErrorKind::Other, "unexpected end of input"));
                }
            }
        }
    }

    /// Reads a reference encoded string from the byte stream.
    fn read_numbered_string_ref(&mut self) -> io::Result<NumberedString> {
        let string_number_result = self.read_usize_varenc();
        let string_number = match string_number_result {
            Ok(number) => number,
            Err(error) => return Err(error),
        };
        if self.is_first_read_string(string_number) {
            // read raw string
            let mut string_buf: Vec<u8> = Vec::new();
            loop {
                match self.bytes.next() {
                    Some(Ok(u)) => {
                        match u {
                            0 => {
                                // Reached end of string
                                match String::from_utf8(string_buf) {
                                    Ok(str) => {
                                        let numbered = self.numbering.number_string(&str.intern());
                                        self.add_string_mapping(string_number, numbered.number);
                                        return Ok(numbered);
                                    }
                                    Err(error) => {
                                        return Err(Error::new(
                                            ErrorKind::Other,
                                            error.to_string(),
                                        ));
                                    }
                                }
                            }
                            b'\\' => {
                                // Found escape symbol, read the next char
                                match self.bytes.next() {
                                    Some(Ok(c)) => {
                                        string_buf.push(c);
                                    }
                                    Some(Err(error)) => {
                                        return Err(error);
                                    }
                                    None => {
                                        return Err(Error::new(
                                            ErrorKind::Other,
                                            "unexpected end of input",
                                        ));
                                    }
                                }
                            }
                            c => {
                                // Found normal char, push to buffer
                                string_buf.push(c);
                            }
                        }
                    }
                    Some(Err(error)) => {
                        // Could not read from byte stream, propagate
                        return Err(error);
                    }
                    None => {
                        // No more bytes left
                        return Err(Error::new(ErrorKind::Other, "unexpected end of input"));
                    }
                }
            }
        } else {
            // We already read this irep, fetch it from the numbering
            return Ok(self
                .numbering
                .numbered_string_from_number(self.string_map[string_number].unwrap())
                .unwrap());
        }
    }

    /// Reads a NumberedIrep from the byte stream.
    fn read_numbered_irep_ref(&mut self) -> io::Result<NumberedIrep> {
        let irep_number_result = self.read_usize_varenc();
        let irep_number = match irep_number_result {
            Ok(number) => number,
            Err(error) => return Err(error),
        };

        if self.is_first_read_irep(irep_number) {
            let id = self.read_numbered_string_ref()?.number;
            let mut sub_done = false;
            let mut sub: Vec<usize> = Vec::new();
            let mut named_sub: Vec<(usize, usize)> = Vec::new();
            loop {
                // read subs and named subs one by one until the 0 terminator is found
                let c = self.read_u8()?;
                match c {
                    b'S' => {
                        if sub_done {
                            return Err(Error::new(ErrorKind::Other, "incorrect binary structure"));
                        }
                        let decoded_sub = self.read_numbered_irep_ref()?;
                        sub.push(decoded_sub.number);
                    }
                    b'N' => {
                        sub_done = true;
                        let decoded_name = self.read_numbered_string_ref()?;
                        let decoded_sub = self.read_numbered_irep_ref()?;
                        named_sub.push((decoded_name.number, decoded_sub.number));
                    }
                    0 => {
                        // Reached the end of this irep
                        // Build the key
                        let key = IrepKey::new(id, &sub, &named_sub);

                        // Insert key in the numbering
                        let numbered = self.numbering.get_or_insert(&key);

                        // Map number from the binary to new number
                        self.add_irep_mapping(irep_number, numbered.number);
                        return Ok(numbered);
                    }
                    other => {
                        return Err(Error::new(
                            ErrorKind::Other,
                            format!("unexpected character in input stream {}", other as char),
                        ));
                    }
                }
            }
        } else {
            return Ok(self
                .numbering
                .numbered_irep_from_number(self.irep_map[irep_number].unwrap())
                .unwrap());
        }
    }

    /// Reads a Symbol from the byte stream.
    fn read_symbol(&mut self) -> io::Result<()> {
        // Read Irep attributes of the symbol
        let _typ = self.read_numbered_irep_ref()?;
        let _value = self.read_numbered_irep_ref()?;
        let _location = self.read_numbered_irep_ref()?;

        // Read string attributes of the symbol
        let _name = self.read_numbered_string_ref()?;
        let _module = self.read_numbered_string_ref()?;
        let _base_name = self.read_numbered_string_ref()?;
        let _mode = self.read_numbered_string_ref()?;
        let _pretty_name = self.read_numbered_string_ref()?;

        // obsolete: symordering
        let symordering = self.read_u8()?;
        Self::expect(symordering, 0)?;

        // Decode the bit-packed flags and extract bits one by one
        let flags: usize = self.read_usize_varenc()?;

        let _is_weak = (flags & (1 << 16)) != 0;
        let _is_type = (flags & (1 << 15)) != 0;
        let _is_property = (flags & (1 << 14)) != 0;
        let _is_macro = (flags & (1 << 13)) != 0;
        let _is_exported = (flags & (1 << 12)) != 0;
        let _is_input = (flags & (1 << 11)) != 0;
        let _is_output = (flags & (1 << 10)) != 0;
        let _is_state_var = (flags & (1 << 9)) != 0;
        let _is_parameter = (flags & (1 << 8)) != 0;
        let _is_auxiliary = (flags & (1 << 7)) != 0;
        let _is_binding = (flags & (1 << 6)) != 0; // deprecated
        let _is_lvalue = (flags & (1 << 5)) != 0;
        let _is_static_lifetime = (flags & (1 << 4)) != 0;
        let _is_thread_local = (flags & (1 << 3)) != 0;
        let _is_file_local = (flags & (1 << 2)) != 0;
        let _is_extern = (flags & (1 << 1)) != 0;
        let _is_volatile = (flags & 1) != 0;
        let _is_volatile = (flags & 0x1) != 0;

        let shifted_flags = flags >> 16;

        if shifted_flags != 0 {
            return Err(Error::new(
                ErrorKind::Other,
                "incorrect binary format: set bits remain in decoded symbol flags",
            ));
        }
        Ok(())
    }

    /// Reads a whole SymbolTable from the byte stream.
    fn read_symbol_table(&mut self) -> io::Result<()> {
        // Write symbol table size
        let symbol_table_len = self.read_usize_varenc()?;

        // Write symbols
        for _ in 0..symbol_table_len {
            self.read_symbol()?;
        }

        Ok(())
    }

    /// Reads an empty function map from the byte stream.
    fn read_function_map(&mut self) -> io::Result<()> {
        let goto_function_len = self.read_usize_varenc()?;
        Self::expect(goto_function_len, 0)?;
        Ok(())
    }

    /// Reads a GOTO binary header from the byte stream.
    fn read_header(&mut self) -> io::Result<()> {
        // Read header
        Self::expect(self.read_u8().unwrap(), 0x7f)?;
        Self::expect(self.read_u8().unwrap(), b'G')?;
        Self::expect(self.read_u8().unwrap(), b'B')?;
        Self::expect(self.read_u8().unwrap(), b'F')?;

        // Read goto binary version
        let goto_binary_version = self.read_usize_varenc()?;
        if goto_binary_version != 5 {
            return Err(Error::new(
                ErrorKind::Other,
                format!(
                    "unsupported GOTO binary version: {}. Supported version: {}",
                    goto_binary_version, 5
                ),
            ));
        }
        Ok(())
    }

    /// Read a GOTO binary file from the byte stream.
    fn read_file(&mut self) -> io::Result<()> {
        self.read_header()?;
        self.read_symbol_table()?;
        self.read_function_map()?;
        Ok(())
    }
}
