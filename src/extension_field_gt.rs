use crate::types::GroupGT;

use super::ECCurve::fp12::{FP12, DENSE};
use super::ECCurve::fp4::FP4;
use super::ECCurve::pair::{another, ate, ate2, fexp, initmp, miller};
use crate::field_elem::FieldElement;
use crate::group_elem::GroupElement;
use crate::group_elem_g1::G1;
use crate::group_elem_g2::{G2, parse_hex_as_FP2};
use std::fmt;
use std::hash::{Hash, Hasher};
use crate::errors::SerzDeserzError;
use crate::constants::GroupGT_SIZE;

use serde::de::{Deserialize, Deserializer, Error as DError, Visitor};
use serde::ser::{Error as SError, Serialize, Serializer};
use std::str::{FromStr, SplitWhitespace};
use zeroize::Zeroize;

#[derive(Clone)]
pub struct GT {
    value: GroupGT,
}

impl fmt::Debug for GT {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut c = self.value.clone();
        write!(f, "{}", c.tostring())
    }
}

impl GT {
    pub fn new() -> Self {
        Self {
            value: GroupGT::new(),
        }
    }

    /// Reduced ate pairing. Returns `e(g1, g2)`
    pub fn ate_pairing(g1: &G1, g2: &G2) -> Self {
        // This check is temporary. Until amcl is fixed.
        if g1.is_identity() || g2.is_identity() {
            return Self::one();
        }
        let e = ate(&g2.to_ecp(), &g1.to_ecp());
        Self { value: fexp(&e) }
    }

    /// Reduced ate double pairing. Returns `e(g1, g2) * e(h1, h2)`
    pub fn ate_2_pairing(g1: &G1, g2: &G2, h1: &G1, h2: &G2) -> Self {
        // This check is temporary. Until amcl is fixed.
        if g1.is_identity() || g2.is_identity() {
            return Self::ate_pairing(h1, h2);
        }
        if h1.is_identity() || h2.is_identity() {
            return Self::ate_pairing(g1, g2);
        }
        let e = ate2(&g2.to_ecp(), &g1.to_ecp(), &h2.to_ecp(), &h1.to_ecp());
        Self { value: fexp(&e) }
    }

    /// Reduced ate multi pairing. Takes a vector of tuples of group elements G1 and G2 as Vec<(&G1, &G2)>.
    /// Returns the product of their pairings.
    /// More efficient than using ate_pairing or ate_2_pairing and multiplying results
    pub fn ate_multi_pairing(elems: Vec<(&G1, &G2)>) -> Self {
        let mut accum = initmp();
        for (g1, g2) in elems {
            if g1.is_identity() || g2.is_identity() {
                continue;
            }
            another(&mut accum, &g2.to_ecp(), &g1.to_ecp());
        }
        let e = miller(&accum);
        Self { value: fexp(&e) }
    }

    pub fn mul(a: &Self, b: &Self) -> Self {
        let mut m = FP12::new_copy(&a.value);
        m.mul(&b.value);
        Self { value: m }
    }

    pub fn pow(&self, e: &FieldElement) -> Self {
        Self {
            value: self.value.pow(&e.to_bignum()),
        }
    }

    /// Return inverse of itself
    pub fn inverse(&self) -> Self {
        let mut inv = self.value.clone();
        inv.inverse();
        Self { value: inv }
    }

    pub fn inverse_mut(&mut self) {
        self.value.inverse()
    }

    pub fn is_one(&self) -> bool {
        return self.value.isunity();
    }

    pub fn one() -> Self {
        let zero = FP4::new_int(0);
        let one = FP4::new_int(1);
        Self {
            value: FP12::new_fp4s(&one, &zero, &zero),
        }
    }

    pub fn to_fp12(&self) -> FP12 {
        self.value.clone()
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut temp = FP12::new();
        temp.copy(&self.value);
        let mut bytes: [u8; GroupGT_SIZE] = [0; GroupGT_SIZE];
        temp.tobytes(&mut bytes);
        bytes.to_vec()
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SerzDeserzError> {
        if bytes.len() != GroupGT_SIZE {
            return Err(SerzDeserzError::GTBytesIncorrectSize(
                bytes.len(),
                GroupGT_SIZE,
            ));
        }
        Ok(Self {
            value: FP12::frombytes(bytes)
        })
    }

    pub fn to_hex(&self) -> String {
        self.value.to_hex()
    }

    pub fn from_hex(s: String) -> Result<Self, SerzDeserzError> {
        let mut iter = s.split_whitespace();
        let a = parse_hex_as_FP4(&mut iter)?;
        let b = parse_hex_as_FP4(&mut iter)?;
        let c = parse_hex_as_FP4(&mut iter)?;
        let mut value = FP12::new();
        value.seta(a);
        value.setb(b);
        value.setc(c);
        value.settype(DENSE);
        Ok(Self { value })
    }

    /// Return a random group element. Only for testing.
    #[cfg(test)]
    pub fn random() -> Self {
        let g1 = G1::random();
        let g2 = G2::random();
        GT::ate_pairing(&g1, &g2)
    }
}

/// Parse given hex string as FP4
pub fn parse_hex_as_FP4(iter: &mut SplitWhitespace) -> Result<FP4, SerzDeserzError> {
    // Logic almost copied from AMCL but with error handling and constant time execution.
    // Constant time is important as hex is used during serialization and deserialization.
    // A seemingly effortless solution is to filter string for errors and pad with 0s before
    // passing to AMCL but that would be expensive as the string is scanned twice
    let a = parse_hex_as_FP2(iter)?;
    let b = parse_hex_as_FP2(iter)?;
    let mut fp4 = FP4::new();
    fp4.seta(a);
    fp4.setb(b);
    Ok(fp4)
}

impl PartialEq for GT {
    fn eq(&self, other: &GT) -> bool {
        self.value.equals(&other.value)
    }
}

impl_group_elem_conversions!(GT, GroupGT, GroupGT_SIZE);

impl_group_elem_traits!(GT, GroupGT);

#[cfg(test)]
mod test {
    use super::*;
    use crate::group_elem_g1::G1Vector;
    use std::time::{Duration, Instant};

    #[test]
    fn test_unity() {
        let one = GT::one();
        assert!(one.is_one());
    }

    #[test]
    fn test_inverse() {
        let minus_one = FieldElement::minus_one();
        for _ in 0..10 {
            let g1 = G1::random();
            let g2 = G2::random();
            let e = GT::ate_pairing(&g1, &g2);
            let e_inv = e.inverse();
            assert!(GT::mul(&e, &e_inv).is_one());

            assert_eq!(GT::pow(&e, &minus_one), e_inv);
            assert_eq!(GT::pow(&e_inv, &minus_one), e);
        }
    }

    #[test]
    fn test_ate_pairing_identity() {
        let g1 = G1::random();
        let g2 = G2::random();
        let g1_identity = G1::identity();
        let g2_identity = G2::identity();

        // e(g1 + identity, g2) == e(g1, g2)*e(identity, g2)
        let lhs = GT::ate_pairing(&(&g1 + &g1_identity), &g2);
        let rhs = GT::mul(
            &GT::ate_pairing(&g1, &g2),
            &GT::ate_pairing(&g1_identity, &g2),
        );
        assert!(lhs == rhs);

        // e(g1, g2 + identity) == e(g1, g2)*e(g1, identity)
        let lhs = GT::ate_pairing(&g1, &(&g2 + &g2_identity));
        let rhs = GT::mul(
            &GT::ate_pairing(&g1, &g2),
            &GT::ate_pairing(&g1, &g2_identity),
        );
        assert!(lhs == rhs);

        let h1 = G1::random();
        let h2 = G2::random();

        // e(g1, g2)*e(identity, h2) == e(g1, g2)
        let lhs = GT::mul(
            &GT::ate_pairing(&g1, &g2),
            &GT::ate_pairing(&g1_identity, &h2),
        );
        let rhs = GT::ate_pairing(&g1, &g2);
        assert!(lhs == rhs);

        // e(identity, g2)*e(h1, h2) == e(h1, h2)
        let lhs = GT::mul(
            &GT::ate_pairing(&g1_identity, &g2),
            &GT::ate_pairing(&h1, &h2),
        );
        let rhs = GT::ate_pairing(&h1, &h2);
        assert!(lhs == rhs);

        assert!(GT::ate_pairing(&g1_identity, &g2_identity).is_one());

        // 2-pairing
        assert!(GT::ate_2_pairing(&g1, &g2, &g1_identity, &h2) == GT::ate_pairing(&g1, &g2));
        assert!(GT::ate_2_pairing(&g1, &g2, &h1, &g2_identity) == GT::ate_pairing(&g1, &g2));
        assert!(GT::ate_2_pairing(&g1_identity, &g2, &h1, &h2) == GT::ate_pairing(&h1, &h2));
        assert!(GT::ate_2_pairing(&g1, &g2_identity, &h1, &h2) == GT::ate_pairing(&h1, &h2));
        assert!(GT::ate_2_pairing(&g1_identity, &g2_identity, &g1_identity, &g2_identity).is_one());

        let k1 = G1::random();
        let k2 = G2::random();

        // multi-pairing
        assert!(
            GT::ate_multi_pairing(vec![(&g1, &g2), (&h1, &h2), (&g1_identity, &k2)])
                == GT::ate_multi_pairing(vec![(&g1, &g2), (&h1, &h2)]),
        );

        assert!(
            GT::ate_multi_pairing(vec![(&g1, &g2), (&h1, &h2), (&k1, &g2_identity)])
                == GT::ate_multi_pairing(vec![(&g1, &g2), (&h1, &h2)]),
        );

        assert!(
            GT::ate_multi_pairing(vec![(&g1, &g2), (&g1_identity, &h2), (&k1, &k2)])
                == GT::ate_multi_pairing(vec![(&g1, &g2), (&k1, &k2)]),
        );

        assert!(
            GT::ate_multi_pairing(vec![(&g1, &g2), (&g1_identity, &h2), (&k1, &k2)])
                == GT::ate_multi_pairing(vec![(&g1, &g2), (&k1, &k2)]),
        );

        assert!(GT::ate_multi_pairing(vec![
            (&g1_identity, &g2_identity),
            (&g1_identity, &g2_identity),
            (&g1_identity, &g2_identity)
        ])
        .is_one());
    }

    #[test]
    fn test_ate_pairing_negative() {
        let g1 = G1::random();
        let g2 = G2::random();
        let g1_neg = -&g1;
        let g2_neg = -&g2;

        // e(g1, -g2) == e(-g1, g2)
        let lhs = GT::ate_pairing(&g1, &g2_neg);
        let rhs = GT::ate_pairing(&g1_neg, &g2);
        assert!(lhs == rhs);

        // e(g1, -g2) == e(-g1, g2) == e(g1, g2)^-1
        let e = GT::ate_pairing(&g1, &g2);
        let e_inv = e.inverse();
        assert!(lhs == e_inv);

        let p = GT::ate_pairing(&g1, &g2);

        // e(g1, g2) = e(-g1, g2)^-1 => e(g1, g2) * e(-g1, g2) == 1
        assert!(GT::mul(&p, &lhs) == GT::one());

        // e(g1, g2) = e(g1, -g2)^-1 => e(g1, g2) * e(g1, -g2) == 1
        assert!(GT::mul(&p, &rhs) == GT::one());
    }

    #[test]
    fn test_ate_pairing() {
        let g1 = G1::random();
        let h1 = G1::random();
        let g2 = G2::random();
        let h2 = G2::random();

        // e(g1 + h1, g2) == e(g1, g2)*e(h1, g2)
        let lhs = GT::ate_pairing(&(&g1 + &h1), &g2);
        let rhs = GT::mul(&GT::ate_pairing(&g1, &g2), &GT::ate_pairing(&h1, &g2));
        let rhs_1 = GT::ate_2_pairing(&g1, &g2, &h1, &g2);
        let rhs_2 = GT::ate_multi_pairing(vec![(&g1, &g2), (&h1, &g2)]);
        assert!(lhs == rhs);
        assert!(rhs_1 == rhs);
        assert!(rhs_2 == rhs);

        // e(g1, g2+h2) == e(g1, g2)*e(g1, h2)
        let lhs = GT::ate_pairing(&g1, &(&g2 + &h2));
        let rhs = GT::mul(&GT::ate_pairing(&g1, &g2), &GT::ate_pairing(&g1, &h2));
        let rhs_1 = GT::ate_2_pairing(&g1, &g2, &g1, &h2);
        let rhs_2 = GT::ate_multi_pairing(vec![(&g1, &g2), (&g1, &h2)]);
        assert!(lhs == rhs);
        assert!(rhs_1 == rhs);
        assert!(rhs_2 == rhs);

        let r = FieldElement::random();
        // e(g1, g2^r) == e(g1^r, g2) == e(g1, g2)^r
        let p1 = GT::ate_pairing(&g1, &(&g2 * &r));
        let p2 = GT::ate_pairing(&(&g1 * &r), &g2);
        let mut p = GT::ate_pairing(&g1, &g2);
        p = p.pow(&r);
        assert!(p1 == p2);
        assert!(p1 == p);
    }

    #[test]
    fn timing_ate_multi_pairing() {
        let count = 10;
        let g1_vec = (0..count).map(|_| G1::random()).collect::<Vec<G1>>();
        let g2_vec = (0..count).map(|_| G2::random()).collect::<Vec<G2>>();
        let mut tuple_vec = vec![];

        let start = Instant::now();
        let mut accum = GT::ate_pairing(&g1_vec[0], &g2_vec[0]);
        tuple_vec.push((&g1_vec[0], &g2_vec[0]));
        for i in 1..count {
            let e = GT::ate_pairing(&g1_vec[i], &g2_vec[i]);
            accum = GT::mul(&accum, &e);
            tuple_vec.push((&g1_vec[i], &g2_vec[i]));
        }
        println!(
            "Time to compute {} pairings naively is {:?}",
            count,
            start.elapsed()
        );

        let start = Instant::now();
        let accum_multi = GT::ate_multi_pairing(tuple_vec);
        println!(
            "Time to compute {} pairings using multi-pairings is {:?}",
            count,
            start.elapsed()
        );
        assert!(accum == accum_multi);
    }

    #[test]
    fn timing_pairing_pow() {
        // Compare cost of e(g1, g2)^r with e(g1^r, g2) and e(g1, g2^r)
        let count = 10;
        let g1_vec = (0..count).map(|_| G1::random()).collect::<Vec<G1>>();
        let g2_vec = (0..count).map(|_| G2::random()).collect::<Vec<G2>>();
        let r_vec = (0..count).map(|_| FieldElement::random()).collect::<Vec<FieldElement>>();

        //e(g1, g2)^r
        let mut pairing_exp = vec![];

        // e(g1^r, g2)
        let mut g1_exp = vec![];

        // e(g1, g2^r)
        let mut g2_exp = vec![];

        let start = Instant::now();
        for i in 0..count {
            pairing_exp.push(GT::pow(&GT::ate_pairing(&g1_vec[i], &g2_vec[i]), &r_vec[i]));
        }
        println!(
            "Time to compute {} pairing and then exponentiation is {:?}",
            count,
            start.elapsed()
        );

        let start = Instant::now();
        for i in 0..count {
            g1_exp.push(GT::ate_pairing(&(&g1_vec[i] * &r_vec[i]), &g2_vec[i]));
        }
        println!(
            "Time to compute {} pairing after exponentiation in G1 is {:?}",
            count,
            start.elapsed()
        );

        let start = Instant::now();
        for i in 0..count {
            g2_exp.push(GT::ate_pairing(&g1_vec[i], &(&g2_vec[i] * &r_vec[i])));
        }
        println!(
            "Time to compute {} pairing after exponentiation in G2 is {:?}",
            count,
            start.elapsed()
        );

        for i in 0..count {
            assert_eq!(pairing_exp[i], g1_exp[i]);
            assert_eq!(pairing_exp[i], g2_exp[i]);
        }
    }
}
