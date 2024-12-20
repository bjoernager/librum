// Copyright 2024 Gabriel Bjørnager Jensen.
//
// This file is part of Oct.
//
// Oct is free software: you can redistribute it
// and/or modify it under the terms of the GNU
// Lesser General Public License as published by
// the Free Software Foundation, either version 3
// of the License, or (at your option) any later
// version.
//
// Oct is distributed in the hope that it will be
// useful, but WITHOUT ANY WARRANTY; without even
// the implied warranty of MERCHANTABILITY or FIT-
// NESS FOR A PARTICULAR PURPOSE. See the GNU Less-
// er General Public License for more details.
//
// You should have received a copy of the GNU Less-
// er General Public License along with Oct. If
// not, see <https://www.gnu.org/licenses/>.

use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use std::iter;
use syn::{
	Attribute,
	Ident,
	Path,
	PathSegment,
	Type,
	TypePath,
};

/// A derivable enumeration representation.
///
/// Any type can, *in theory*, be used as a discriminant.
/// This type, however, only includes primitives.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Repr {
	U8,
	I8,
	U16,
	I16,
	U32,
	I32,
	U64,
	I64,
	U128,
	I128,
	Usize,
	Isize,
}

impl Repr {
	#[inline]
	#[must_use]
	pub fn get(attrs: &[Attribute]) -> Option<Self> {
		let mut this = None;

		for attr in attrs {
			if attr.path().is_ident("repr") {
				attr.parse_nested_meta(|meta| {
					let ident = meta.path.require_ident()?;

					if      ident == "u8"    { this = Some(Self::U8) }
					else if ident == "i8"    { this = Some(Self::I8) }
					else if ident == "u16"   { this = Some(Self::U16) }
					else if ident == "i16"   { this = Some(Self::I16) }
					else if ident == "u32"   { this = Some(Self::U32) }
					else if ident == "i32"   { this = Some(Self::I32) }
					else if ident == "u64"   { this = Some(Self::U64) }
					else if ident == "i64"   { this = Some(Self::I64) }
					else if ident == "u128"  { this = Some(Self::U128) }
					else if ident == "i128"  { this = Some(Self::I128) }
					else if ident == "usize" { this = Some(Self::Usize) }
					else if ident == "isize" { this = Some(Self::Isize) }
					else                    { panic!("`{ident}` is not a derivable enumeration representation") };

					Ok(())
				}).unwrap();
			}

			// Ignore all other attributes.
		}

		this
	}

	#[inline]
	#[must_use]
	pub const fn to_str(self) -> &'static str {
		match self {
			Self::U8    => "u8",
			Self::I8    => "i8",
			Self::U16   => "u16",
			Self::I16   => "i16",
			Self::U32   => "u32",
			Self::I32   => "i32",
			Self::U64   => "u64",
			Self::I64   => "i64",
			Self::U128  => "u128",
			Self::I128  => "i128",
			Self::Usize => "usize",
			Self::Isize => "isize",
		}
	}

	#[inline(always)]
	#[must_use]
	pub fn to_ident(self, span: Span) -> Ident {
		let ident = self.to_str();

		Ident::new(ident, span)
	}

	#[inline(always)]
	#[must_use]
	pub fn to_path(self, span: Span) -> Path {
		let ident = self.to_ident(span);

		Path {
			leading_colon: None,
			segments: iter::once(PathSegment {
				ident,
				arguments: Default::default(),
			}).collect(),
		}
	}

	#[inline]
	#[must_use]
	pub fn to_type(self, span: Span) -> Type {
		Type::Path(TypePath {
			qself: None,
			path:  self.to_path(span),
		})
	}
}

impl Default for Repr {
	#[inline(always)]
	fn default() -> Self {
		Self::Isize
	}
}

impl ToTokens for Repr {
	#[inline(always)]
	fn to_tokens(&self, tokens: &mut TokenStream) {
		self.to_ident(Span::call_site()).to_tokens(tokens);
	}
}
