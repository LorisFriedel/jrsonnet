use jrsonnet_gcmodule::{Cc, Trace};
use jrsonnet_interner::IBytes;
use jrsonnet_parser::LocExpr;

use crate::{function::FuncVal, Context, Result, Thunk, Val};

mod spec;
use spec::*;

/// Represents a Jsonnet array value.
#[derive(Debug, Clone, Trace)]
// may contrain other ArrValue
#[trace(tracking(force))]
pub enum ArrValue {
	/// Layout optimized byte array.
	Bytes(BytesArray),
	/// Every element is lazy evaluated.
	Lazy(LazyArray),
	/// Every element is defined somewhere in source code
	Expr(ExprArray),
	/// Every field is already evaluated.
	Eager(EagerArray),
	/// Concatenation of two arrays of any kind.
	Extended(Cc<ExtendedArray>),
	/// Represents a integer array in form `[start, start + 1, ... end - 1, end]`.
	/// This kind of arrays is generated by `std.range(start, end)` call, and used for loops.
	Range(RangeArray),
	/// Sliced array view.
	Slice(Cc<SliceArray>),
	/// Reversed array view.
	/// Returned by `std.reverse(other)` call
	Reverse(Cc<ReverseArray>),
	/// Returned by `std.map` call
	Mapped(MappedArray),
	/// Returned by `std.repeat` call
	Repeated(RepeatedArray),
}

pub trait ArrayLikeIter<T>: Iterator<Item = T> + DoubleEndedIterator + ExactSizeIterator {}
impl<I, T> ArrayLikeIter<T> for I where
	I: Iterator<Item = T> + DoubleEndedIterator + ExactSizeIterator
{
}

impl ArrValue {
	pub fn empty() -> Self {
		Self::Range(RangeArray::empty())
	}

	pub fn expr(ctx: Context, exprs: impl IntoIterator<Item = LocExpr>) -> Self {
		Self::Expr(ExprArray::new(ctx, exprs))
	}

	pub fn lazy(thunks: Cc<Vec<Thunk<Val>>>) -> Self {
		Self::Lazy(LazyArray(thunks))
	}

	pub fn eager(values: Cc<Vec<Val>>) -> Self {
		Self::Eager(EagerArray(values))
	}

	pub fn repeated(data: ArrValue, repeats: usize) -> Option<Self> {
		Some(Self::Repeated(RepeatedArray::new(data, repeats)?))
	}

	pub fn bytes(bytes: IBytes) -> Self {
		Self::Bytes(BytesArray(bytes))
	}

	#[must_use]
	pub fn map(self, mapper: FuncVal) -> Self {
		Self::Mapped(MappedArray::new(self, mapper))
	}

	pub fn filter(self, filter: impl Fn(&Val) -> Result<bool>) -> Result<Self> {
		// TODO: ArrValue::Picked(inner, indexes) for large arrays
		let mut out = Vec::new();
		for i in self.iter() {
			let i = i?;
			if filter(&i)? {
				out.push(i);
			};
		}
		Ok(Self::eager(Cc::new(out)))
	}

	pub fn extended(a: ArrValue, b: ArrValue) -> Self {
		// TODO: benchmark for an optimal value, currently just a arbitrary choice
		const ARR_EXTEND_THRESHOLD: usize = 100;

		if a.is_empty() {
			b
		} else if b.is_empty() {
			a
		} else if a.len() + b.len() > ARR_EXTEND_THRESHOLD {
			Self::Extended(Cc::new(ExtendedArray::new(a, b)))
		} else if let (Some(a), Some(b)) = (a.iter_cheap(), b.iter_cheap()) {
			let mut out = Vec::with_capacity(a.len() + b.len());
			out.extend(a);
			out.extend(b);
			Self::eager(Cc::new(out))
		} else {
			let mut out = Vec::with_capacity(a.len() + b.len());
			out.extend(a.iter_lazy());
			out.extend(b.iter_lazy());
			Self::lazy(Cc::new(out))
		}
	}

	pub fn range_exclusive(a: i32, b: i32) -> Self {
		Self::Range(RangeArray::new_exclusive(a, b))
	}
	pub fn range_inclusive(a: i32, b: i32) -> Self {
		Self::Range(RangeArray::new_inclusive(a, b))
	}

	#[must_use]
	pub fn slice(
		self,
		from: Option<usize>,
		to: Option<usize>,
		step: Option<usize>,
	) -> Option<Self> {
		let len = self.len();
		let from = from.unwrap_or(0);
		let to = to.unwrap_or(len).min(len);
		let step = step.unwrap_or(1);
		if from >= to || step == 0 {
			return None;
		}

		Some(Self::Slice(Cc::new(SliceArray {
			inner: self,
			from: from as u32,
			to: to as u32,
			step: step as u32,
		})))
	}

	/// Array length.
	pub fn len(&self) -> usize {
		pass!(self.len())
	}

	/// Is array contains no elements?
	pub fn is_empty(&self) -> bool {
		pass!(self.is_empty())
	}

	/// Get array element by index, evaluating it, if it is lazy.
	///
	/// Returns `None` on out-of-bounds condition.
	pub fn get(&self, index: usize) -> Result<Option<Val>> {
		pass!(self.get(index))
	}

	/// Returns None if get is either non cheap, or out of bounds
	fn get_cheap(&self, index: usize) -> Option<Val> {
		pass!(self.get_cheap(index))
	}

	/// Get array element by index, without evaluation.
	///
	/// Returns `None` on out-of-bounds condition.
	pub fn get_lazy(&self, index: usize) -> Option<Thunk<Val>> {
		pass!(self.get_lazy(index))
	}

	#[cfg(feature = "nightly")]
	pub fn iter(&self) -> UnknownArrayIter<'_> {
		pass_iter_call!(self.iter => UnknownArrayIter)
	}
	#[cfg(not(feature = "nightly"))]
	pub fn iter(&self) -> impl ArrayLikeIter<Result<Val>> + '_ {
		(0..self.len()).map(|i| self.get(i).transpose().expect("length checked"))
	}

	/// Iterate over elements, returning lazy values.
	#[cfg(feature = "nightly")]
	pub fn iter_lazy(&self) -> UnknownArrayIterLazy<'_> {
		pass_iter_call!(self.iter_lazy => UnknownArrayIterLazy)
	}
	#[cfg(not(feature = "nightly"))]
	pub fn iter_lazy(&self) -> impl ArrayLikeIter<Thunk<Val>> + '_ {
		(0..self.len()).map(|i| self.get_lazy(i).expect("length checked"))
	}

	#[cfg(feature = "nightly")]
	pub fn iter_cheap(&self) -> Option<UnknownArrayIterCheap<'_>> {
		macro_rules! question {
			($v:expr) => {
				$v?
			};
		}
		Some(pass_iter_call!(self.iter_cheap in question => UnknownArrayIterCheap))
	}

	#[cfg(not(feature = "nightly"))]
	pub fn iter_cheap(&self) -> Option<impl ArrayLikeIter<Val> + '_> {
		if self.is_cheap() {
			Some((0..self.len()).map(|i| self.get_cheap(i).expect("length and is_cheap checked")))
		} else {
			None
		}
	}

	/// Return a reversed view on current array.
	#[must_use]
	pub fn reversed(self) -> Self {
		Self::Reverse(Cc::new(ReverseArray(self)))
	}

	pub fn ptr_eq(a: &Self, b: &Self) -> bool {
		match (a, b) {
			(ArrValue::Bytes(a), ArrValue::Bytes(b)) => a.0 == b.0,
			(ArrValue::Lazy(a), ArrValue::Lazy(b)) => Cc::ptr_eq(&a.0, &b.0),
			(ArrValue::Expr(a), ArrValue::Expr(b)) => Cc::ptr_eq(&a.0, &b.0),
			(ArrValue::Eager(a), ArrValue::Eager(b)) => Cc::ptr_eq(&a.0, &b.0),
			(ArrValue::Extended(a), ArrValue::Extended(b)) => Cc::ptr_eq(a, b),
			(ArrValue::Range(a), ArrValue::Range(b)) => a == b,
			_ => false,
		}
	}

	pub fn is_cheap(&self) -> bool {
		match self {
			ArrValue::Eager(_) | ArrValue::Range(..) | ArrValue::Bytes(_) => true,
			ArrValue::Extended(v) => v.a.is_cheap() && v.b.is_cheap(),
			ArrValue::Slice(r) => r.inner.is_cheap(),
			ArrValue::Reverse(i) => i.0.is_cheap(),
			ArrValue::Repeated(v) => v.is_cheap(),
			ArrValue::Expr(_) | ArrValue::Lazy(_) | ArrValue::Mapped(_) => false,
		}
	}
}
impl From<Vec<Val>> for ArrValue {
	fn from(value: Vec<Val>) -> Self {
		Self::eager(Cc::new(value))
	}
}
impl From<Vec<Thunk<Val>>> for ArrValue {
	fn from(value: Vec<Thunk<Val>>) -> Self {
		Self::lazy(Cc::new(value))
	}
}

#[cfg(target_pointer_width = "64")]
static_assertions::assert_eq_size!(ArrValue, [u8; 16]);
