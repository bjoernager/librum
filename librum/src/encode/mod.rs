// Copyright 2024 Gabriel Bjørnager Jensen.
//
// This file is part of Librum.
//
// Librum is free software: you can redistribute it
// and/or modify it under the terms of the GNU
// Lesser General Public License as published by
// the Free Software Foundation, either version 3
// of the License, or (at your option) any later
// version.
//
// Librum is distributed in the hope that it will
// be useful, but WITHOUT ANY WARRANTY; without
// even the implied warranty of MERCHANTABILITY or
// FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// Lesser General Public License for more details.
//
// You should have received a copy of the GNU Less-
// er General Public License along with Librum. If
// not, see <https://www.gnu.org/licenses/>.

#[cfg(test)]
mod tests;

use crate::OStream;
use crate::error::{
	CollectionEncodeError,
	EnumEncodeError,
	IsizeEncodeError,
	ItemEncodeError,
	RefCellEncodeError,
	UsizeEncodeError,
};

use core::cell::{Cell, LazyCell, RefCell};
use core::convert::Infallible;
use core::ffi::CStr;
use core::hash::BuildHasher;
use core::hint::unreachable_unchecked;
use core::marker::{PhantomData, PhantomPinned};
use core::net::{
	IpAddr,
	Ipv4Addr,
	Ipv6Addr,
	SocketAddr,
	SocketAddrV4,
	SocketAddrV6,
};
use core::num::{Saturating, Wrapping};
use core::ops::{
	Bound,
	Range,
	RangeFrom,
	RangeFull,
	RangeInclusive,
	RangeTo,
	RangeToInclusive,
};
use core::time::Duration;

#[cfg(feature = "alloc")]
use alloc::borrow::{Cow, ToOwned};

#[cfg(feature = "alloc")]
use alloc::boxed::Box;

#[cfg(feature = "alloc")]
use alloc::collections::LinkedList;

#[cfg(feature = "alloc")]
use alloc::ffi::CString;

#[cfg(feature = "alloc")]
use alloc::string::String;

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

#[cfg(feature = "alloc")]
use alloc::rc::Rc;

#[cfg(all(feature = "alloc", target_has_atomic = "ptr"))]
use alloc::sync::Arc;

#[cfg(feature = "std")]
use std::collections::{HashMap, HashSet};

#[cfg(feature = "std")]
use std::sync::{LazyLock, Mutex, RwLock};

#[cfg(feature = "std")]
use std::time::{SystemTime, UNIX_EPOCH};

/// Denotes a type capable of being encoded.
///
/// It is recommended to simply derive this trait for custom types.
/// It can, of course, also just be manually implemented.
///
/// If all possible encodings have a known maximum size, then the [`SizedEncode`](crate::SizedEncode) trait should additionally be implemented.
///
/// # Examples
///
/// A manual implementation of `Encode`:
///
/// ```
/// // Manual implementation of custom type. This im-
/// // plementation is equivalent to what would have
/// // been derived.
///
/// use librum::{Encode, OStream};
/// use core::convert::Infallible;
///
/// struct Foo {
///     bar: u16,
///     baz: f32,
/// }
///
/// impl Encode for Foo {
///     // Both `u16` and `f32` encode infallibly.
///
///     type Error = Infallible;
///
///     fn encode(&self, stream: &mut OStream) -> Result<(), Self::Error> {
///         // Encode fields using chaining.
///
///         self.bar.encode(stream)?;
///         self.baz.encode(stream)?;
///
///         Ok(())
///     }
/// }
/// ```
pub trait Encode {
	type Error;

	/// Encodes `self` into the provided stream.
	///
	/// # Errors
	///
	/// If encoding fails, such as if `self` is unencodable, an error is returned.
	///
	/// # Panics
	///
	/// If `stream` cannot contain the entirety of the resulting encoding, then this method should panic.
	fn encode(&self, stream: &mut OStream) -> Result<(), Self::Error>;
}

impl<T: Encode + ?Sized> Encode for &T {
	type Error = T::Error;

	#[inline(always)]
	fn encode(&self, stream: &mut OStream) -> Result<(), Self::Error> {
		T::encode(self, stream)
	}
}

impl<T: Encode + ?Sized> Encode for &mut T {
	type Error = T::Error;

	#[inline(always)]
	fn encode(&self, stream: &mut OStream) -> Result<(), Self::Error> {
		T::encode(self, stream)
	}
}

/// Implemented for tuples with up to twelve members.
#[cfg_attr(doc, doc(fake_variadic))]
impl<T: Encode> Encode for (T, ) {
	type Error = T::Error;

	#[inline(always)]
	fn encode(&self, stream: &mut OStream) -> Result<(), Self::Error> {
		self.0.encode(stream)
	}
}

impl<T: Encode, const N: usize> Encode for [T; N] {
	type Error = CollectionEncodeError<Infallible, ItemEncodeError<usize, T::Error>>;

	/// Encodes each element sequentially.
	/// The length is hard-coded into the type and is therefore not encoded.
	#[inline(always)]
	fn encode(&self, stream: &mut OStream) -> Result<(), Self::Error> {
		for (i, v) in self.iter().enumerate() {
			v
				.encode(stream)
				.map_err(|e| CollectionEncodeError::Item(ItemEncodeError { index: i, error: e }))?;
		}

		Ok(())
	}
}

impl<T: Encode> Encode for [T] {
	type Error = CollectionEncodeError<UsizeEncodeError, ItemEncodeError<usize, T::Error>>;

	/// Encodes each element sequentially with an extra length specifier (of type [`usize`]) prepended first.
	#[inline(always)]
	fn encode(&self, stream: &mut OStream) -> Result<(), Self::Error> {
		self
			.len()
			.encode(stream)
			.map_err(CollectionEncodeError::Length)?;

		for (i,v) in self.iter().enumerate() {
			v
				.encode(stream)
				.map_err(|e| CollectionEncodeError::Item(ItemEncodeError { index: i, error: e }))?;
		}

		Ok(())
	}
}

#[cfg(all(feature = "alloc", target_has_atomic = "ptr"))]
#[cfg_attr(doc, doc(cfg(all(feature = "alloc", target_has_atomic = "ptr"))))]
impl<T: Encode + ?Sized> Encode for Arc<T> {
	type Error = T::Error;

	#[inline(always)]
	fn encode(&self, stream: &mut OStream) -> Result<(), Self::Error> {
		T::encode(self, stream)
	}
}

impl Encode for bool {
	type Error = <u8 as Encode>::Error;

	#[inline(always)]
	fn encode(&self, stream: &mut OStream) -> Result<(), Self::Error> {
		u8::from(*self).encode(stream)
	}
}

impl<T: Encode> Encode for Bound<T> {
	type Error = EnumEncodeError<u8, T::Error>;

	#[inline(always)]
	fn encode(&self, stream: &mut OStream) -> Result<(), Self::Error> {
		match *self {
			Self::Included(ref bound) => {
				0x0u8.encode(stream).unwrap();
				bound.encode(stream).map_err(EnumEncodeError::Field)?;
			}

			Self::Excluded(ref bound) => {
				0x1u8.encode(stream).unwrap();
				bound.encode(stream).map_err(EnumEncodeError::Field)?;
			}

			Self::Unbounded => {
				0x2u8.encode(stream).unwrap();
			}
		}

		Ok(())
	}
}

#[cfg(feature = "alloc")]
#[cfg_attr(doc, doc(cfg(feature = "alloc")))]
impl<T: Encode + ?Sized> Encode for Box<T> {
	type Error = T::Error;

	#[inline(always)]
	fn encode(&self, stream: &mut OStream) -> Result<(), Self::Error> {
		T::encode(self, stream)
	}
}

impl<T: Copy + Encode> Encode for Cell<T> {
	type Error = T::Error;

	#[inline(always)]
	fn encode(&self, stream: &mut OStream) -> Result<(), Self::Error> {
		self.get().encode(stream)
	}
}

impl Encode for char {
	type Error = <u32 as Encode>::Error;

	#[inline(always)]
	fn encode(&self, stream: &mut OStream) -> Result<(), Self::Error> {
		u32::from(*self).encode(stream)
	}
}

#[cfg(feature = "alloc")]
#[cfg_attr(doc, doc(cfg(feature = "alloc")))]
impl<T: Encode + ?Sized + ToOwned> Encode for Cow<'_, T> {
	type Error = T::Error;

	#[inline(always)]
	fn encode(&self, stream: &mut OStream) -> Result<(), Self::Error> {
		T::encode(self, stream)
	}
}

impl Encode for CStr {
	type Error = <[u8] as Encode>::Error;

	/// Encodes the string identically to [a byte slice](slice) containing the string's byte values **excluding** the null terminator.
	#[inline(always)]
	fn encode(&self, stream: &mut OStream) -> Result<(), Self::Error> {
		self.to_bytes().encode(stream)
	}
}

#[cfg(feature = "alloc")]
#[cfg_attr(doc, doc(cfg(feature = "alloc")))]
impl Encode for CString {
	type Error = <CStr as Encode>::Error;

	/// See the the implementation of [`CStr`].
	#[inline(always)]
	fn encode(&self, stream: &mut OStream) -> Result<(), Self::Error> {
		self.as_c_str().encode(stream)
	}
}

impl Encode for Duration {
	type Error = Infallible;

	/// Encodes the duration's seconds and nanoseconds counters sequentially.
	#[inline(always)]
	fn encode(&self, stream: &mut OStream) -> Result<(), Self::Error> {
		self.as_secs().encode(stream).unwrap();
		self.subsec_nanos().encode(stream).unwrap();

		Ok(())
	}
}

#[cfg(feature = "std")]
#[cfg_attr(doc, doc(cfg(feature = "std")))]
impl<K, V, S, E> Encode for HashMap<K, V, S>
where
	K: Encode<Error = E>,
	V: Encode<Error = E>,
	S: BuildHasher,
{
	type Error = E;

	#[inline(always)]
	fn encode(&self, stream: &mut OStream) -> Result<(), Self::Error> {
		for (key, value) in self {
			key.encode(stream)?;
			value.encode(stream)?;
		}

		Ok(())
	}
}

#[cfg(feature = "std")]
#[cfg_attr(doc, doc(cfg(feature = "std")))]
impl<K, S> Encode for HashSet<K, S>
where
	K: Encode,
	S: BuildHasher,
{
	type Error = K::Error;

	#[inline(always)]
	fn encode(&self, stream: &mut OStream) -> Result<(), Self::Error> {
		for key in self {
			key.encode(stream)?;
		}

		Ok(())
	}
}

// Especially useful for `Result<T, Infallible>`.
// **If** that is even needed, of course.
impl Encode for Infallible {
	type Error = Self;

	#[inline(always)]
	fn encode(&self, _stream: &mut OStream) -> Result<(), Self::Error> {
		// SAFETY: `Infallible` can **never** be construct-
		// ed.
		unsafe { unreachable_unchecked() }
	}
}

impl Encode for IpAddr {
	type Error = EnumEncodeError<u8, Infallible>;

	/// Encodes a the address with a preceding discriminant denoting the IP version of the address (i.e. `4` for IPv4 and `6` for IPv6).
	///
	/// See also the implementations of [`Ipv4Addr`] and [`Ipv6Addr`].
	#[inline(always)]
	fn encode(&self, stream: &mut OStream) -> Result<(), Self::Error> {
		// The discriminant here is the IP version.

		match *self {
			Self::V4(ref addr) => {
				0x4u8.encode(stream).map_err(EnumEncodeError::Discriminant)?;
				addr.encode(stream).map_err(EnumEncodeError::Field)?;
			}

			Self::V6(ref addr) => {
				0x6u8.encode(stream).map_err(EnumEncodeError::Discriminant)?;
				addr.encode(stream).map_err(EnumEncodeError::Field)?;
			}
		}

		Ok(())
	}
}

impl Encode for Ipv4Addr {
	type Error = Infallible;

	/// Encodes the address's bits in big-endian.
	#[inline(always)]
	fn encode(&self, stream: &mut OStream) -> Result<(), Self::Error> {
		let value = self.to_bits();
		value.encode(stream)
	}
}

impl Encode for Ipv6Addr {
	type Error = Infallible;

	/// Encodes the address's bits in big-endian.
	#[inline(always)]
	fn encode(&self, stream: &mut OStream) -> Result<(), Self::Error> {
		let value = self.to_bits();
		value.encode(stream)
	}
}

impl Encode for isize {
	type Error = IsizeEncodeError;

	/// Casts `self` to [`i16`] and encodes the result.
	#[inline]
	fn encode(&self, stream: &mut OStream) -> Result<(), Self::Error> {
		let value = i16::try_from(*self)
			.map_err(|_| IsizeEncodeError(*self))?;

		value.encode(stream).unwrap();
		Ok(())
	}
}

impl<T: Encode> Encode for LazyCell<T> {
	type Error = T::Error;

	#[inline(always)]
	fn encode(&self, stream: &mut OStream) -> Result<(), Self::Error> {
		T::encode(self, stream)
	}
}

#[cfg(feature = "std")]
#[cfg_attr(doc, doc(cfg(feature = "std")))]
impl<T: Encode> Encode for LazyLock<T> {
	type Error = T::Error;

	#[inline(always)]
	fn encode(&self, stream: &mut OStream) -> Result<(), Self::Error> {
		T::encode(self, stream)
	}
}

#[cfg(feature = "alloc")]
#[cfg_attr(doc, doc(cfg(feature = "alloc")))]
impl<T: Encode<Error = E>, E> Encode for LinkedList<T> {
	type Error = CollectionEncodeError<UsizeEncodeError, (usize, E)>;

	#[inline(always)]
	fn encode(&self, stream: &mut OStream) -> Result<(), Self::Error> {
		self
			.len()
			.encode(stream)
			.map_err(CollectionEncodeError::Length)?;

		for (i, v) in self.iter().enumerate() {
			v
				.encode(stream)
				.map_err(|e| CollectionEncodeError::Item((i, e)))?;
		}

		Ok(())
	}
}

#[cfg(feature = "std")]
#[cfg_attr(doc, doc(cfg(feature = "std")))]
impl<T: Encode + ?Sized> Encode for Mutex<T> {
	type Error = T::Error;

	#[inline(always)]
	fn encode(&self, stream: &mut OStream) -> Result<(), Self::Error> {
		self
			.lock()
			.unwrap_or_else(std::sync::PoisonError::into_inner)
			.encode(stream)
	}
}

impl<T: Encode> Encode for Option<T> {
	type Error = T::Error;

	/// Encodes a sign denoting the optional's variant.
	/// This is `false` for `None` instances and `true` for `Some` instances.
	///
	/// If `Some`, then the contained value is encoded after this sign..
	fn encode(&self, stream: &mut OStream) -> Result<(), Self::Error> {
		match *self {
			None => {
				false.encode(stream).unwrap();
			}

			Some(ref v) => {
				true.encode(stream).unwrap();
				v.encode(stream)?;
			}
		};

		Ok(())
	}
}

impl<T> Encode for PhantomData<T> {
	type Error = Infallible;

	#[inline(always)]
	fn encode(&self, _stream: &mut OStream) -> Result<(), Self::Error> {
		Ok(())
	}
}

impl Encode for PhantomPinned {
	type Error = Infallible;

	#[inline(always)]
	fn encode(&self, _stream: &mut OStream) -> Result<(), Self::Error> {
		Ok(())
	}
}

impl<T: Encode> Encode for Range<T> {
	type Error = T::Error;

	#[inline(always)]
	fn encode(&self, stream: &mut OStream) -> Result<(), Self::Error> {
		self.start.encode(stream)?;
		self.end.encode(stream)?;

		Ok(())
	}
}

impl<T: Encode> Encode for RangeFrom<T> {
	type Error = T::Error;

	#[inline(always)]
	fn encode(&self, stream: &mut OStream) -> Result<(), Self::Error> {
		self.start.encode(stream)
	}
}

impl Encode for RangeFull {
	type Error = Infallible;

	#[inline(always)]
	fn encode(&self, _stream: &mut OStream) -> Result<(), Self::Error> {
		Ok(())
	}
}

impl<T: Encode> Encode for RangeInclusive<T> {
	type Error = T::Error;

	#[inline(always)]
	fn encode(&self, stream: &mut OStream) -> Result<(), Self::Error> {
		self.start().encode(stream)?;
		self.end().encode(stream)?;

		Ok(())
	}
}

impl<T: Encode> Encode for RangeTo<T> {
	type Error = T::Error;

	#[inline(always)]
	fn encode(&self, stream: &mut OStream) -> Result<(), Self::Error> {
		self.end.encode(stream)
	}
}

impl<T: Encode> Encode for RangeToInclusive<T> {
	type Error = T::Error;

	#[inline(always)]
	fn encode(&self, stream: &mut OStream) -> Result<(), Self::Error> {
		self.end.encode(stream)?;

		Ok(())
	}
}

#[cfg(feature = "alloc")]
#[cfg_attr(doc, doc(cfg(feature = "alloc")))]
impl<T: Encode + ?Sized> Encode for Rc<T> {
	type Error = T::Error;

	#[inline(always)]
	fn encode(&self, stream: &mut OStream) -> Result<(), Self::Error> {
		T::encode(self, stream)
	}
}

impl<T: Encode + ?Sized> Encode for RefCell<T> {
	type Error = RefCellEncodeError<T::Error>;

	#[inline(always)]
	fn encode(&self, stream: &mut OStream) -> Result<(), Self::Error> {
		let value = self.try_borrow()
			.map_err(RefCellEncodeError::Borrow)?;

		T::encode(&value, stream)
			.map_err(RefCellEncodeError::Value)?;

		Ok(())
	}
}

impl<T, E, Err> Encode for core::result::Result<T, E>
where
	T: Encode<Error = Err>,
	E: Encode<Error = Err>,
{
	type Error = Err;

	/// Encodes a sign denoting the result's variant.
	/// This is `false` for `Ok` instances and `true` for `Err` instances.
	///
	/// If `Ok`, then the contained value is encoded after this sign.
	#[inline]
	fn encode(&self, stream: &mut OStream) -> Result<(), Self::Error> {
		// The sign here is `false` for `Ok` objects and
		// `true` for `Err` objects.

		match *self {
			Ok(ref v) => {
				false.encode(stream).unwrap();
				v.encode(stream)?;
			}

			Err(ref e) => {
				true.encode(stream).unwrap();
				e.encode(stream)?;
			}
		};

		Ok(())
	}
}

#[cfg(feature = "std")]
#[cfg_attr(doc, doc(cfg(feature = "std")))]
impl<T: Encode + ?Sized> Encode for RwLock<T> {
	type Error = T::Error;

	#[inline(always)]
	fn encode(&self, stream: &mut OStream) -> Result<(), Self::Error> {
		self
			.read()
			.or_else(|e| Ok(e.into_inner()))?
			.encode(stream)
	}
}

impl<T: Encode> Encode for Saturating<T> {
	type Error = T::Error;

	#[inline(always)]
	fn encode(&self, stream: &mut OStream) -> Result<(), Self::Error> {
		self.0.encode(stream)
	}
}

impl Encode for SocketAddr {
	type Error = Infallible;

	/// This implementation encoded as discriminant denoting the IP version of the address (i.e. `4` for IPv4 and `6` for IPv6).
	/// This is then followed by the respective address' own encoding (either [`SocketAddrV4`] or [`SocketAddrV6`]).
	#[inline]
	fn encode(&self, stream: &mut OStream) -> Result<(), Self::Error> {
		// The discriminant here is the IP version.

		match *self {
			Self::V4(ref addr) => {
				0x4u8.encode(stream)?;
				addr.encode(stream)?;
			}

			Self::V6(ref addr) => {
				0x6u8.encode(stream)?;
				addr.encode(stream)?;
			}
		}

		Ok(())
	}
}

impl Encode for SocketAddrV4 {
	type Error = Infallible;

	/// Encodes the address's bits followed by the port number, both of which in big-endian.
	#[inline(always)]
	fn encode(&self, stream: &mut OStream) -> Result<(), Self::Error> {
		self.ip().encode(stream)?;
		self.port().encode(stream)?;

		Ok(())
	}
}

impl Encode for SocketAddrV6 {
	type Error = Infallible;

	/// Encodes the address's bits followed by the port number, flow information, and scope identifier -- all of which in big-endian.
	#[inline(always)]
	fn encode(&self, stream: &mut OStream) -> Result<(), Self::Error> {
		self.ip().encode(stream)?;
		self.port().encode(stream)?;
		self.flowinfo().encode(stream)?;
		self.scope_id().encode(stream)?;

		Ok(())
	}
}

impl Encode for str {
	type Error = <[u8] as Encode>::Error;

	/// Encodes the string identically to [a byte slice](slice) containing the string's byte values.
	#[inline(always)]
	fn encode(&self, stream: &mut OStream) -> Result<(), Self::Error> {
		self.as_bytes().encode(stream)
	}
}

#[cfg(feature = "alloc")]
#[cfg_attr(doc, doc(cfg(feature = "alloc")))]
impl Encode for String {
	type Error = <str as Encode>::Error;

	/// See [`str`].
	#[inline(always)]
	fn encode(&self, stream: &mut OStream) -> Result<(), Self::Error> {
		self.as_str().encode(stream)
	}
}

#[cfg(feature = "std")]
#[cfg_attr(doc, doc(cfg(feature = "std")))]
impl Encode for SystemTime {
	type Error = Infallible;

	/// Encodes the time point as the nearest, signed UNIX timestamp.
	///
	/// Examples of some timestamps and their encodings include:
	///
	/// | ISO 8601                    | UNIX / Librum |
	/// | :-------------------------- | -------------: |
	/// | `2024-11-03T12:02:01+01:00` |    +1730631721 |
	/// | `1989-06-03T20:00:00+09:00` |      +13258800 |
	/// | `1970-01-01T00:00:00Z`      |             +0 |
	/// | `1945-05-04T18:30:00+02:00` |     -778231800 |
	#[expect(clippy::cast_possible_wrap)]
	#[inline]
	fn encode(&self, stream: &mut OStream) -> Result<(), Self::Error> {
		let time = if *self >= UNIX_EPOCH {
			let duration = self
				.duration_since(UNIX_EPOCH)
				.expect("cannot compute duration since the epoch");

				duration.as_secs() as i64
		} else {
			let duration = UNIX_EPOCH
				.duration_since(*self)
				.expect("cannot compute duration until the epoch");

			0x0 - duration.as_secs() as i64
		};

		time.encode(stream).unwrap();
		Ok(())
	}
}

impl Encode for () {
	type Error = Infallible;

	#[inline(always)]
	fn encode(&self, _stream: &mut OStream) -> Result<(), Self::Error> {
		Ok(())
	}
}

impl Encode for usize {
	type Error = UsizeEncodeError;

	/// Casts `self` to [`u16`] and encodes the result.
	#[inline]
	fn encode(&self, stream: &mut OStream) -> Result<(), Self::Error> {
		let value = u16::try_from(*self)
			.map_err(|_| UsizeEncodeError(*self))?;

		value.encode(stream).unwrap();
		Ok(())
	}
}

#[cfg(feature = "alloc")]
#[cfg_attr(doc, doc(cfg(feature = "alloc")))]
impl<T: Encode> Encode for Vec<T> {
	type Error = <[T] as Encode>::Error;

	#[inline(always)]
	fn encode(&self, stream: &mut OStream) -> Result<(), Self::Error> {
		self.as_slice().encode(stream)
	}
}

impl<T: Encode> Encode for Wrapping<T> {
	type Error = T::Error;

	#[inline(always)]
	fn encode(&self, stream: &mut OStream) -> Result<(), Self::Error> {
		self.0.encode(stream)
	}
}

macro_rules! impl_numeric {
	($ty:ty$(,)?) => {
		impl ::librum::Encode for $ty {
			type Error = ::core::convert::Infallible;

			#[inline]
			fn encode(&self, stream: &mut OStream) -> ::core::result::Result<(), Self::Error> {
				stream.write(&self.to_be_bytes());

				Ok(())
			}
		}
	};
}

macro_rules! impl_tuple {
	{
		$($captures:ident: $tys:ident),+$(,)?
	} => {
		#[doc(hidden)]
		impl<$($tys, )* E> ::librum::Encode for ($($tys, )*)
		where
			$($tys: Encode<Error = E>, )* {
			type Error = E;

			#[inline(always)]
			fn encode(&self, stream: &mut ::librum::OStream) -> ::core::result::Result<(), Self::Error> {
				let ($(ref $captures, )*) = *self;

				$(
					$captures.encode(stream)?;
				)*

				Ok(())
			}
		}
	};
}

macro_rules! impl_non_zero {
	($ty:ty$(,)?) => {
		impl ::librum::Encode for ::core::num::NonZero<$ty> {
			type Error = <$ty as ::librum::Encode>::Error;

			#[inline(always)]
			fn encode(&self, stream: &mut OStream) -> ::core::result::Result<(), Self::Error> {
				self.get().encode(stream)
			}
		}
	};
}

macro_rules! impl_atomic {
	{
		width: $width:literal,
		ty: $ty:ty,
		atomic_ty: $atomic_ty:ty$(,)?
	} => {
		#[cfg(target_has_atomic = $width)]
		#[cfg_attr(doc, doc(cfg(target_has_atomic = $width)))]
		impl ::librum::Encode for $atomic_ty {
			type Error = <$ty as ::librum::Encode>::Error;

			/// Encodes the atomic with the same scheme as that of the atomic type's primitive counterpart.
			///
			/// The atomic object itself is read with the [`Relaxed`](core::sync::atomic::Ordering) ordering scheme.
			#[inline(always)]
			fn encode(&self, stream: &mut ::librum::OStream) -> ::core::result::Result<(), Self::Error> {
				self.load(::std::sync::atomic::Ordering::Relaxed).encode(stream)
			}
		}
	};
}

//impl_numeric!(f128);
//impl_numeric!(f16);
impl_numeric!(f32);
impl_numeric!(f64);
impl_numeric!(i128);
impl_numeric!(i16);
impl_numeric!(i32);
impl_numeric!(i64);
impl_numeric!(i8);
impl_numeric!(u128);
impl_numeric!(u16);
impl_numeric!(u32);
impl_numeric!(u64);
impl_numeric!(u8);

impl_tuple! {
	value0: T0,
	value1: T1,
}

impl_tuple! {
	value0: T0,
	value1: T1,
	value2: T2,
}

impl_tuple! {
	value0: T0,
	value1: T1,
	value2: T2,
	value3: T3,
}

impl_tuple! {
	value0: T0,
	value1: T1,
	value2: T2,
	value3: T3,
	value4: T4,
}

impl_tuple! {
	value0: T0,
	value1: T1,
	value2: T2,
	value3: T3,
	value4: T4,
	value5: T5,
}

impl_tuple! {
	value0: T0,
	value1: T1,
	value2: T2,
	value3: T3,
	value4: T4,
	value5: T5,
	value6: T6,
}

impl_tuple! {
	value0: T0,
	value1: T1,
	value2: T2,
	value3: T3,
	value4: T4,
	value5: T5,
	value6: T6,
	value7: T7,
}

impl_tuple! {
	value0: T0,
	value1: T1,
	value2: T2,
	value3: T3,
	value4: T4,
	value5: T5,
	value6: T6,
	value7: T7,
	value8: T8,
}

impl_tuple! {
	value0: T0,
	value1: T1,
	value2: T2,
	value3: T3,
	value4: T4,
	value5: T5,
	value6: T6,
	value7: T7,
	value8: T8,
	value9: T9,
}

impl_tuple! {
	value0:  T0,
	value1:  T1,
	value2:  T2,
	value3:  T3,
	value4:  T4,
	value5:  T5,
	value6:  T6,
	value7:  T7,
	value8:  T8,
	value9:  T9,
	value10: T10,
}

impl_tuple! {
	value0:  T0,
	value1:  T1,
	value2:  T2,
	value3:  T3,
	value4:  T4,
	value5:  T5,
	value6:  T6,
	value7:  T7,
	value8:  T8,
	value9:  T9,
	value10: T10,
	value11: T11,
}

impl_non_zero!(i128);
impl_non_zero!(i16);
impl_non_zero!(i32);
impl_non_zero!(i64);
impl_non_zero!(i8);
impl_non_zero!(isize);
impl_non_zero!(u128);
impl_non_zero!(u16);
impl_non_zero!(u32);
impl_non_zero!(u64);
impl_non_zero!(u8);
impl_non_zero!(usize);

impl_atomic! {
	width: "8",
	ty: bool,
	atomic_ty: std::sync::atomic::AtomicBool,
}

impl_atomic! {
	width: "16",
	ty: i16,
	atomic_ty: std::sync::atomic::AtomicI16,
}

impl_atomic! {
	width: "32",
	ty: i32,
	atomic_ty: std::sync::atomic::AtomicI32,
}

impl_atomic! {
	width: "64",
	ty: i64,
	atomic_ty: std::sync::atomic::AtomicI64,
}

impl_atomic! {
	width: "8",
	ty: i8,
	atomic_ty: std::sync::atomic::AtomicI8,
}

impl_atomic! {
	width: "ptr",
	ty: isize,
	atomic_ty: std::sync::atomic::AtomicIsize,
}

impl_atomic! {
	width: "16",
	ty: u16,
	atomic_ty: std::sync::atomic::AtomicU16,
}

impl_atomic! {
	width: "32",
	ty: u32,
	atomic_ty: std::sync::atomic::AtomicU32,
}

impl_atomic! {
	width: "64",
	ty: u64,
	atomic_ty: std::sync::atomic::AtomicU64,
}

impl_atomic! {
	width: "8",
	ty: u8,
	atomic_ty: std::sync::atomic::AtomicU8,
}

impl_atomic! {
	width: "ptr",
	ty: usize,
	atomic_ty: std::sync::atomic::AtomicUsize,
}