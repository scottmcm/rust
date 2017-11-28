// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Simple const propagation pass.
//!
//! This walks basic blocks, keeping track of assignments like
//!
//!     DEST = const SRC
//!
//! and updating any following uses from
//!
//!     USE(DEST)
//!
//! to
//!
//!     USE(const SRC)
//!
//! This doesn't try to track aliasing, so ignores any borrowed destination.
//! It also assumes someone else will clean up unused locals later, if any.

use rustc::hir;
use rustc::mir::*;
use rustc::mir::visit::{LvalueContext, MutVisitor};
use rustc::ty::TyCtxt;
use rustc_data_structures::control_flow_graph::iterate::reverse_post_order;
use rustc_data_structures::indexed_vec::IndexVec;
use rustc_data_structures::indexed_set::IdxSetBuf;
use transform::{MirPass, MirSource};

pub struct ConstPropagation;

impl MirPass for ConstPropagation {
    fn run_pass<'a, 'tcx>(&self,
                          tcx: TyCtxt<'a, 'tcx, 'tcx>,
                          source: MirSource,
                          mir: &mut Mir<'tcx>) {
        // Don't run on constant MIR, because trans might not be able to
        // evaluate the modified MIR.
        // FIXME(eddyb) Remove check after miri is merged.
        let id = tcx.hir.as_local_node_id(source.def_id).unwrap();
        match (tcx.hir.body_owner_kind(id), source.promoted) {
            (_, Some(_)) |
            (hir::BodyOwnerKind::Const, _) |
            (hir::BodyOwnerKind::Static(_), _) => return,

            (hir::BodyOwnerKind::Fn, _) => {
                if tcx.is_const_fn(source.def_id) {
                    // Don't run on const functions, as, again, trans might not be able to evaluate
                    // the optimized IR.
                    return
                }
            }
        }

        // This is only worth doing when it can help other complex MIR
        // optimization passes, since it happens naturally in SSA.
        if tcx.sess.opts.debugging_opts.mir_opt_level <= 1 {
            return;
        }

        // If there are no const assignments, skip out before we invalidate caches
        if !mir.basic_blocks().iter().any(has_const_assignment) {
            return;
        }

        let predecessors = mir.predecessors().clone();
        let rpo = reverse_post_order(mir, START_BLOCK);

        let (basic_blocks, local_decls) = mir.basic_blocks_and_local_decls_mut();
        let local_decls = &*local_decls;

        let mut ever_borrowed = IdxSetBuf::new_empty(local_decls.len());
        let mut block_values = IndexVec::from_elem_n(None, basic_blocks.len());
        for block in rpo {
            let ever_borrowed = &mut ever_borrowed;
            let current_values = {
                let pred_values =
                    predecessors[block]
                        .iter()
                        .map(|&b| block_values[b].as_ref());
                merge_values(pred_values)
                    .unwrap_or_else(|| IndexVec::from_elem_n(None, local_decls.len()))
            };
            debug!("Starting ConstPropagation on {:?} with values {:?}", block, current_values);
            let mut visitor = ConstPropagator { ever_borrowed, current_values };
            visitor.visit_basic_block_data(block, &mut basic_blocks[block]);
            block_values[block] = Some(visitor.current_values);
        }
    }
}

fn has_const_assignment<'tcx>(block: &BasicBlockData<'tcx>) -> bool {
    block.statements.iter().any(|statement| {
        match statement.kind {
            StatementKind::Assign(_, Rvalue::Use(Operand::Constant(_))) => true,
            _ => false,
        }
    })
}

fn merge_values<'a, 'tcx: 'a, I>(iter: I) -> Option<LocalValues<'tcx>>
    where I: Iterator<Item = Option<&'a LocalValues<'tcx>>>
{
    let mut now = None;
    for other in iter {
        let other = other?;
        match now.take() {
            None => now = Some(other.clone()),
            Some(so_far) => now = Some(combine_values(so_far, other)),
        }
    }
    now
}

fn combine_values<'tcx>(mut x: LocalValues<'tcx>, y: &LocalValues<'tcx>)
    -> LocalValues<'tcx>
{
    debug_assert_eq!(x.len(), y.len());
    for (a, b) in x.iter_mut().zip(y) {
        if *a != *b {
            *a = None;
        }
    }
    x
}

type LocalValues<'tcx> = IndexVec<Local, Option<Box<Constant<'tcx>>>>;
struct ConstPropagator<'a, 'tcx> {
    ever_borrowed: &'a mut IdxSetBuf<Local>,
    current_values: LocalValues<'tcx>,
}

impl<'a, 'tcx> ConstPropagator<'a, 'tcx> {
    fn mark_borrowed(&mut self, lvalue: &Lvalue<'tcx>) {
        match *lvalue {
            Lvalue::Local(local) => {
                self.ever_borrowed.add(&local);
                self.current_values[local] = None;
            }
            Lvalue::Static(_) => {}
            Lvalue::Projection(ref projection) => {
                self.mark_borrowed(&projection.base);
            }
        }
    }
}

impl<'a, 'tcx> MutVisitor<'tcx> for ConstPropagator<'a, 'tcx> {
    fn visit_assign(
        &mut self,
        _block: BasicBlock,
        lvalue: &mut Lvalue<'tcx>,
        rvalue: &mut Rvalue<'tcx>,
        location: Location
    ) {
        self.visit_rvalue(rvalue, location);

        if let Lvalue::Local(local) = *lvalue {
            self.current_values[local] = None;
            if let Rvalue::Use(Operand::Constant(ref constant)) = *rvalue {
                if !self.ever_borrowed.contains(&local) {
                    self.current_values[local] = Some(Box::clone(constant));
                    return;
                }
            }
        }

        self.visit_lvalue(lvalue, LvalueContext::Store, location);
    }

    fn visit_operand(
        &mut self,
        operand: &mut Operand<'tcx>,
        _location: Location
    ) {
        // Normal Move optimizations will simplify those, so only look at Copy
        if let Operand::Copy(Lvalue::Local(local)) = *operand {
            if let Some(ref constant) = self.current_values[local] {
                *operand = Operand::Constant(Box::clone(constant));
            }
        }

        // Intentionally don't recurse, so the code below doesn't invalidate
        // our cache when something is used as an argument.
    }

    fn visit_local(
        &mut self,
        local: &mut Local,
        _context: LvalueContext<'tcx>,
        _location: Location
    ) {
        // On any use of a local that the visitor actually reaches, invalidate.
        // Conveniently, this invalidates on StorageDead, reducing clutter.
        self.current_values[*local] = None;
    }

    fn visit_rvalue(
        &mut self,
        rvalue: &mut Rvalue<'tcx>,
        location: Location
    ) {
        self.super_rvalue(rvalue, location);

        match *rvalue {
            Rvalue::Ref(_, _, ref lvalue) => {
                self.mark_borrowed(lvalue);
            }
            // FIXME: fold operators if their arguments are now const
            _ => {}
        }
    }
}