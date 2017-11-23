// Copyright 2017 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![warn(warnings)]

use rustc::hir;
use rustc::ty::{TyCtxt, TypeVariants};
use rustc::mir::*;
use rustc::mir::visit::*;
use rustc_data_structures::fx::{FxHashMap, FxHashSet};
use rustc_data_structures::indexed_vec::{Idx, IndexVec};
use transform::{MirPass, MirSource};
use std::mem;

/// Scalar Replacement of Aggregates:
/// Expands locals of aggregate types into many locals instead.
/// (Currently only handles tuples.)
pub struct Sroa;

impl MirPass for Sroa {
    fn run_pass<'a, 'tcx>(&self,
                          tcx: TyCtxt<'a, 'tcx, 'tcx>,
                          source: MirSource,
                          mir: &mut Mir<'tcx>) {
        let node_path = tcx.item_path_str(source.def_id);
        debug!("running on: {:?}", node_path);
        // we only run when mir_opt_level > 2
        if tcx.sess.opts.debugging_opts.mir_opt_level <= 1 {
            return;
        }

        // Don't run on constant MIR, because trans might not be able to
        // evaluate the modified MIR.
        // FIXME(eddyb) Remove check after miri is merged.
        let id = tcx.hir.as_local_node_id(source.def_id).unwrap();
        match (tcx.hir.body_owner_kind(id), source.promoted) {
            (hir::BodyOwnerKind::Fn, None) => {},
            _ => return
        }

        // Don't try to modify the return pointer or arguments
        let skip_locals = mir.arg_count + 1;

        loop {
            let candidates: FxHashSet<_> = mir
                .local_decls
                .iter_enumerated()
                .skip(skip_locals)
                .filter(|(_, x)| match x.ty.sty {
                    TypeVariants::TyTuple(types, _) => types.len() > 0,
                    _ => false,
                })
                .map(|(i, _)| i)
                .collect();
            if candidates.len() == 0 {
                return;
            }

            let mut visitor = NonEscapingLocalsVisitor { candidates };
            eprintln!("visitor {:#?}", visitor);
            visitor.visit_mir(mir);
            if visitor.candidates.len() == 0 {
                return;
            }

            let replacements = visitor
                .candidates
                .iter()
                .map(|&local| {
                    let span = mir.local_decls[local].source_info.span;
                    let ty = mir.local_decls[local].ty;
                    let types = match ty.sty {
                        TypeVariants::TyTuple(types, _) => types,
                        _ => bug!("No longer a tuple?"),
                    };
                    let new_locals = types
                        .iter()
                        .map(|local_ty| {
                            let decl = LocalDecl::new_internal(local_ty, span);
                            mir.local_decls.push(decl)
                        })
                        .collect();
                    (local, new_locals)
                })
                .collect();
            let mut visitor = LocalsReplacementVisitor { replacements };
            eprintln!("visitor {:#?}", visitor);
            visitor.visit_mir(mir);

            for (local, _) in visitor.replacements {
                let span = mir.local_decls[local].source_info.span;
                mir.local_decls[local] = LocalDecl::new_internal(tcx.types.err, span);
            }
        }
    }
}

#[derive(Debug)]
struct LocalsReplacementVisitor {
    pub replacements: FxHashMap<Local, IndexVec<Field, Local>>
}

impl LocalsReplacementVisitor {
    fn replace_statement<'tcx>(
        &self,
        new_statements: &mut Vec<Statement<'tcx>>,
        mut statement: Statement<'tcx>,
    ) {
        let source_info = statement.source_info;

        let locals;
        let operands;
        match statement.kind {
            StatementKind::StorageLive(ref local)
            if self.replacements.contains_key(local) => {
                for &new_local in &self.replacements[local] {
                    new_statements.push(Statement {
                        source_info,
                        kind: StatementKind::StorageLive(new_local),
                    })
                }
                return;
            }
            StatementKind::StorageDead(ref local)
            if self.replacements.contains_key(local) => {
                for &new_local in &self.replacements[local] {
                    new_statements.push(Statement {
                        source_info,
                        kind: StatementKind::StorageDead(new_local),
                    })
                }
                return;
            }
            StatementKind::Assign(
                Lvalue::Local(ref local),
                Rvalue::Aggregate(_, ref mut operands_ref)
            ) if self.replacements.contains_key(local) => {
                locals = &self.replacements[local];
                operands = mem::replace(operands_ref, Vec::new());
            }
            _ => {
                new_statements.push(statement);
                return;
            }
        }

        for (i, operand) in operands.into_iter().enumerate()
        {
            new_statements.push(Statement {
                source_info,
                kind: StatementKind::Assign(
                    Lvalue::Local(locals[Idx::new(i)]),
                    Rvalue::Use(operand),
                )
            });
        }
    }
}

impl<'tcx> MutVisitor<'tcx> for LocalsReplacementVisitor {
    fn visit_lvalue(
        &mut self,
        lvalue: &mut Lvalue<'tcx>,
        context: LvalueContext<'tcx>,
        location: Location
    ) {
        match *lvalue {
            Lvalue::Projection(box Projection {
                base: Lvalue::Local(local),
                elem: ProjectionElem::Field(field, _),
            })
            if self.replacements.contains_key(&local) => {
                *lvalue = Lvalue::Local(self.replacements[&local][field])
            }
            _ => {
                self.super_lvalue(lvalue, context, location)
            }
        }
    }

    fn visit_basic_block_data(
        &mut self,
        block: BasicBlock,
        data: &mut BasicBlockData<'tcx>
    ) {
        let old_statements = mem::replace(&mut data.statements, Vec::new());
        for s in old_statements {
            self.replace_statement(&mut data.statements, s);
        }
        self.super_basic_block_data(block, data)
    }
}

#[derive(Debug)]
struct NonEscapingLocalsVisitor {
    pub candidates: FxHashSet<Local>,
}

impl<'tcx> Visitor<'tcx> for NonEscapingLocalsVisitor {
    fn visit_local(
        &mut self,
        local: &Local,
        context: LvalueContext<'tcx>,
        location: Location,
    ) {
        match context {
            LvalueContext::StorageLive |
            LvalueContext::StorageDead => {}
            _ => {
                // Not a case we know we can handle,
                // so remove it from the candidates.
                if self.candidates.remove(local) {
                    eprintln!("{:#?} {:#?} {:#?}", local, context, location);
                }
            }
        }
    }

    fn visit_projection(
        &mut self,
        lvalue: &LvalueProjection<'tcx>,
        context: LvalueContext<'tcx>,
        location: Location
    ) {
        match *lvalue {
            Projection {
                base: Lvalue::Local(ref local),
                elem: ProjectionElem::Field(..)
            }
            if self.candidates.contains(local) => {
                // Ok to get a field out
            }
            _ => {
                self.super_projection(lvalue, context, location)
            }
        }
    }

    fn visit_assign(
        &mut self,
        block: BasicBlock,
        lvalue: &Lvalue<'tcx>,
        rvalue: &Rvalue<'tcx>,
        location: Location,
    ) {
        match (lvalue, rvalue) {
            ( &Lvalue::Local(ref local),
              rvalue @ &Rvalue::Aggregate(..) )
            if self.candidates.contains(local) => {
                // Aggregating into a candidate is fine
                // so long as what's going in is fine.
                self.visit_rvalue(rvalue, location);
            }
            _ => {
                self.super_assign(block, lvalue, rvalue, location)
            }
        }
    }

    fn visit_statement(
        &mut self,
        block: BasicBlock,
        statement: &Statement<'tcx>,
        location: Location,
    ) {
        if self.candidates.len() == 0 {
            // If previous statements meant we're out of candidates,
            // don't waste time looking through more
            return;
        }
        self.super_statement(block, statement, location)
    }

    fn visit_basic_block_data(
        &mut self,
        block: BasicBlock,
        data: &BasicBlockData<'tcx>,
    ) {
        if self.candidates.len() == 0 {
            // If previous blocks meant we're out of candidates,
            // don't waste time looking through things
            return;
        }
        self.super_basic_block_data(block, data)
    }
}