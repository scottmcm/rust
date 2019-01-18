/// A trait for customizing the behavior of the `?` operator.
///
/// A type implementing `Try` is one that has a canonical way to view it
/// in terms of a success/failure dichotomy.  This trait allows both
/// extracting those success or failure values from an existing instance and
/// creating a new instance from a success or failure value.
#[unstable(feature = "try_trait", issue = "42327")]
#[rustc_on_unimplemented(
   on(all(
       any(from_method="from_error", from_method="from_ok"),
       from_desugaring="?"),
      message="the `?` operator can only be used in a \
               function that returns `Result` or `Option` \
               (or another type that implements `{Try}`)",
      label="cannot use the `?` operator in a function that returns `{Self}`"),
   on(all(from_method="into_result", from_desugaring="?"),
      message="the `?` operator can only be applied to values \
               that implement `{Try}`",
      label="the `?` operator cannot be applied to type `{Self}`")
)]
#[doc(alias = "?")]
pub trait Try {
    /// The type of this value when viewed as successful.
    #[unstable(feature = "try_trait", issue = "42327")]
    type Ok;
    /// The type of this value when viewed as failed.
    #[unstable(feature = "try_trait", issue = "42327")]
    type Error;

    /// Applies the "?" operator. A return of `Ok(t)` means that the
    /// execution should continue normally, and the result of `?` is the
    /// value `t`. A return of `Err(e)` means that execution should branch
    /// to the innermost enclosing `catch`, or return from the function.
    ///
    /// If an `Err(e)` result is returned, the value `e` will be "wrapped"
    /// in the return type of the enclosing scope (which must itself implement
    /// `Try`). Specifically, the value `X::from_error(From::from(e))`
    /// is returned, where `X` is the return type of the enclosing function.
    #[unstable(feature = "try_trait", issue = "42327")]
    fn into_result(self) -> Result<Self::Ok, Self::Error>;

    /// Wrap an error value to construct the composite result. For example,
    /// `Result::Err(x)` and `Result::from_error(x)` are equivalent.
    #[unstable(feature = "try_trait", issue = "42327")]
    fn from_error(v: Self::Error) -> Self;

    /// Wrap an OK value to construct the composite result. For example,
    /// `Result::Ok(x)` and `Result::from_ok(x)` are equivalent.
    #[unstable(feature = "try_trait", issue = "42327")]
    fn from_ok(v: Self::Ok) -> Self;
}

/// Still needs docs
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[unstable(feature = "try_trait_v2", issue = "42327")]
pub enum ControlFlow<C, B> {
    /// Still needs docs
    #[unstable(feature = "try_trait_v2", issue = "42327")]
    Continue(C),
    /// Still needs docs
    #[unstable(feature = "try_trait_v2", issue = "42327")]
    Break(B),
}

#[unstable(feature = "try_trait_v2", issue = "42327")]
impl<C, B> ControlFlow<C, B> {
    /// Still needs docs
    #[unstable(feature = "try_trait_v2", issue = "42327")]
    #[inline]
    pub fn break_value(self) -> Option<B> {
        match self {
            ControlFlow::Break(x) => Some(x),
            _ => None,
        }
    }
    /// Still needs docs
    #[unstable(feature = "try_trait_v2", issue = "42327")]
    #[inline]
    pub fn continue_value(self) -> Option<C> {
        match self {
            ControlFlow::Continue(x) => Some(x),
            _ => None,
        }
    }
}

#[unstable(feature = "try_trait_v2", issue = "42327")]
impl<R: TryBlock> ControlFlow<R::Inner, R> {
    /// Still needs docs
    #[unstable(feature = "try_trait_v2", issue = "42327")]
    #[inline]
    pub fn unbubble(self) -> R {
        match self {
            ControlFlow::Continue(x) => TryBlock::done(x),
            ControlFlow::Break(x) => x,
        }
    }
}

#[unstable(feature = "try_trait_v2", issue = "42327")]
impl<C, B> Try for ControlFlow<C, B> {
    type Ok = C;
    type Error = B;
    #[inline]
    fn into_result(self) -> Result<Self::Ok, Self::Error> {
        match self {
            ControlFlow::Continue(y) => Ok(y),
            ControlFlow::Break(x) => Err(x),
        }
    }
    #[inline]
    fn from_error(v: Self::Error) -> Self { ControlFlow::Break(v) }
    #[inline]
    fn from_ok(v: Self::Ok) -> Self { ControlFlow::Continue(v) }
}

/// Still needs docs
#[unstable(feature = "try_trait_v2", issue = "42327")]
#[doc(alias = "try")]
pub trait TryBlock {
    /// Still needs docs
    type Inner;
    /// Still needs docs
    fn done(inner: Self::Inner) -> Self;
}

#[unstable(feature = "try_trait_v2", issue = "42327")]
#[doc(alias = "?")]
/// Still needs docs
pub trait Bubble<T = Self> : TryBlock + Try<Ok=<Self as TryBlock>::Inner> {
    /// Still needs docs
    #[unstable(feature = "try_trait_v2", issue = "42327")]
    fn bubble(self) -> ControlFlow<Self::Inner, T>;
}

/*
When the lowering is updated...

#[unstable(feature = "try_trait_v2", issue = "42327")]
#[doc(alias = "?")]
/// Still needs docs
pub trait Bubble<T = Self> : TryBlock {
    /// Still needs docs
    #[unstable(feature = "try_trait_v2", issue = "42327")]
    fn bubble(self) -> ControlFlow<Self::Inner, T>;
}
*/

#[unstable(feature = "try_trait_v2", issue = "42327")]
impl<C, B> TryBlock for ControlFlow<C, B> {
    type Inner = C;
    fn done(inner: Self::Inner) -> Self {
        ControlFlow::Continue(inner)
    }
}

#[unstable(feature = "try_trait_v2", issue = "42327")]
impl<C, B> Bubble for ControlFlow<C, B> {
    fn bubble(self) -> ControlFlow<Self::Inner, Self> {
        match self {
            ControlFlow::Continue(x) => ControlFlow::Continue(x),
            x => ControlFlow::Break(x),
        }
    }
}
