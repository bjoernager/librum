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

use core::convert::Infallible;
use core::error::Error;
use core::fmt::{self, Debug, Display, Formatter};

/// A collection's item could not be encoded.
///
/// See also [`CollectionEncodeError`](crate::error::CollectionEncodeError).
#[derive(Debug)]
#[must_use]
pub struct ItemEncodeError<I, E> {
	/// The index of the invalid item.
	pub index: I,

	/// The encoder's error.
	pub error: E,
}

impl<I, E> Display for ItemEncodeError<I, E>
where
	I: Display,
	E: Display,
{
	#[inline(always)]
	fn fmt(&self, f: &mut Formatter) -> fmt::Result {
		write!(f, "could not encode item at `{}`: {}", self.index, self.error)
	}
}

impl<I, E> Error for ItemEncodeError<I, E>
where
	Self: Debug + Display,
	E: Error + 'static,
{
	#[inline(always)]
	fn source(&self) -> Option<&(dyn Error + 'static)> {
		Some(&self.error)
	}
}

impl<I, E: Into<Self>> From<ItemEncodeError<I, E>> for Infallible {
	#[inline(always)]
	fn from(_value: ItemEncodeError<I, E>) -> Self {
		unreachable!()
	}
}
