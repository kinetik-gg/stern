//! Stable icon identifiers with injectable collision testing.

use std::collections::BTreeMap;

use crate::{Error, ErrorKind, Result, Weight};

/// Deterministic 64-bit identity for a canonical icon and weight.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct StableId(pub u64);

/// Assigns stable FNV-1a identifiers.
///
/// # Errors
///
/// Returns [`ErrorKind::IdCollision`](crate::ErrorKind::IdCollision) if two
/// definitions hash to the same identifier.
pub fn assign_stable_ids<'a>(
    definitions: impl IntoIterator<Item = (&'a str, Weight)>,
) -> Result<BTreeMap<(String, Weight), StableId>> {
    assign_stable_ids_with(definitions, fnv1a)
}

/// Assigns identifiers using an injectable hash function and rejects collisions.
///
/// # Errors
///
/// Returns [`ErrorKind::IdCollision`](crate::ErrorKind::IdCollision) if two
/// definitions hash to the same identifier.
pub fn assign_stable_ids_with<'a, F>(
    definitions: impl IntoIterator<Item = (&'a str, Weight)>,
    hash: F,
) -> Result<BTreeMap<(String, Weight), StableId>>
where
    F: Fn(&[u8]) -> u64,
{
    let mut by_id = BTreeMap::<StableId, (String, Weight)>::new();
    let mut result = BTreeMap::new();
    for (name, weight) in definitions {
        let identity = format!("phosphor:2.1.1:{name}:{}", weight.as_str());
        let id = StableId(hash(identity.as_bytes()));
        if let Some((previous_name, previous_weight)) = by_id.insert(id, (name.to_owned(), weight))
        {
            return Err(Error::new(
                ErrorKind::IdCollision,
                format!("{:#018x}", id.0),
                format!(
                    "`{previous_name}`/{} collides with `{name}`/{}",
                    previous_weight.as_str(),
                    weight.as_str()
                ),
            ));
        }
        result.insert((name.to_owned(), weight), id);
    }
    Ok(result)
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}
