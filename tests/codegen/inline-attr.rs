// Checks that the inlining function attributes are emitted on functions
//
// compile-flags: -C opt-level=0 -Cno-prepopulate-passes

#![crate_type = "lib"]
#![feature(inline_always_mir)]

// CHECK-LABEL: ; inline_attr::no_attr
// CHECK-NEXT: Function Attrs:
// CHECK-NOT: inline
// CHECK: define void @
// CHECK-SAME: no_attr
pub fn no_attr() {}

// CHECK-LABEL: ; inline_attr::empty_attr
// CHECK-NEXT: Function Attrs:
// CHECK-SAME: inlinehint
#[inline]
pub fn empty_attr() {}

// CHECK-LABEL: ; inline_attr::attr_always_mir
// CHECK-NEXT: Function Attrs:
// CHECK-SAME: inlinehint
#[inline(always_mir)]
pub fn attr_always_mir() {}

// CHECK-LABEL: ; inline_attr::attr_always
// CHECK-NEXT: Function Attrs:
// CHECK-SAME: alwaysinline
#[inline(always)]
pub fn attr_always() {}

// CHECK-LABEL: ; inline_attr::attr_never
// CHECK-NEXT: Function Attrs:
// CHECK-SAME: noinline
#[inline(never)]
pub fn attr_never() {}

// CHECK-LABEL: force_everything_to_be_emitted
pub fn force_everything_to_be_emitted() {
	no_attr();
	empty_attr();
	attr_always_mir();
	attr_never();
	attr_always();
}
