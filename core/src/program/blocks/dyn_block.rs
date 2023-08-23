use super::{fmt, hasher, Digest, Felt, Operation};

// Dyn BLOCK
// ================================================================================================
/// Block for a dynamic function where the target is specified by the stack.
///
/// Executes the function referenced by the hash on top of the stack. Fails if the body is
/// unavailable to the VM, or if the execution of the dynamically-specified function fails.
///
/// The hash of a dyn block is computed as:
///
/// > hash(DYN_CONSTANT || padding, domain=CALL_DOMAIN)
///
/// Where `fn_hash` is 4 field elements (256 bits), and `padding` is 4 ZERO elements (256 bits).
/// TODO: check on this hashing. Does it make sense?
#[derive(Clone, Debug)]
pub struct Dyn {
    hash: Digest,
}

impl Dyn {
    // CONSTANTS
    // --------------------------------------------------------------------------------------------
    /// The domain of the Dyn block (used for control block hashing).
    pub const DOMAIN: Felt = Felt::new(Operation::Dyn.op_code() as u64);

    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Returns a new [Dyn] block instantiated with the specified function body hash.
    pub fn new() -> Self {
        let hash = hasher::merge_in_domain(&[Digest::default(), Digest::default()], Self::DOMAIN);
        Self { hash }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns a hash of this code block.
    pub fn hash(&self) -> Digest {
        self.hash
    }
}

impl Default for Dyn {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Dyn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Dyn")?;

        Ok(())
    }
}
