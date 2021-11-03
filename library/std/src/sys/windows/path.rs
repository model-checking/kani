use super::{c, fill_utf16_buf, to_u16s};
use crate::ffi::OsStr;
use crate::io;
use crate::mem;
use crate::path::Path;
use crate::path::Prefix;
use crate::ptr;

#[cfg(test)]
mod tests;

pub const MAIN_SEP_STR: &str = "\\";
pub const MAIN_SEP: char = '\\';

/// # Safety
///
/// `bytes` must be a valid wtf8 encoded slice
#[inline]
unsafe fn bytes_as_os_str(bytes: &[u8]) -> &OsStr {
    // &OsStr is layout compatible with &Slice, which is compatible with &Wtf8,
    // which is compatible with &[u8].
    mem::transmute(bytes)
}

#[inline]
pub fn is_sep_byte(b: u8) -> bool {
    b == b'/' || b == b'\\'
}

#[inline]
pub fn is_verbatim_sep(b: u8) -> bool {
    b == b'\\'
}

pub fn parse_prefix(path: &OsStr) -> Option<Prefix<'_>> {
    use Prefix::{DeviceNS, Disk, Verbatim, VerbatimDisk, VerbatimUNC, UNC};

    if let Some(path) = strip_prefix(path, r"\\") {
        // \\
        if let Some(path) = strip_prefix(path, r"?\") {
            // \\?\
            if let Some(path) = strip_prefix(path, r"UNC\") {
                // \\?\UNC\server\share

                let (server, path) = parse_next_component(path, true);
                let (share, _) = parse_next_component(path, true);

                Some(VerbatimUNC(server, share))
            } else {
                let (prefix, _) = parse_next_component(path, true);

                // in verbatim paths only recognize an exact drive prefix
                if let Some(drive) = parse_drive_exact(prefix) {
                    // \\?\C:
                    Some(VerbatimDisk(drive))
                } else {
                    // \\?\prefix
                    Some(Verbatim(prefix))
                }
            }
        } else if let Some(path) = strip_prefix(path, r".\") {
            // \\.\COM42
            let (prefix, _) = parse_next_component(path, false);
            Some(DeviceNS(prefix))
        } else {
            let (server, path) = parse_next_component(path, false);
            let (share, _) = parse_next_component(path, false);

            if !server.is_empty() && !share.is_empty() {
                // \\server\share
                Some(UNC(server, share))
            } else {
                // no valid prefix beginning with "\\" recognized
                None
            }
        }
    } else if let Some(drive) = parse_drive(path) {
        // C:
        Some(Disk(drive))
    } else {
        // no prefix
        None
    }
}

// Parses a drive prefix, e.g. "C:" and "C:\whatever"
fn parse_drive(prefix: &OsStr) -> Option<u8> {
    // In most DOS systems, it is not possible to have more than 26 drive letters.
    // See <https://en.wikipedia.org/wiki/Drive_letter_assignment#Common_assignments>.
    fn is_valid_drive_letter(drive: &u8) -> bool {
        drive.is_ascii_alphabetic()
    }

    match prefix.bytes() {
        [drive, b':', ..] if is_valid_drive_letter(drive) => Some(drive.to_ascii_uppercase()),
        _ => None,
    }
}

// Parses a drive prefix exactly, e.g. "C:"
fn parse_drive_exact(prefix: &OsStr) -> Option<u8> {
    // only parse two bytes: the drive letter and the drive separator
    if prefix.len() == 2 { parse_drive(prefix) } else { None }
}

fn strip_prefix<'a>(path: &'a OsStr, prefix: &str) -> Option<&'a OsStr> {
    // `path` and `prefix` are valid wtf8 and utf8 encoded slices respectively, `path[prefix.len()]`
    // is thus a code point boundary and `path[prefix.len()..]` is a valid wtf8 encoded slice.
    match path.bytes().strip_prefix(prefix.as_bytes()) {
        Some(path) => unsafe { Some(bytes_as_os_str(path)) },
        None => None,
    }
}

// Parse the next path component.
//
// Returns the next component and the rest of the path excluding the component and separator.
// Does not recognize `/` as a separator character if `verbatim` is true.
fn parse_next_component(path: &OsStr, verbatim: bool) -> (&OsStr, &OsStr) {
    let separator = if verbatim { is_verbatim_sep } else { is_sep_byte };

    match path.bytes().iter().position(|&x| separator(x)) {
        Some(separator_start) => {
            let mut separator_end = separator_start + 1;

            // a series of multiple separator characters is treated as a single separator,
            // except in verbatim paths
            while !verbatim && separator_end < path.len() && separator(path.bytes()[separator_end])
            {
                separator_end += 1;
            }

            let component = &path.bytes()[..separator_start];

            // Panic safe
            // The max `separator_end` is `bytes.len()` and `bytes[bytes.len()..]` is a valid index.
            let path = &path.bytes()[separator_end..];

            // SAFETY: `path` is a valid wtf8 encoded slice and each of the separators ('/', '\')
            // is encoded in a single byte, therefore `bytes[separator_start]` and
            // `bytes[separator_end]` must be code point boundaries and thus
            // `bytes[..separator_start]` and `bytes[separator_end..]` are valid wtf8 slices.
            unsafe { (bytes_as_os_str(component), bytes_as_os_str(path)) }
        }
        None => (path, OsStr::new("")),
    }
}

/// Returns a UTF-16 encoded path capable of bypassing the legacy `MAX_PATH` limits.
///
/// This path may or may not have a verbatim prefix.
pub(crate) fn maybe_verbatim(path: &Path) -> io::Result<Vec<u16>> {
    // Normally the MAX_PATH is 260 UTF-16 code units (including the NULL).
    // However, for APIs such as CreateDirectory[1], the limit is 248.
    //
    // [1]: https://docs.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createdirectorya#parameters
    const LEGACY_MAX_PATH: usize = 248;
    // UTF-16 encoded code points, used in parsing and building UTF-16 paths.
    // All of these are in the ASCII range so they can be cast directly to `u16`.
    const SEP: u16 = b'\\' as _;
    const ALT_SEP: u16 = b'/' as _;
    const QUERY: u16 = b'?' as _;
    const COLON: u16 = b':' as _;
    const DOT: u16 = b'.' as _;
    const U: u16 = b'U' as _;
    const N: u16 = b'N' as _;
    const C: u16 = b'C' as _;

    // \\?\
    const VERBATIM_PREFIX: &[u16] = &[SEP, SEP, QUERY, SEP];
    // \??\
    const NT_PREFIX: &[u16] = &[SEP, QUERY, QUERY, SEP];
    // \\?\UNC\
    const UNC_PREFIX: &[u16] = &[SEP, SEP, QUERY, SEP, U, N, C, SEP];

    let mut path = to_u16s(path)?;
    if path.starts_with(VERBATIM_PREFIX) || path.starts_with(NT_PREFIX) {
        // Early return for paths that are already verbatim.
        return Ok(path);
    } else if path.len() < LEGACY_MAX_PATH {
        // Early return if an absolute path is less < 260 UTF-16 code units.
        // This is an optimization to avoid calling `GetFullPathNameW` unnecessarily.
        match path.as_slice() {
            // Starts with `D:`, `D:\`, `D:/`, etc.
            // Does not match if the path starts with a `\` or `/`.
            [drive, COLON, 0] | [drive, COLON, SEP | ALT_SEP, ..]
                if *drive != SEP && *drive != ALT_SEP =>
            {
                return Ok(path);
            }
            // Starts with `\\`, `//`, etc
            [SEP | ALT_SEP, SEP | ALT_SEP, ..] => return Ok(path),
            _ => {}
        }
    }

    // Firstly, get the absolute path using `GetFullPathNameW`.
    // https://docs.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getfullpathnamew
    let lpfilename = path.as_ptr();
    fill_utf16_buf(
        // SAFETY: `fill_utf16_buf` ensures the `buffer` and `size` are valid.
        // `lpfilename` is a pointer to a null terminated string that is not
        // invalidated until after `GetFullPathNameW` returns successfully.
        |buffer, size| unsafe {
            // While the docs for `GetFullPathNameW` have the standard note
            // about needing a `\\?\` path for a long lpfilename, this does not
            // appear to be true in practice.
            // See:
            // https://stackoverflow.com/questions/38036943/getfullpathnamew-and-long-windows-file-paths
            // https://googleprojectzero.blogspot.com/2016/02/the-definitive-guide-on-win32-to-nt.html
            c::GetFullPathNameW(lpfilename, size, buffer, ptr::null_mut())
        },
        |mut absolute| {
            path.clear();

            // Secondly, add the verbatim prefix. This is easier here because we know the
            // path is now absolute and fully normalized (e.g. `/` has been changed to `\`).
            let prefix = match absolute {
                // C:\ => \\?\C:\
                [_, COLON, SEP, ..] => VERBATIM_PREFIX,
                // \\.\ => \\?\
                [SEP, SEP, DOT, SEP, ..] => {
                    absolute = &absolute[4..];
                    VERBATIM_PREFIX
                }
                // Leave \\?\ and \??\ as-is.
                [SEP, SEP, QUERY, SEP, ..] | [SEP, QUERY, QUERY, SEP, ..] => &[],
                // \\ => \\?\UNC\
                [SEP, SEP, ..] => {
                    absolute = &absolute[2..];
                    UNC_PREFIX
                }
                // Anything else we leave alone.
                _ => &[],
            };

            path.reserve_exact(prefix.len() + absolute.len() + 1);
            path.extend_from_slice(prefix);
            path.extend_from_slice(absolute);
            path.push(0);
        },
    )?;
    Ok(path)
}
