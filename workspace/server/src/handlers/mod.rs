mod package;
mod publisher;

pub(crate) use package::PackageHandler;
pub(crate) use publisher::PublisherHandler;

use crate::Result;
use k256::ecdsa::recoverable;
use web3_address::ethereum::Address;

/// Verify a signature against a message and return the address.
pub(crate) fn verify_signature(
    signature: [u8; 65],
    message: &[u8],
) -> Result<Address> {
    let recoverable: recoverable::Signature =
        signature.as_slice().try_into()?;
    let public_key = recoverable.recover_verifying_key(message)?;
    let public_key: [u8; 33] = public_key.to_bytes().as_slice().try_into()?;
    let address: Address = (&public_key).try_into()?;
    Ok(address)
}
