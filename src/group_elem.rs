use rand::{CryptoRng, RngCore};

use crate::errors::{SerzDeserzError, ValueError};
use crate::curve_order_elem::CurveOrderElement;
use std::slice::Iter;

#[macro_export]
macro_rules! add_group_elems {
    ( $( $elem:expr ),* ) => {
        {
            let mut sum = GroupElement::new();
            $(
                sum += $elem;
            )*
            sum
        }
    };
}

pub trait GroupElement: Clone + Sized {
    fn new() -> Self;

    /// Return the identity element
    fn identity() -> Self;

    /// Return a group generator.
    fn generator() -> Self;

    /// Return a random group element
    fn random() -> Self {
        let n = CurveOrderElement::random();
        Self::generator().scalar_mul_const_time(&n)
    }

    /// Return a random group element using the given random number generator
    fn random_using_rng<R: RngCore + CryptoRng>(rng: &mut R) -> Self {
        let n = CurveOrderElement::random_using_rng(rng);
        Self::generator().scalar_mul_const_time(&n)
    }

    /// Check if the the point is the identity element of the group
    fn is_identity(&self) -> bool;

    /// Set the point to the identity element of the group
    fn set_to_identity(&mut self);

    /// Hash an arbitrary sized message using SHAKE and return output as group element
    #[deprecated(since = "0.4.0", note = "Please use `hash_to_curve` instead")]
    fn from_msg_hash(msg: &[u8]) -> Self;

    /// Uses IETF constant time hash_to_curve method to map data to a point
    fn hash_to_curve(msg: &[u8], dst: &hash2curve::DomainSeparationTag) -> Self;

    /// Return byte representation as vector
    fn to_vec(&self) -> Vec<u8>;

    /// Create an element from a byte representation
    fn from_slice(bytes: &[u8]) -> Result<Self, SerzDeserzError>;

    /// Writes bytes to given slice. Raises exception when given slice is not of desired length.
    fn write_to_slice(&self, target: &mut [u8]) -> Result<(), SerzDeserzError>;

    /// Writes bytes to given slice. Will panic when given slice is not of desired length.
    fn write_to_slice_unchecked(&self, target: &mut [u8]);

    /// Add a group element to itself. `self = self + b`
    fn add_assign_(&mut self, b: &Self);

    /// Subtract a group element from itself. `self = self - b`
    fn sub_assign_(&mut self, b: &Self);

    /// Return sum of a group element and itself. `self + b`
    fn plus(&self, b: &Self) -> Self;

    /// Return difference of a group element and itself. `self - b`
    fn minus(&self, b: &Self) -> Self;

    /// Multiply point on the curve (element of group G1) with a scalar. Constant time operation.
    /// self * field_element_a.
    fn scalar_mul_const_time(&self, a: &CurveOrderElement) -> Self;

    /// Return the double of the group element
    fn double(&self) -> Self;

    fn double_mut(&mut self);

    /// Returns hex string as a sequence of FPs separated by whitespace.
    /// Each FP is itself a 2-tuple of strings separated by whitespace, 1st string is the excess and 2nd is a BigNum
    fn to_hex(&self) -> String;

    /// Returns a group element by parsing the hex representation of itself. The hex
    /// representation should match the one from `to_hex`
    fn from_hex(s: String) -> Result<Self, SerzDeserzError>;

    /// Returns negation of this element
    fn negation(&self) -> Self;

    fn is_extension() -> bool;

    /// Checks if the element has correct order by checking if self *  group order (curve order) == Identity element (point at infinity).
    /// Uses constant time scalar multiplication.
    /// Question: But since we always know the multiplicand (group order) is there a faster way?
    fn has_correct_order(&self) -> bool;

    // TODO: Implement has_correct_order for variable time as well. Need to implement variable time scalar multiplication for group G2.
}

#[macro_export]
macro_rules! impl_group_elem_conversions {
    ( $group_element:ident, $group:ident, $group_size:ident ) => {
        impl From<$group> for $group_element {
            fn from(x: $group) -> Self {
                Self { value: x }
            }
        }

        impl From<&$group> for $group_element {
            fn from(x: &$group) -> Self {
                Self { value: x.clone() }
            }
        }

        impl From<&[u8; $group_size]> for $group_element {
            fn from(x: &[u8; $group_size]) -> Self {
                Self {
                    value: $group::frombytes(x),
                }
            }
        }

        impl From<[u8; $group_size]> for $group_element {
            fn from(x: [u8; $group_size]) -> Self {
                Self::from(&x)
            }
        }

        impl Hash for $group_element {
            fn hash<H: Hasher>(&self, state: &mut H) {
                let mut bytes: [u8; $group_size] = [0; $group_size];
                self.write_to_slice_unchecked(&mut bytes);
                state.write(&self.to_vec())
            }
        }
    };
}

#[macro_export]
macro_rules! impl_group_elem_traits {
    ( $group_element:ident, $group:ident ) => {
        impl Default for $group_element {
            fn default() -> Self {
                Self::new()
            }
        }

        #[allow(unused_mut)]
        impl fmt::Display for $group_element {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                let mut c = self.value.clone();
                write!(f, "{}", c.tostring())
            }
        }

        impl Zeroize for $group_element {
            fn zeroize(&mut self) {
                // x, y and z of ECP and ECP2 are private. So the only sensible way of zeroing them out seems setting them to infinity
                use core::{ptr, sync::atomic};
                unsafe {
                    ptr::write_volatile(&mut self.value, $group::new());
                }
                atomic::compiler_fence(atomic::Ordering::SeqCst);
            }
        }

        impl Drop for $group_element {
            fn drop(&mut self) {
                self.zeroize()
            }
        }

        impl Serialize for $group_element {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_newtype_struct("$group_element", &self.to_hex())
            }
        }

        impl<'a> Deserialize<'a> for $group_element {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'a>,
            {
                struct GroupElemVisitor;

                impl<'a> Visitor<'a> for GroupElemVisitor {
                    type Value = $group_element;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("expected $group_element")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<$group_element, E>
                    where
                        E: DError,
                    {
                        Ok($group_element::from_hex(value.to_string()).map_err(DError::custom)?)
                    }
                }

                deserializer.deserialize_str(GroupElemVisitor)
            }
        }
    };
}

#[macro_export]
macro_rules! impl_group_elem_ops {
    ( $group_element:ident ) => {
        impl PartialEq for $group_element {
            fn eq(&self, other: &$group_element) -> bool {
                let l = self.clone();
                let mut r = other.clone();
                l.value.equals(&mut r.value)
            }
        }

        impl Eq for $group_element {}

        impl Add for $group_element {
            type Output = Self;

            fn add(self, other: Self) -> Self {
                self.plus(&other)
            }
        }

        impl Add<$group_element> for &$group_element {
            type Output = $group_element;

            fn add(self, other: $group_element) -> $group_element {
                self.plus(&other)
            }
        }

        impl<'a> Add<&'a $group_element> for $group_element {
            type Output = Self;
            fn add(self, other: &'a $group_element) -> Self {
                self.plus(other)
            }
        }

        impl<'a> Add<&'a $group_element> for &$group_element {
            type Output = $group_element;
            fn add(self, other: &'a $group_element) -> $group_element {
                self.plus(other)
            }
        }

        impl AddAssign for $group_element {
            fn add_assign(&mut self, other: Self) {
                self.add_assign_(&other)
            }
        }

        impl<'a> AddAssign<&'a $group_element> for $group_element {
            fn add_assign(&mut self, other: &'a $group_element) {
                self.add_assign_(other)
            }
        }

        impl Sub for $group_element {
            type Output = Self;

            fn sub(self, other: Self) -> Self {
                self.minus(&other)
            }
        }

        impl Sub<$group_element> for &$group_element {
            type Output = $group_element;

            fn sub(self, other: $group_element) -> $group_element {
                self.minus(&other)
            }
        }

        impl<'a> Sub<&'a $group_element> for $group_element {
            type Output = Self;
            fn sub(self, other: &'a $group_element) -> Self {
                self.minus(other)
            }
        }

        impl<'a> Sub<&'a $group_element> for &$group_element {
            type Output = $group_element;
            fn sub(self, other: &'a $group_element) -> $group_element {
                self.minus(other)
            }
        }

        impl SubAssign for $group_element {
            fn sub_assign(&mut self, other: Self) {
                self.sub_assign_(&other)
            }
        }

        impl<'a> SubAssign<&'a $group_element> for $group_element {
            fn sub_assign(&mut self, other: &'a $group_element) {
                self.sub_assign_(other)
            }
        }

        impl Mul<CurveOrderElement> for $group_element {
            type Output = Self;

            fn mul(self, other: CurveOrderElement) -> Self {
                self.scalar_mul_const_time(&other)
            }
        }

        impl Mul<&CurveOrderElement> for $group_element {
            type Output = Self;

            fn mul(self, other: &CurveOrderElement) -> Self {
                self.scalar_mul_const_time(other)
            }
        }

        impl Mul<CurveOrderElement> for &$group_element {
            type Output = $group_element;

            fn mul(self, other: CurveOrderElement) -> $group_element {
                self.scalar_mul_const_time(&other)
            }
        }

        impl Mul<&CurveOrderElement> for &$group_element {
            type Output = $group_element;

            fn mul(self, other: &CurveOrderElement) -> $group_element {
                self.scalar_mul_const_time(other)
            }
        }

        impl Neg for $group_element {
            type Output = Self;

            fn neg(self) -> Self::Output {
                let mut t = self.to_ecp();
                t.neg();
                t.into()
            }
        }

        impl Neg for &$group_element {
            type Output = $group_element;

            fn neg(self) -> Self::Output {
                let mut t = self.to_ecp();
                t.neg();
                t.into()
            }
        }
    };
}

macro_rules! impl_scalar_mul_ops {
    ( $group_element:ident ) => {
        impl Mul<$group_element> for CurveOrderElement {
            type Output = $group_element;

            fn mul(self, other: $group_element) -> $group_element {
                other.scalar_mul_const_time(&self)
            }
        }

        impl Mul<&$group_element> for CurveOrderElement {
            type Output = $group_element;

            fn mul(self, other: &$group_element) -> $group_element {
                other.scalar_mul_const_time(&self)
            }
        }

        impl Mul<$group_element> for &CurveOrderElement {
            type Output = $group_element;

            fn mul(self, other: $group_element) -> $group_element {
                other.scalar_mul_const_time(self)
            }
        }

        impl Mul<&$group_element> for &CurveOrderElement {
            type Output = $group_element;

            fn mul(self, other: &$group_element) -> $group_element {
                other.scalar_mul_const_time(self)
            }
        }
    };
}

macro_rules! impl_group_element_lookup_table {
    ( $group_element:ident, $name:ident  ) => {
        pub struct $name([$group_element; 8]);

        impl $name {
            /// Given public A and odd x with 0 < x < 2^4, return x.A.
            pub fn select(&self, x: usize) -> &$group_element {
                debug_assert_eq!(x & 1, 1);
                debug_assert!(x < 16);

                &self.0[x / 2]
            }
        }

        impl<'a> From<&'a $group_element> for $name {
            fn from(a: &'a $group_element) -> Self {
            let mut a_i: [$group_element; 8] = [
                    $group_element::new(),
                    $group_element::new(),
                    $group_element::new(),
                    $group_element::new(),
                    $group_element::new(),
                    $group_element::new(),
                    $group_element::new(),
                    $group_element::new(),
                ];
                let a_2 = a.double();
                a_i[0] = a.clone();
                for i in 0..7 {
                    a_i[i + 1] = &a_i[i] + &a_2;
                }
                // Now Ai = [A, 3A, 5A, 7A, 9A, 11A, 13A, 15A]
                Self(a_i)
            }
        }
    };
}

macro_rules! impl_optmz_scalar_mul_ops {
    ( $group_element:ident, $group:ident, $lookup_table:ident ) => {
        impl $group_element {
            /// Return underlying elliptic curve point, ECP
            pub fn to_ecp(&self) -> $group {
                self.value.clone()
            }

            /// Multiply point on the curve (element of group G1) with a scalar. Variable time operation
            /// Uses wNAF.
            pub fn scalar_mul_variable_time(&self, a: &CurveOrderElement) -> Self {
                // TODO: Optimization: Attach the lookup table to the struct
                let table = $lookup_table::from(self);
                let wnaf = a.to_wnaf(5);
                $group_element::wnaf_mul(&table, &wnaf)
            }

            /// Return multiples of itself. eg. Given `n`=5, returns self, 2*self, 3*self, 4*self, 5*self
            pub fn get_multiples(&self, n: usize) -> Vec<$group_element> {
                // TODO: Can use `selector` from ECP
                let mut res = vec![self.clone()];
                for i in 2..=n {
                    res.push(&res[i - 2] + self);
                }
                res
            }

            pub fn to_wnaf_lookup_table(&self, width: usize) -> $lookup_table {
                // Only supporting table of width 5 for now
                debug_assert_eq!(width, 5);
                $lookup_table::from(self)
            }

            pub fn wnaf_mul(table: &$lookup_table, wnaf: &[i8]) -> Self {
                let mut result = $group_element::identity();

                for n in wnaf.iter().rev() {
                    result = result.double();

                    let v = *n;
                    if v > 0 {
                        result = result + table.select(v as usize);
                    } else if v < 0 {
                        result = result - table.select(-v as usize);
                    }
                }

                result
            }
        }
    };
}

pub trait GroupElementVector<T>: Sized {
    fn new(size: usize) -> Self;

    fn with_capacity(capacity: usize) -> Self;

    fn as_slice(&self) -> &[T];

    fn as_mut_slice(&mut self) -> &mut [T];

    fn len(&self) -> usize;

    fn push(&mut self, value: T);

    fn append(&mut self, other: &mut Self);

    fn pop(&mut self) -> Option<T>;

    fn insert(&mut self, index: usize, element: T);

    fn remove(&mut self, index: usize) -> T;

    /// Compute sum of all elements of the vector
    fn sum(&self) -> T;

    /// Multiply each element of the vector with a given field
    /// element `n` (scale the vector). Modifies the vector.
    fn scale(&mut self, n: &CurveOrderElement);

    /// Multiply each element of the vector with a given field
    /// element `n` to create a new vector
    fn scaled_by(&self, n: &CurveOrderElement) -> Self;

    /// Add 2 vectors
    fn plus(&self, b: &Self) -> Result<Self, ValueError>;

    /// Subtract 2 vectors
    fn minus(&self, b: &Self) -> Result<Self, ValueError>;

    fn iter(&self) -> Iter<T>;

    fn random(size: usize) -> Self;
}

#[macro_export]
macro_rules! impl_group_elem_vec_ops {
    ( $group_element:ident, $group_element_vec:ident ) => {
        impl GroupElementVector<$group_element> for $group_element_vec {
            fn new(size: usize) -> Self {
                Self {
                    elems: (0..size)
                        .into_par_iter()
                        .map(|_| $group_element::new())
                        .collect(),
                }
            }

            fn with_capacity(capacity: usize) -> Self {
                Self {
                    elems: Vec::<$group_element>::with_capacity(capacity),
                }
            }

            fn as_slice(&self) -> &[$group_element] {
                &self.elems
            }

            fn as_mut_slice(&mut self) -> &mut [$group_element] {
                &mut self.elems
            }

            fn len(&self) -> usize {
                self.elems.len()
            }

            fn push(&mut self, value: $group_element) {
                self.elems.push(value)
            }

            fn append(&mut self, other: &mut Self) {
                self.elems.append(&mut other.elems)
            }

            fn pop(&mut self) -> Option<$group_element> {
                self.elems.pop()
            }

            fn insert(&mut self, index: usize, element: $group_element) {
                self.elems.insert(index, element)
            }

            fn remove(&mut self, index: usize) -> $group_element {
                self.elems.remove(index)
            }

            fn sum(&self) -> $group_element {
                self.as_slice()
                    .par_iter()
                    .cloned()
                    .reduce(|| $group_element::new(), |a, b| a + b)
            }

            fn scale(&mut self, n: &CurveOrderElement) {
                // TODO: Since each element is multiplied with same field element, use the
                // optimized version.
                for i in 0..self.len() {
                    self[i] = &self[i] * n;
                }
            }

            fn scaled_by(&self, n: &CurveOrderElement) -> Self {
                // TODO: Since each element is multiplied with same field element, use the
                // optimized version.
                let mut scaled = Self::with_capacity(self.len());
                for i in 0..self.len() {
                    scaled.push(&self[i] * n)
                }
                scaled.into()
            }

            fn plus(&self, b: &Self) -> Result<Self, ValueError> {
                check_vector_size_for_equality!(self, b)?;
                let mut sum_vector = Self::new(self.len());
                sum_vector
                    .as_mut_slice()
                    .par_iter_mut()
                    .enumerate()
                    .for_each(|(i, e)| *e = &self[i] + &b[i]);
                Ok(sum_vector)
            }

            fn minus(&self, b: &Self) -> Result<Self, ValueError> {
                check_vector_size_for_equality!(self, b)?;
                let mut diff_vector = Self::new(self.len());
                diff_vector
                    .as_mut_slice()
                    .par_iter_mut()
                    .enumerate()
                    .for_each(|(i, e)| *e = &self[i] - &b[i]);
                Ok(diff_vector)
            }

            fn iter(&self) -> Iter<$group_element> {
                self.as_slice().iter()
            }

            fn random(size: usize) -> Self {
                (0..size)
                    .into_par_iter()
                    .map(|_| $group_element::random())
                    .collect::<Vec<$group_element>>()
                    .into()
            }
        }
    };
}

macro_rules! impl_group_elem_vec_product_ops {
    ( $group_element:ident, $group_element_vec:ident, $lookup_table:ident ) => {
        impl $group_element_vec {
            /// Computes inner product of 2 vectors, one of field elements and other of group elements.
            /// [a1, a2, a3, ...field elements].[b1, b2, b3, ...group elements] = (a1*b1 + a2*b2 + a3*b3)
            pub fn inner_product_const_time<'g, 'f>(
                &'g self,
                b: impl IntoIterator<Item = &'f CurveOrderElement>,
            ) -> Result<$group_element, ValueError> {
                self.multi_scalar_mul_const_time(b)
            }

            pub fn inner_product_var_time<'g, 'f>(
                &'g self,
                b: impl IntoIterator<Item = &'f CurveOrderElement>,
            ) -> Result<$group_element, ValueError> {
                self.multi_scalar_mul_var_time(b)
            }

            #[deprecated(since = "0.3.0", note = "Please use the `inner_product_var_time` function instead")]
            pub fn inner_product_var_time_with_ref_vecs(
                group_elems: Vec<&$group_element>,
                field_elems: Vec<&CurveOrderElement>,
            ) -> Result<$group_element, ValueError> {
                Self::multi_scalar_mul_var_time_without_precomputation(group_elems, field_elems)
            }

            /// Calculates Hadamard product of 2 group element vectors.
            /// Hadamard product of `a` and `b` = `a` o `b` = (a0 o b0, a1 o b1, ...).
            /// Here `o` denotes group operation, which in elliptic curve is point addition
            pub fn hadamard_product(&self, b: &Self) -> Result<Self, ValueError> {
                check_vector_size_for_equality!(self, b)?;
                let mut hadamard_product = Self::new(self.len());
                hadamard_product.as_mut_slice().par_iter_mut().enumerate().for_each(|(i, e)| {
                    *e = &self[i] + &b[i]
                });
                Ok(hadamard_product)
            }

            pub fn split_at(&self, mid: usize) -> (Self, Self) {
                let (l, r) = self.as_slice().split_at(mid);
                (Self::from(l), Self::from(r))
            }

            /// Constant time multi-scalar multiplication. Naive approach computing `n` scalar
            /// multiplications and n-1 additions for `n` field elements
            pub fn multi_scalar_mul_const_time_naive(
                &self,
                field_elems: &CurveOrderElementVector,
            ) -> Result<$group_element, ValueError> {
                check_vector_size_for_equality!(field_elems, self)?;
                let mut accum = $group_element::new();
                for i in 0..self.len() {
                    accum += &self[i] * &field_elems[i];
                }
                Ok(accum)
            }

            /// Constant time multi-scalar multiplication
            pub fn multi_scalar_mul_const_time<'g, 'f>(
                &'g self,
                field_elems: impl IntoIterator<Item = &'f CurveOrderElement>,
            ) -> Result<$group_element, ValueError> {
                Self::multi_scalar_mul_const_time_without_precomputation(self.as_slice(), field_elems)
            }

            /// Variable time multi-scalar multiplication
            pub fn multi_scalar_mul_var_time<'g, 'f>(
                &'g self,
                field_elems: impl IntoIterator<Item = &'f CurveOrderElement>,
            ) -> Result<$group_element, ValueError> {
                Self::multi_scalar_mul_var_time_without_precomputation(self.as_slice(), field_elems)
            }

            /// Strauss multi-scalar multiplication
            pub fn multi_scalar_mul_var_time_without_precomputation<'g, 'f>(
                group_elems: impl IntoIterator<Item = &'g $group_element>,
                field_elems: impl IntoIterator<Item = &'f CurveOrderElement>,
            ) -> Result<$group_element, ValueError> {
                let lookup_tables: Vec<_> = group_elems
                    .into_iter()
                    .map(|e| $lookup_table::from(e))
                    .collect();

                Self::multi_scalar_mul_var_time_with_precomputation_done(
                    &lookup_tables,
                    field_elems,
                )
            }

            #[deprecated(since = "0.3.0", note = "Please use the `multi_scalar_mul_var_time_without_precomputation` function instead")]
            pub fn multi_scalar_mul_var_time_from_ref_vecs(
                group_elems: Vec<&$group_element>,
                field_elems: Vec<&CurveOrderElement>,
            ) -> Result<$group_element, ValueError> {
                let lookup_tables: Vec<_> = group_elems
                    .iter()
                    .map(|e| $lookup_table::from(*e))
                    .collect();

                Self::multi_scalar_mul_var_time_with_precomputation_done(
                    &lookup_tables,
                    field_elems,
                )
            }

            /// Strauss multi-scalar multiplication. Passing the lookup tables since in lot of cases generators will be fixed
            pub fn multi_scalar_mul_var_time_with_precomputation_done<'f>(
                lookup_tables: &[$lookup_table],
                field_elems: impl IntoIterator<Item = &'f CurveOrderElement>,
            ) -> Result<$group_element, ValueError> {
                let mut nafs: Vec<_> = field_elems.into_iter().map(|e| e.to_wnaf(5)).collect();

                check_vector_size_for_equality!(nafs, lookup_tables)?;

                // Pad the NAFs with 0 so that all nafs are of same length
                let new_length = pad_collection!(nafs, 0);

                let mut r = $group_element::identity();

                for i in (0..new_length).rev() {
                    let mut t = r.double();

                    for (naf, lookup_table) in nafs.iter().zip(lookup_tables.iter()) {
                        if naf[i] > 0 {
                            t = t + lookup_table.select(naf[i] as usize);
                        } else if naf[i] < 0 {
                            t = t - lookup_table.select(-naf[i] as usize);
                        }
                    }
                    r = t;
                }

                Ok(r)
            }

            /// Constant time multi-scalar multiplication.
            /// Taken from Guide to Elliptic Curve Cryptography book, "Algorithm 3.48 Simultaneous multiple point multiplication" without precomputing the addition
            /// Still helps with reducing doublings
            pub fn multi_scalar_mul_const_time_without_precomputation<'g, 'f>(
                group_elems: impl IntoIterator<Item = &'g $group_element>,
                field_elems: impl IntoIterator<Item = &'f CurveOrderElement>,
            ) -> Result<$group_element, ValueError> {

                // Choosing window of size 3.
                let group_elem_multiples: Vec<_> = group_elems
                    .into_iter()
                    .map(|e| e.get_multiples(7)) // 2^3 - 1
                    .collect();

                Self::multi_scalar_mul_const_time_with_precomputation_done(
                    &group_elem_multiples,
                    field_elems,
                )
            }

            pub fn multi_scalar_mul_const_time_with_precomputation_done<'f>(
                group_elem_multiples: &[Vec<$group_element>],
                field_elems: impl IntoIterator<Item = &'f CurveOrderElement>,
            ) -> Result<$group_element, ValueError> {
                // TODO: The test shows that precomputing multiples does not help much. Experiment with bigger window.

                let mut field_elems_base_repr: Vec<_> = field_elems
                    .into_iter()
                    .map(|e| e.to_power_of_2_base(3))
                    .collect();

                check_vector_size_for_equality!(group_elem_multiples, field_elems_base_repr)?;

                // Pad the representations with 0 so that all are of same length
                let new_length = pad_collection!(field_elems_base_repr, 0);

                let mut r = $group_element::new();
                for i in (0..new_length).rev() {
                    // r = r * 2^3
                    r.double_mut();
                    r.double_mut();
                    r.double_mut();
                    for (b, m) in field_elems_base_repr
                        .iter()
                        .zip(group_elem_multiples.iter())
                    {
                        // TODO: The following can be replaced with a pre-computation.
                        if b[i] != 0 {
                            r = r + &m[(b[i] - 1) as usize]
                        }
                    }
                }
                Ok(r)
            }

            /// Non-constant time operation. Scale this group element vector by a factor. Each group
            /// element is multiplied by the same factor so wnaf is computed only once.
            pub fn scale_var_time(&mut self, n: &CurveOrderElement) {
                let wnaf = n.to_wnaf(5);
                self.elems.as_mut_slice().par_iter_mut().for_each(|e| {
                    let table = $lookup_table::from(&(*e));
                    *e = $group_element::wnaf_mul(&table, &wnaf);
                })
            }

            /// Non-constant time operation. Return a scaled vector. Each group
            /// element is multiplied by the same factor so wnaf is computed only once.
            pub fn scaled_by_var_time(&self, n: &CurveOrderElement) -> Self {
                let mut scaled: Self = self.clone();
                scaled.scale_var_time(n);
                scaled
            }
        }
    };
}

#[macro_export]
macro_rules! impl_group_elem_vec_conversions {
    ( $group_element:ident, $group_element_vec:ident ) => {
        impl From<Vec<$group_element>> for $group_element_vec {
            fn from(x: Vec<$group_element>) -> Self {
                Self { elems: x }
            }
        }

        impl From<&[$group_element]> for $group_element_vec {
            fn from(x: &[$group_element]) -> Self {
                Self { elems: x.to_vec() }
            }
        }

        impl Into<Vec<$group_element>> for $group_element_vec {
            fn into(self) -> Vec<$group_element> {
                self.elems
            }
        }

        impl<'a> Into<&'a [$group_element]> for &'a $group_element_vec {
            fn into(self) -> &'a [$group_element] {
                &self.elems
            }
        }

        impl Index<usize> for $group_element_vec {
            type Output = $group_element;

            fn index(&self, idx: usize) -> &$group_element {
                &self.elems[idx]
            }
        }

        impl IndexMut<usize> for $group_element_vec {
            fn index_mut(&mut self, idx: usize) -> &mut $group_element {
                &mut self.elems[idx]
            }
        }

        impl PartialEq for $group_element_vec {
            fn eq(&self, other: &Self) -> bool {
                if self.len() != other.len() {
                    return false;
                }
                for i in 0..self.len() {
                    if self[i] != other[i] {
                        return false;
                    }
                }
                true
            }
        }

        impl IntoIterator for $group_element_vec {
            type Item = $group_element;
            type IntoIter = ::std::vec::IntoIter<$group_element>;

            fn into_iter(self) -> Self::IntoIter {
                self.elems.into_iter()
            }
        }

        impl AsRef<[$group_element]> for $group_element_vec {
            fn as_ref(&self) -> &[$group_element] {
                self.elems.as_slice()
            }
        }
    };
}

#[cfg(test)]
mod test {
    use super::*;
    use serde::{Serialize, Deserialize};
    use crate::constants::GROUP_G1_SIZE;
    use crate::curve_order_elem::CurveOrderElementVector;
    #[cfg(any(feature = "bls381", feature = "bn254"))]
    use crate::constants::{GROUP_G2_SIZE, GROUP_GT_SIZE};
    #[cfg(any(feature = "bls381", feature = "bn254"))]
    use crate::extension_field_gt::GT;
    use crate::group_elem_g1::{G1LookupTable, G1Vector, G1};
    #[cfg(any(feature = "bls381", feature = "bn254"))]
    use crate::group_elem_g2::{G2LookupTable, G2Vector, G2};
    use std::collections::{HashMap, HashSet};
    use std::time::Instant;

    #[test]
    fn test_to_and_from_bytes() {
        let count = 100;
        macro_rules! to_and_fro_bytes {
            ( $group:ident, $group_size:ident ) => {
                for _ in 0..count {
                    let x = $group::random();
                    let mut bytes: [u8; $group_size] = [0; $group_size];
                    x.write_to_slice(&mut bytes).unwrap();
                    let y = $group::from(&bytes);
                    assert_eq!(x, y);

                    let bytes1 = x.to_vec();
                    assert_eq!(x, $group::from_slice(bytes1.as_slice()).unwrap());

                    // Increase length of byte vector by adding a byte. Choice of byte is arbitrary
                    let mut bytes2 = bytes1.clone();
                    bytes2.push(0);
                    assert!($group::from_slice(&bytes2).is_err());
                    assert!(x.write_to_slice(&mut bytes2).is_err());

                    // Decrease length of byte vector
                    assert!($group::from_slice(&bytes2[0..$group_size - 4]).is_err());
                    assert!(x.write_to_slice(&mut bytes2[0..$group_size - 4]).is_err());
                }
            };
        }

        to_and_fro_bytes!(G1, GROUP_G1_SIZE);
        #[cfg(any(feature = "bls381", feature = "bn254"))]
        to_and_fro_bytes!(G2, GROUP_G2_SIZE);
        #[cfg(any(feature = "bls381", feature = "bn254"))]
        to_and_fro_bytes!(GT, GROUP_GT_SIZE);
    }

    #[test]
    fn test_hashing() {
        // If the element can be added to HashSet or HashMap, it must be hashable.
        macro_rules! hashing {
            ( $group:ident ) => {{
                let mut set = HashSet::new();
                let mut map = HashMap::new();
                set.insert($group::random());
                map.insert($group::random(), $group::random());
            }};
        }

        hashing!(G1);
        #[cfg(any(feature = "bls381", feature = "bn254"))]
        hashing!(G2);
    }

    #[test]
    fn test_equality() {
        macro_rules! eql {
            ( $group:ident ) => {
                for _ in 0..10 {
                    // Very unlikely that 2 randomly chosen elements will be equal
                    let a = $group::random();
                    let b = $group::random();
                    assert_ne!(&a, &b);
                }
            };
        }
        eql!(G1);
        #[cfg(any(feature = "bls381", feature = "bn254"))]
        eql!(G2);
    }

    #[test]
    fn test_negating_group_elems() {
        macro_rules! negating {
            ( $group:ident ) => {{
                let b = $group::random();
                let neg_b = -&b;
                assert_ne!(b, neg_b);
                let neg_neg_b = -&neg_b;
                assert_eq!(b, neg_neg_b);
                assert_eq!(&b + &neg_b, $group::identity());
            }};
        }
        negating!(G1);
        #[cfg(any(feature = "bls381", feature = "bn254"))]
        negating!(G2);
    }

    #[test]
    fn test_scalar_mult_operators() {
        macro_rules! scalar_mult {
            ( $group:ident ) => {
                for _ in 0..10 {
                    let g = $group::random();
                    let f = CurveOrderElement::random();
                    let m = g.scalar_mul_const_time(&f);
                    // Operands can be in any order
                    assert_eq!(m, &g * &f);
                    assert_eq!(m, &f * &g);
                }
            };
        }

        scalar_mult!(G1);
        #[cfg(any(feature = "bls381", feature = "bn254"))]
        scalar_mult!(G2)
    }

    #[test]
    fn test_group_elem_addition() {
        let count = 10;
        macro_rules! addition {
            ( $group:ident ) => {{
                for _ in 0..count {
                    let a = $group::random();
                    let b = $group::random();
                    let c = $group::random();

                    let sum = &a + &b + &c;

                    let mut expected_sum = $group::new();
                    expected_sum = expected_sum.plus(&a);
                    expected_sum = expected_sum.plus(&b);
                    expected_sum = expected_sum.plus(&c);
                    assert_eq!(sum, expected_sum);
                }
            }};
        }
        addition!(G1);
        #[cfg(any(feature = "bls381", feature = "bn254"))]
        addition!(G2);
    }

    #[test]
    fn test_negation() {
        macro_rules! neg {
            ( $group:ident ) => {
                for _ in 0..10 {
                    let a = $group::random();
                    let b = a.negation();
                    assert!((a + b).is_identity())
                }
            };
        }

        neg!(G1);
        #[cfg(any(feature = "bls381", feature = "bn254"))]
        neg!(G2);
    }

    #[test]
    fn timing_correct_order_check() {
        let count = 10;
        macro_rules! order_check {
            ( $group:ident ) => {{
                let start = Instant::now();
                for _ in 0..count {
                    let a = $group::random();
                    assert!(a.has_correct_order())
                }
                println!(
                    "For {} elements, time to check correct order is {:?}",
                    count,
                    start.elapsed()
                )
            }};
        }
        order_check!(G1);
        #[cfg(any(feature = "bls381", feature = "bn254"))]
        order_check!(G2);
    }

    #[test]
    fn timing_group_elem_addition_and_scalar_multiplication() {
        let count = 100;
        macro_rules! add_mul {
            ( $group:ident ) => {
                let points: Vec<_> = (0..100).map(|_| $group::random()).collect();
                let mut r = $group::random();
                let mut start = Instant::now();
                for i in 0..count {
                    r = r + &points[i];
                }
                println!("Addition time for {} elems = {:?}", count, start.elapsed());

                let fs: Vec<_> = (0..100).map(|_| CurveOrderElement::random()).collect();
                start = Instant::now();
                for i in 0..count {
                    let _  = &points[i] * &fs[i];
                }
                println!(
                    "Scalar multiplication time for {} elems = {:?}",
                    count,
                    start.elapsed()
                );
            };
        }

        add_mul!(G1);
        #[cfg(any(feature = "bls381", feature = "bn254"))]
        add_mul!(G2);
    }

    #[test]
    fn test_hex_group_elem() {
        macro_rules! hex {
            ( $group:ident ) => {
                for _ in 0..100 {
                    let r = $group::random();
                    let h = r.to_hex();
                    let r_ = $group::from_hex(h).unwrap();
                    assert_eq!(r, r_);

                    // Very unlikely that 2 randomly chosen elements will be equal
                    let s = $group::random();
                    assert_ne!(r, s);
                }
            };
        }
        hex!(G1);
        #[cfg(any(feature = "bls381", feature = "bn254"))]
        hex!(G2);
        #[cfg(any(feature = "bls381", feature = "bn254"))]
        hex!(GT);
    }

    #[test]
    fn test_serialization_deserialization_group_elem() {
        macro_rules! serz {
            ( $group:ident, $s_name:ident ) => {
                #[derive(Serialize, Deserialize)]
                struct $s_name {
                    val: $group,
                }

                for _ in 0..100 {
                    let r = $group::random();
                    let s = $s_name { val: r.clone() };

                    let sz = serde_json::to_string(&s);

                    let st = sz.unwrap();
                    let g: $s_name = serde_json::from_str(&st).unwrap();
                    assert_eq!(g.val, r)
                }
            };
        }

        serz!(G1, S1);
        #[cfg(any(feature = "bls381", feature = "bn254"))]
        serz!(G2, S2);
        #[cfg(any(feature = "bls381", feature = "bn254"))]
        serz!(GT, ST);
    }

    #[test]
    fn test_lookup_table() {
        let x = [1, 3, 5, 7, 9, 11, 13, 15];
        macro_rules! lk_tbl {
            ( $group:ident, $lookup_table:ident ) => {
                let a = $group::random();
                let table = $lookup_table::from(&a);
                for i in x.iter() {
                    let f = CurveOrderElement::from(*i as u8);
                    let expected = &a * f;
                    assert_eq!(expected, *table.select(*i as usize));
                }
            };
        }
        lk_tbl!(G1, G1LookupTable);
        #[cfg(any(feature = "bls381", feature = "bn254"))]
        lk_tbl!(G2, G2LookupTable);
    }

    #[test]
    fn test_wnaf_mul() {
        macro_rules! wnaf_mul {
            ( $group:ident, $lookup_table:ident ) => {
                for _ in 0..100 {
                    let a = $group::random();
                    let r = CurveOrderElement::random();
                    let expected = &a * &r;

                    let table = $lookup_table::from(&a);
                    let wnaf = r.to_wnaf(5);
                    let p = $group::wnaf_mul(&table, &wnaf);

                    assert_eq!(expected, p);
                }
            };
        }
        wnaf_mul!(G1, G1LookupTable);
        #[cfg(any(feature = "bls381", feature = "bn254"))]
        wnaf_mul!(G2, G2LookupTable);
    }

    #[test]
    fn test_multi_scalar_multiplication() {
        macro_rules! mul_scal_mul {
            ( $group:ident, $vector:ident ) => {
                for _ in 0..5 {
                    let mut fs = vec![];
                    let mut gs = vec![];
                    let gen = $group::generator();

                    for i in 0..70 {
                        fs.push(CurveOrderElement::random());
                        gs.push(gen.scalar_mul_const_time(&fs[i]));
                    }

                    let gv = $vector::from(gs.as_slice());
                    let fv = CurveOrderElementVector::from(fs.as_slice());
                    let res = gv.multi_scalar_mul_const_time_naive(&fv).unwrap();

                    let res_1 = gv.multi_scalar_mul_var_time(fv.as_ref()).unwrap();

                    let mut expected = $group::new();
                    let mut expected_1 = $group::new();
                    for i in 0..fs.len() {
                        expected.add_assign_(&gs[i].scalar_mul_const_time(&fs[i]));
                        expected_1.add_assign_(&(&gs[i] * &fs[i]));
                    }

                    let res_2 = $vector::multi_scalar_mul_const_time_without_precomputation(
                        gs.as_slice(),
                        fs.as_slice(),
                    )
                    .unwrap();

                    assert_eq!(expected, res);
                    assert_eq!(expected_1, res);
                    assert_eq!(res_1, res);
                    assert_eq!(res_2, res);

                    let res_3 = $vector::multi_scalar_mul_const_time_without_precomputation(
                        gv.as_ref(),
                        fv.as_ref(),
                    )
                    .unwrap();
                    assert_eq!(res_3, res);
                }
            };
        }
        mul_scal_mul!(G1, G1Vector);
        #[cfg(any(feature = "bls381", feature = "bn254"))]
        mul_scal_mul!(G2, G2Vector);
    }

    #[test]
    fn timing_vector_scaling() {
        let size = 30;
        macro_rules! scale {
            ( $group_vec:ident ) => {
                let r = CurveOrderElement::random();
                let vector = $group_vec::random(size);
                let start = Instant::now();
                let s1 = vector.scaled_by(&r);
                println!(
                    "Constant time scaling for {} elems takes {:?}",
                    size,
                    start.elapsed()
                );

                let start = Instant::now();
                let s2 = vector.scaled_by_var_time(&r);
                println!(
                    "Variable time scaling for {} elems takes {:?}",
                    size,
                    start.elapsed()
                );

                assert_eq!(s1, s2);
                let mut s3 = vector.clone();
                s3.scale_var_time(&r);
                assert_eq!(s1, s3)
            };
        }
        scale!(G1Vector);
        #[cfg(any(feature = "bls381", feature = "bn254"))]
        scale!(G2Vector);
    }
}
