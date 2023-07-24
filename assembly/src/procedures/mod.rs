use super::{
    crypto::hash::{Blake3_160, RpoDigest},
    BTreeSet, ByteReader, ByteWriter, CodeBlock, Deserializable, DeserializationError, LabelError,
    LibraryPath, Serializable, String, ToString, PROCEDURE_LABEL_PARSER,
};
use crate::{ast::ProcedureScope, LibraryNamespace};
use core::{
    fmt,
    ops::{self, Deref},
    str::from_utf8,
};

// PROCEDURE
// ================================================================================================

#[derive(Clone, Debug)]
pub enum MaterializedProcedureScope {
    Internal(LibraryPath),
    Exported,
    Library(LibraryNamespace),
}

impl MaterializedProcedureScope {
    pub fn from_path(scope: ProcedureScope, path: LibraryPath) -> Self {
        match scope {
            ProcedureScope::INTERNAL => Self::Internal(path),
            ProcedureScope::EXPORT => Self::Exported,
            ProcedureScope::LIBRARY => Self::Library(
                LibraryNamespace::new(path.first()).expect("path must be well formed"),
            ),
        }
    }

    pub fn is_export(&self) -> bool {
        matches!(
            self,
            MaterializedProcedureScope::Exported | MaterializedProcedureScope::Library(_)
        )
    }
}

impl MaterializedProcedureScope {
    pub fn is_valid_invocation(&self, invocation_scope: &LibraryPath) -> bool {
        match self {
            MaterializedProcedureScope::Internal(scope) => scope == invocation_scope,
            MaterializedProcedureScope::Exported => true,
            MaterializedProcedureScope::Library(namespace) => {
                invocation_scope.first() == namespace.as_str()
            }
        }
    }
}

/// Miden assembly procedure consisting of procedure MAST and basic metadata.
///
/// Procedure metadata includes:
/// - Number of procedure locals available to the procedure.
/// - A set of MAST roots of procedures which are invoked from this procedure.
#[derive(Clone, Debug)]
pub struct Procedure {
    num_locals: u32,
    code: CodeBlock,
    callset: CallSet,
}

impl Procedure {
    /// Returns the number of memory locals reserved by the procedure.
    pub fn num_locals(&self) -> u32 {
        self.num_locals
    }

    /// Returns the root of this procedure's MAST.
    pub fn mast_root(&self) -> RpoDigest {
        self.code.hash()
    }

    /// Returns a reference to the MAST of this procedure.
    pub fn code(&self) -> &CodeBlock {
        &self.code
    }

    /// Returns a reference to a set of all procedures (identified by their MAST roots) which may
    /// be called during the execution of this procedure.
    pub fn callset(&self) -> &CallSet {
        &self.callset
    }
}

// NAMED PROCEDURE
// ================================================================================================

/// A named Miden assembly procedure consisting of procedure MAST and procedure metadata.
///
/// Procedure metadata includes:
/// - Procedure name.
/// - Procedure ID which is computed as a hash of the procedure's fully qualified path.
/// - A boolean flag indicating whether the procedure is exported from a module.
/// - Number of procedure locals available to the procedure.
/// - A set of MAST roots of procedures which are invoked from this procedure.
#[derive(Clone, Debug)]
pub struct NamedProcedure {
    id: ProcedureId,
    name: ProcedureName,
    scope: MaterializedProcedureScope,
    procedure: Procedure,
}

impl NamedProcedure {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Returns a new [Procedure] instantiated with the specified properties.
    pub fn new(
        id: ProcedureId,
        name: ProcedureName,
        scope: MaterializedProcedureScope,
        num_locals: u32,
        code: CodeBlock,
        callset: CallSet,
    ) -> Self {
        NamedProcedure {
            id,
            name,
            scope,
            procedure: Procedure {
                num_locals,
                code,
                callset,
            },
        }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns ID of this procedure.
    pub fn id(&self) -> &ProcedureId {
        &self.id
    }

    /// Returns a label of this procedure.
    pub fn name(&self) -> &ProcedureName {
        &self.name
    }

    pub fn scope(&self) -> &MaterializedProcedureScope {
        &self.scope
    }

    /// Returns `true` if this is an exported procedure.
    pub fn is_export(&self) -> bool {
        self.scope.is_export()
    }

    /// Returns the number of memory locals reserved by the procedure.
    pub fn num_locals(&self) -> u32 {
        self.procedure.num_locals
    }

    /// Returns the root of this procedure's MAST.
    pub fn mast_root(&self) -> RpoDigest {
        self.procedure.code.hash()
    }

    /// Returns a reference to the MAST of this procedure.
    pub fn code(&self) -> &CodeBlock {
        &self.procedure.code
    }

    /// Returns a reference to a set of all procedures (identified by their MAST roots) which may
    /// be called during the execution of this procedure.
    pub fn callset(&self) -> &CallSet {
        &self.procedure.callset
    }

    /// Returns the inner procedure containing all procedure attributes except for procedure name
    /// and ID.
    pub fn inner(&self) -> &Procedure {
        &self.procedure
    }

    /// Converts this procedure into its inner procedure containing all procedure attributes except
    /// for procedure name and ID.
    pub fn into_inner(self) -> (Procedure, MaterializedProcedureScope) {
        (self.procedure, self.scope)
    }
}

// PROCEDURE NAME
// ================================================================================================

/// Procedure name.
///
/// Procedure name must comply with the following rules:
/// - It cannot require more than 255 characters to serialize.
/// - It must start with a ASCII letter.
/// - It must consist of only ASCII letters, numbers, and underscores.
///
/// The only exception from the above rules is the name for the main procedure of an executable
/// module which is set to `#main`.
///
/// # Type-safety
/// Any instance of this type can be created only via the checked [`Self::try_from`].
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProcedureName {
    name: String,
}

impl ProcedureName {
    // CONSTANTS
    // --------------------------------------------------------------------------------------------

    /// Reserved name for a main procedure.
    pub const MAIN_PROC_NAME: &str = "#main";

    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Create a new procedure name with the reserved label for `main`.
    pub fn main() -> Self {
        Self {
            name: Self::MAIN_PROC_NAME.into(),
        }
    }

    // HELPERS
    // --------------------------------------------------------------------------------------------

    /// Check if the procedure label is the reserved name for `main`.
    pub fn is_main(&self) -> bool {
        self.name == Self::MAIN_PROC_NAME
    }
}

impl TryFrom<String> for ProcedureName {
    type Error = LabelError;

    fn try_from(name: String) -> Result<Self, Self::Error> {
        Self::try_from(name.as_ref())
    }
}

impl TryFrom<&str> for ProcedureName {
    type Error = LabelError;

    fn try_from(name: &str) -> Result<Self, Self::Error> {
        Ok(Self {
            name: (PROCEDURE_LABEL_PARSER.parse_label(name)?).to_string(),
        })
    }
}

impl Deref for ProcedureName {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.name
    }
}

impl AsRef<str> for ProcedureName {
    fn as_ref(&self) -> &str {
        &self.name
    }
}

impl Serializable for ProcedureName {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        debug_assert!(
            PROCEDURE_LABEL_PARSER.parse_label(&self.name).is_ok(),
            "The constructor should ensure the length is within limits"
        );

        target.write_u8(self.name.len() as u8);
        target.write_bytes(self.name.as_bytes());
    }
}

impl Deserializable for ProcedureName {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let nlen = source.read_u8()? as usize;
        let name = source.read_vec(nlen)?;
        let name =
            from_utf8(&name).map_err(|e| DeserializationError::InvalidValue(e.to_string()))?;
        ProcedureName::try_from(name.to_string())
            .map_err(|e| DeserializationError::InvalidValue(e.to_string()))
    }
}

impl fmt::Display for ProcedureName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

// PROCEDURE ID
// ================================================================================================

/// A procedure identifier computed as a hash of a fully qualified procedure path.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProcedureId(pub [u8; Self::SIZE]);

impl ProcedureId {
    /// Truncated length of the id
    pub const SIZE: usize = 20;

    /// Creates a new procedure id from its path, composed by module path + name identifier.
    ///
    /// No validation is performed regarding the consistency of the path format.
    pub fn new<L>(path: L) -> Self
    where
        L: AsRef<str>,
    {
        let mut digest = [0u8; Self::SIZE];
        let hash = Blake3_160::hash(path.as_ref().as_bytes());
        digest.copy_from_slice(&(*hash)[..Self::SIZE]);
        Self(digest)
    }

    /// Creates a new procedure ID from a name to be resolved in the kernel.
    pub fn from_kernel_name(name: &str) -> Self {
        let path = LibraryPath::kernel_path().append_unchecked(name);
        Self::new(path)
    }

    /// Creates a new procedure ID from its name and module path.
    ///
    /// No validation is performed regarding the consistency of the module path or procedure name
    /// format.
    pub fn from_name(name: &str, module_path: &LibraryPath) -> Self {
        let path = module_path.append_unchecked(name);
        Self::new(path)
    }

    /// Creates a new procedure ID from its local index and module path.
    ///
    /// No validation is performed regarding the consistency of the module path format.
    pub fn from_index(index: u16, module_path: &LibraryPath) -> Self {
        let path = module_path.append_unchecked(index.to_string());
        Self::new(path)
    }
}

impl From<[u8; ProcedureId::SIZE]> for ProcedureId {
    fn from(value: [u8; ProcedureId::SIZE]) -> Self {
        Self(value)
    }
}

impl From<&LibraryPath> for ProcedureId {
    fn from(path: &LibraryPath) -> Self {
        ProcedureId::new(path)
    }
}

impl ops::Deref for ProcedureId {
    type Target = [u8; Self::SIZE];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for ProcedureId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x")?;
        for byte in self.0.iter() {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl Serializable for ProcedureId {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        target.write_bytes(&self.0);
    }
}

impl Deserializable for ProcedureId {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let proc_id = source.read_array::<{ Self::SIZE }>()?;
        Ok(Self(proc_id))
    }
}

// CALLSET
// ================================================================================================

/// Contains a list of all procedures which may be invoked from a procedure via call or syscall
/// instructions.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CallSet(BTreeSet<RpoDigest>);

impl CallSet {
    pub fn contains(&self, mast_root: &RpoDigest) -> bool {
        self.0.contains(mast_root)
    }

    pub fn insert(&mut self, mast_root: RpoDigest) {
        self.0.insert(mast_root);
    }

    pub fn append(&mut self, other: &CallSet) {
        for &item in other.0.iter() {
            self.0.insert(item);
        }
    }
}

impl ops::Deref for CallSet {
    type Target = BTreeSet<RpoDigest>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
mod test {
    use super::{super::MAX_LABEL_LEN, LabelError, ProcedureName};

    #[test]
    fn test_procedure_name_max_len() {
        assert!(ProcedureName::try_from("a".to_owned()).is_ok());

        let long = "a".repeat(256);
        assert_eq!(
            ProcedureName::try_from(long.clone()),
            Err(LabelError::LabelTooLong(long, MAX_LABEL_LEN))
        );
    }
}
