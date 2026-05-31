//! CRC16 used by the OpenBlink Program command.
//!
//! The device computes a reflected CRC16 with polynomial `0xD175` and seed
//! `0xFFFF`. We compute the identical checksum with the `crc` crate. For a
//! reflected algorithm (`refin`/`refout` = true) the crate expects the
//! polynomial in MSB-first form, which is the bit reversal of `0xD175`,
//! i.e. `0xAE8B`.

use crc::{Algorithm, Crc};

/// Clean-room reference implementation of the documented right-shifting
/// reflected CRC16. Used to derive the `check` constant at compile time and to
/// validate the `crc` crate configuration in tests.
const fn reference_crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    let mut i = 0;
    while i < data.len() {
        crc ^= data[i] as u16;
        let mut j = 0;
        while j < 8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xD175;
            } else {
                crc >>= 1;
            }
            j += 1;
        }
        i += 1;
    }
    crc
}

/// CRC16 parameters matching the OpenBlink firmware.
pub const OPENBLINK_CRC16: Algorithm<u16> = Algorithm {
    width: 16,
    poly: 0xAE8B, // bit reversal of the reflected polynomial 0xD175
    init: 0xFFFF,
    refin: true,
    refout: true,
    xorout: 0x0000,
    check: reference_crc16(b"123456789"),
    residue: 0x0000,
};

const CRC: Crc<u16> = Crc::<u16>::new(&OPENBLINK_CRC16);

/// Computes the OpenBlink CRC16 of `data`.
pub fn crc16(data: &[u8]) -> u16 {
    CRC.checksum(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_reference_on_known_vectors() {
        for v in [&b""[..], &b"\x01"[..], &b"123456789"[..], &b"RITE0300"[..]] {
            assert_eq!(crc16(v), reference_crc16(v), "mismatch for {v:?}");
        }
    }

    #[test]
    fn golden_check_value() {
        // Computed independently with the documented algorithm (poly 0xD175).
        assert_eq!(crc16(b"123456789"), 0x97DE);
        assert_eq!(OPENBLINK_CRC16.check, 0x97DE);
    }

    #[test]
    fn empty_input_returns_seed() {
        assert_eq!(crc16(b""), 0xFFFF);
    }

    #[test]
    fn matches_reference_on_pseudorandom_inputs() {
        // Deterministic LCG so the test is reproducible.
        let mut state: u32 = 0x1234_5678;
        let mut next = || {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (state >> 24) as u8
        };
        for len in 0..512usize {
            let data: Vec<u8> = (0..len).map(|_| next()).collect();
            assert_eq!(
                crc16(&data),
                reference_crc16(&data),
                "mismatch at len {len}"
            );
        }
    }
}
