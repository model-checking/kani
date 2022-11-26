// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Represent information about an artifact type.

use std::ffi::OsStr;
use std::ops::Deref;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ArtifactType {
    Goto,
    Metadata,
    SymTab,
    SymTabGoto,
    TypeMap,
    VTableRestriction,
}

impl ArtifactType {
    const fn extension(&self) -> &'static str {
        match self {
            ArtifactType::Goto => "out",
            ArtifactType::Metadata => "kani-metadata.json",
            ArtifactType::SymTab => "symtab.json",
            ArtifactType::SymTabGoto => "symtab.out",
            ArtifactType::TypeMap => "type_map.json",
            ArtifactType::VTableRestriction => "restrictions.json",
        }
    }
}

/// Create a new path by removing the initial extension and attaching a new one.
/// E.g.:
/// ```
/// # use std::path::PathBuf;
/// # use kani_metadata::{ArtifactType, convert_type};
/// let path = PathBuf::from("my_file.symtab.out");
/// let goto = convert_type(&path, ArtifactType::SymTabGoto, ArtifactType::Goto);
/// assert_eq!(goto.as_os_str(), "my_file.out");
/// ```
pub fn convert_type(path: &Path, from: ArtifactType, to: ArtifactType) -> PathBuf {
    let mut result = path.to_path_buf();
    // Strip current extensions and replace by the new one.
    match from {
        // Artifact types that has only one extension.
        ArtifactType::Goto => {
            result.set_extension(&to);
        }
        // Artifact types that has two extensions.
        ArtifactType::Metadata
        | ArtifactType::SymTab
        | ArtifactType::SymTabGoto
        | ArtifactType::TypeMap
        | ArtifactType::VTableRestriction => {
            result.set_extension("");
            result.set_extension(&to);
        }
    }
    result
}

impl AsRef<str> for ArtifactType {
    fn as_ref(&self) -> &str {
        self.extension()
    }
}

impl AsRef<OsStr> for ArtifactType {
    fn as_ref(&self) -> &OsStr {
        self.extension().as_ref()
    }
}

impl Deref for ArtifactType {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.extension()
    }
}

#[cfg(test)]
mod test {
    use super::{convert_type, ArtifactType::*};
    use std::path::PathBuf;

    #[test]
    fn test_convert_ok() {
        let path = PathBuf::from("/tmp/my_file.rs").with_extension(&SymTabGoto);
        let goto = convert_type(&path, SymTabGoto, Goto);
        assert_eq!(goto.as_os_str(), "/tmp/my_file.out");

        let orig = convert_type(&goto, Goto, SymTabGoto);
        assert_eq!(orig, path);
    }

    #[test]
    fn test_set_extension_ok() {
        let path = PathBuf::from("/tmp/my_file.rs").with_extension(&SymTabGoto);
        assert_eq!(path.as_os_str(), "/tmp/my_file.symtab.out");
    }
}
