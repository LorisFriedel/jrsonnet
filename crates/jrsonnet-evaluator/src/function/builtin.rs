use std::{any::Any, borrow::Cow};

use jrsonnet_gcmodule::Trace;

use super::{arglike::ArgsLike, parse::parse_builtin_call, CallLocation};
use crate::{error::Result, gc::TraceBox, tb, Context, Val};

pub type BuiltinParamName = Cow<'static, str>;

#[derive(Clone, Trace)]
pub struct BuiltinParam {
	/// Parameter name for named call parsing
	pub name: Option<BuiltinParamName>,
	/// Is implementation allowed to return empty value
	pub has_default: bool,
}

/// Description of function defined by native code
///
/// Prefer to use #[builtin] macro, instead of manual implementation of this trait
pub trait Builtin: Trace {
	/// Function name to be used in stack traces
	fn name(&self) -> &str;
	/// Parameter names for named calls
	fn params(&self) -> &[BuiltinParam];
	/// Call the builtin
	fn call(&self, ctx: Context, loc: CallLocation<'_>, args: &dyn ArgsLike) -> Result<Val>;

	fn as_any(&self) -> &dyn Any;
}

pub trait StaticBuiltin: Builtin + Send + Sync
where
	Self: 'static,
{
	// In impl, to make it object safe:
	// const INST: &'static Self;
}

#[derive(Trace)]
pub struct NativeCallback {
	pub(crate) params: Vec<BuiltinParam>,
	handler: TraceBox<dyn NativeCallbackHandler>,
}
impl NativeCallback {
	#[deprecated = "prefer using builtins directly, use this interface only for bindings"]
	pub fn new(params: Vec<Cow<'static, str>>, handler: impl NativeCallbackHandler) -> Self {
		Self {
			params: params
				.into_iter()
				.map(|n| BuiltinParam {
					name: Some(n),
					has_default: false,
				})
				.collect(),
			handler: tb!(handler),
		}
	}
}

impl Builtin for NativeCallback {
	fn name(&self) -> &str {
		// TODO: standard natives gets their names from definition
		// But builitins should already have them
		"<native>"
	}

	fn params(&self) -> &[BuiltinParam] {
		&self.params
	}

	fn call(&self, ctx: Context, _loc: CallLocation<'_>, args: &dyn ArgsLike) -> Result<Val> {
		let args = parse_builtin_call(ctx, &self.params, args, true)?;
		let args = args
			.into_iter()
			.map(|a| a.expect("legacy natives have no default params"))
			.map(|a| a.evaluate())
			.collect::<Result<Vec<Val>>>()?;
		self.handler.call(&args)
	}

	fn as_any(&self) -> &dyn Any {
		self
	}
}

pub trait NativeCallbackHandler: Trace {
	fn call(&self, args: &[Val]) -> Result<Val>;
}
