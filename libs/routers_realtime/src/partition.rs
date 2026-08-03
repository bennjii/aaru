//! The fleet-wide stable hashing contract.
//!
//! Everything here is wire law, not implementation detail: producers in any
//! language derive a vehicle's partition, and every binary in the fleet must
//! agree with them bit-for-bit, across processes, releases, and rewrites.
//! Nothing in this module may change without a coordinated id-space
//! migration.

use crate::event::VehicleId;

/// How many partitions the vehicle id space divides into. Fixed: subjects,
/// consumer names, and partition-to-pod assignment are all derived from it.
pub const PARTITIONS: u64 = 1024;

/// FNV-1a 64. Stable by construction, which `DefaultHasher` is not: its
/// algorithm may change between Rust releases, and the fleet has to agree
/// on every placement this feeds.
pub fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// splitmix64 finaliser. FNV-1a avalanches poorly on its own — worst in the
/// low bits, which are exactly what a modulo reads — so every consumer of a
/// stable hash mixes before comparing or reducing.
pub fn mix(mut x: u64) -> u64 {
    x ^= x >> 30;
    x = x.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

/// The vehicle's partition: `splitmix64(id) % PARTITIONS`.
///
/// This is the cross-language producer contract. The id itself is already a
/// hash (FNV-1a 64 of the upstream string; see `realtime/v1/event.proto`),
/// so one mix pass repairs its low-bit weakness and the modulo is safe.
pub fn partition_of(vehicle: VehicleId) -> u64 {
    mix(vehicle.0) % PARTITIONS
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Externally computed vectors. These pin the algorithm itself: a
    /// transcription slip in either constant still spreads keys evenly, and
    /// only a known answer catches it. `fnv1a(b"a")` is the published FNV-1a
    /// 64 test vector.
    #[test]
    fn matches_reference_vectors() {
        assert_eq!(fnv1a(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a(b"vehicle-42"), 0xf4dc_ea25_6ede_2c6c);

        assert_eq!(mix(0), 0);
        assert_eq!(mix(1), 0x5692_161d_100b_05e5);
        assert_eq!(mix(0xdead_beef), 0x4e06_2702_ec92_9eea);
        assert_eq!(mix(u64::MAX), 0xb4d0_55fc_f2cb_bd7b);

        assert_eq!(partition_of(VehicleId(1)), 485);
        assert_eq!(partition_of(VehicleId(0xdead_beef)), 746);
        assert_eq!(partition_of(VehicleId(u64::MAX)), 379);
    }

    /// Sequential ids are the pathological case for a bare modulo: without
    /// the mix they would fill partitions 0..n and leave the rest empty.
    #[test]
    fn sequential_ids_spread_across_partitions() {
        let per_partition = 100;
        let mut counts = vec![0usize; PARTITIONS as usize];

        for id in 0..(PARTITIONS * per_partition) {
            counts[partition_of(VehicleId(id)) as usize] += 1;
        }

        for (partition, count) in counts.iter().enumerate() {
            assert!(
                (25..400).contains(count),
                "partition {partition} took {count} of an expected ~{per_partition}"
            );
        }
    }
}
