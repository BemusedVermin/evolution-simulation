//! Positional per-channel contribution vector.
//!
//! [`ChannelVector`] is a thin newtype over `Vec<Q3232>` whose sole job is to
//! flag, in the type system, that a bare [`Vec<Q3232>`] of channel
//! contributions is positionally tied to a specific channel-registry
//! vocabulary. The index-to-channel mapping is recovered at runtime from a
//! [`beast_channels::ChannelRegistry`] (via its sorted iteration order), and
//! the integrity of that mapping is enforced at load time by the
//! [`beast_channels::RegistryFingerprint`] stored in the save envelope (see
//! [`crate::save`]).
//!
//! The newtype deserializes **transparently**: on-disk JSON for an
//! [`crate::gene::EffectVector`] still has a plain `[Q3232, Q3232, ...]`
//! array at `channel`, so saves written before this newtype existed still
//! round-trip cleanly. A regression test below pins that behaviour.

use beast_core::Q3232;
use serde::{Deserialize, Serialize};

/// Positional per-channel contribution vector.
///
/// Indexed by sorted channel id from the loaded
/// [`beast_channels::ChannelRegistry`]. Callers should treat the index as an
/// opaque handle; the only legitimate way to recover the channel id is to
/// iterate the registry in parallel.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ChannelVector(Vec<Q3232>);

impl ChannelVector {
    /// Construct from a raw `Vec<Q3232>`. The caller is responsible for the
    /// vector being indexed against the *current* registry; mismatches are
    /// caught at save-load boundaries, not here.
    #[inline]
    #[must_use]
    pub fn new(values: Vec<Q3232>) -> Self {
        Self(values)
    }

    /// Number of channels.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether there are zero channels.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Iterate in index order.
    #[inline]
    pub fn iter(&self) -> core::slice::Iter<'_, Q3232> {
        self.0.iter()
    }

    /// Iterate mutably in index order.
    #[inline]
    pub fn iter_mut(&mut self) -> core::slice::IterMut<'_, Q3232> {
        self.0.iter_mut()
    }

    /// Borrow as a slice.
    #[inline]
    #[must_use]
    pub fn as_slice(&self) -> &[Q3232] {
        &self.0
    }

    /// Borrow mutably as a slice.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [Q3232] {
        &mut self.0
    }

    /// Consume and return the underlying `Vec<Q3232>`.
    #[inline]
    #[must_use]
    pub fn into_inner(self) -> Vec<Q3232> {
        self.0
    }
}

impl From<Vec<Q3232>> for ChannelVector {
    #[inline]
    fn from(values: Vec<Q3232>) -> Self {
        Self(values)
    }
}

impl From<ChannelVector> for Vec<Q3232> {
    #[inline]
    fn from(vector: ChannelVector) -> Self {
        vector.0
    }
}

impl AsRef<[Q3232]> for ChannelVector {
    #[inline]
    fn as_ref(&self) -> &[Q3232] {
        &self.0
    }
}

impl AsMut<[Q3232]> for ChannelVector {
    #[inline]
    fn as_mut(&mut self) -> &mut [Q3232] {
        &mut self.0
    }
}

impl<'a> IntoIterator for &'a ChannelVector {
    type Item = &'a Q3232;
    type IntoIter = core::slice::Iter<'a, Q3232>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a mut ChannelVector {
    type Item = &'a mut Q3232;
    type IntoIter = core::slice::IterMut<'a, Q3232>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ChannelVector {
        ChannelVector::from(vec![
            Q3232::from_num(0.25_f64),
            -Q3232::from_num(0.5_f64),
            Q3232::from_num(1.5_f64),
        ])
    }

    #[test]
    fn from_vec_preserves_contents() {
        let raw = vec![Q3232::from_num(0.1_f64), Q3232::from_num(0.2_f64)];
        let cv = ChannelVector::from(raw.clone());
        assert_eq!(cv.as_slice(), raw.as_slice());
    }

    #[test]
    fn iter_yields_values_in_order() {
        let cv = sample();
        let collected: Vec<Q3232> = (&cv).into_iter().copied().collect();
        assert_eq!(collected, cv.as_slice());
    }

    #[test]
    fn iter_mut_permits_in_place_updates() {
        let mut cv = sample();
        for v in &mut cv {
            *v = *v + Q3232::from_num(1_i32);
        }
        assert_eq!(
            cv.as_slice(),
            &[
                Q3232::from_num(1.25_f64),
                Q3232::from_num(0.5_f64),
                Q3232::from_num(2.5_f64),
            ]
        );
    }

    #[test]
    fn serializes_as_bare_array() {
        // #[serde(transparent)] keeps the JSON byte-identical to a bare Vec<Q3232>.
        let cv = sample();
        let raw_vec: Vec<Q3232> = cv.clone().into_inner();
        let cv_json = serde_json::to_string(&cv).unwrap();
        let vec_json = serde_json::to_string(&raw_vec).unwrap();
        assert_eq!(cv_json, vec_json);
    }

    #[test]
    fn deserializes_same_json_as_a_vec() {
        // Old save files stored the channel vector as a bare JSON array of
        // Q3232 values (whatever Q3232's serde shape is — we inherit it
        // transparently). Deserializing a `Vec<Q3232>`-shaped payload into a
        // `ChannelVector` must succeed without any wrapper object.
        let raw: Vec<Q3232> = vec![
            Q3232::from_num(0.25_f64),
            -Q3232::from_num(0.5_f64),
            Q3232::from_num(1.5_f64),
        ];
        let json = serde_json::to_string(&raw).unwrap();
        let cv: ChannelVector = serde_json::from_str(&json).unwrap();
        assert_eq!(cv.as_slice(), raw.as_slice());
    }

    #[test]
    fn round_trips_through_json() {
        let cv = sample();
        let json = serde_json::to_string(&cv).unwrap();
        let back: ChannelVector = serde_json::from_str(&json).unwrap();
        assert_eq!(cv, back);
    }
}
