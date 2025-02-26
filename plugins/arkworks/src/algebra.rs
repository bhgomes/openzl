//! Arkworks Algebra

use crate::{
    ec::ProjectiveCurve,
    ff::{BigInteger, Field, FpParameters, PrimeField},
    r1cs_std::{fields::fp::FpVar, groups::CurveVar},
    serialize::CanonicalSerialize,
};
use alloc::vec::Vec;
use core::marker::PhantomData;

#[cfg(feature = "serde")]
use openzl_util::serde::Serializer;

/// Constraint Field Type
type ConstraintField<C> = <<C as ProjectiveCurve>::BaseField as Field>::BasePrimeField;

/// Converts `scalar` to the bit representation of `O`.
#[inline]
pub fn convert_bits<T, O>(scalar: T) -> O::BigInt
where
    T: BigInteger,
    O: PrimeField,
{
    O::BigInt::from_bits_le(&scalar.to_bits_le())
}

/// Checks that the modulus of `A` is smaller than that of `B`.
#[inline]
pub fn modulus_is_smaller<A, B>() -> bool
where
    A: PrimeField,
    B: PrimeField,
{
    let modulus_a = A::Params::MODULUS;
    let modulus_b = B::Params::MODULUS;
    if modulus_a.num_bits() <= modulus_b.num_bits() {
        convert_bits::<_, B>(modulus_a) < modulus_b
    } else {
        modulus_a < convert_bits::<_, A>(modulus_b)
    }
}

/// Converts `point` into its canonical byte-representation.
#[inline]
pub fn affine_point_as_bytes<C>(point: &C::Affine) -> Vec<u8>
where
    C: ProjectiveCurve,
{
    let mut buffer = Vec::new();
    point
        .serialize(&mut buffer)
        .expect("Serialization is not allowed to fail.");
    buffer
}

/// Uses `serializer` to serialize `point`.
#[cfg(feature = "serde")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "serde")))]
#[inline]
pub fn serialize_group_element<C, S>(point: &C::Affine, serializer: S) -> Result<S::Ok, S::Error>
where
    C: ProjectiveCurve,
    S: Serializer,
{
    serializer.serialize_bytes(&affine_point_as_bytes::<C>(point))
}

/// Elliptic Curve Scalar Element Variable
///
/// # Safety
///
/// This type can only be used whenever the embedded scalar field is **smaller** than the
/// outer scalar field.
pub struct ScalarVar<C, CV>(pub(crate) FpVar<ConstraintField<C>>, PhantomData<CV>)
where
    C: ProjectiveCurve,
    CV: CurveVar<C, ConstraintField<C>>;

impl<C, CV> ScalarVar<C, CV>
where
    C: ProjectiveCurve,
    CV: CurveVar<C, ConstraintField<C>>,
{
    /// Builds a new [`ScalarVar`] from a given `scalar`.
    #[inline]
    pub fn new(scalar: FpVar<ConstraintField<C>>) -> Self {
        Self(scalar, PhantomData)
    }
}

/// Returns the modulus bits of scalar field of a given curve `C`.
pub const fn scalar_bits<C>() -> usize
where
    C: ProjectiveCurve,
{
    <<C as ProjectiveCurve>::ScalarField as PrimeField>::Params::MODULUS_BITS as usize
}
