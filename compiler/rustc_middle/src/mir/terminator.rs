use crate::mir::interpret::Scalar;
use crate::ty::{self, Ty, TyCtxt};
use rustc_ast::{InlineAsmOptions, InlineAsmTemplatePiece};
use smallvec::{smallvec, SmallVec};

use super::{
    AssertMessage, BasicBlock, InlineAsmOperand, Operand, Place, SourceInfo, Successors,
    SuccessorsMut,
};
pub use rustc_ast::Mutability;
use rustc_macros::HashStable;
use rustc_span::Span;
use std::borrow::Cow;
use std::fmt::{self, Debug, Formatter, Write};
use std::iter;
use std::slice;

pub use super::query::*;

#[derive(Debug, Clone, TyEncodable, TyDecodable, Hash, HashStable, PartialEq, PartialOrd)]
pub struct SwitchTargets {
    /// Possible values. The locations to branch to in each case
    /// are found in the corresponding indices from the `targets` vector.
    values: SmallVec<[u128; 1]>,

    /// Possible branch sites. The last element of this vector is used
    /// for the otherwise branch, so targets.len() == values.len() + 1
    /// should hold.
    //
    // This invariant is quite non-obvious and also could be improved.
    // One way to make this invariant is to have something like this instead:
    //
    // branches: Vec<(ConstInt, BasicBlock)>,
    // otherwise: Option<BasicBlock> // exhaustive if None
    //
    // However we’ve decided to keep this as-is until we figure a case
    // where some other approach seems to be strictly better than other.
    targets: SmallVec<[BasicBlock; 2]>,
}

impl SwitchTargets {
    /// Creates switch targets from an iterator of values and target blocks.
    ///
    /// The iterator may be empty, in which case the `SwitchInt` instruction is equivalent to
    /// `goto otherwise;`.
    pub fn new(targets: impl Iterator<Item = (u128, BasicBlock)>, otherwise: BasicBlock) -> Self {
        let (values, mut targets): (SmallVec<_>, SmallVec<_>) = targets.unzip();
        targets.push(otherwise);
        Self { values, targets }
    }

    /// Builds a switch targets definition that jumps to `then` if the tested value equals `value`,
    /// and to `else_` if not.
    pub fn static_if(value: u128, then: BasicBlock, else_: BasicBlock) -> Self {
        Self { values: smallvec![value], targets: smallvec![then, else_] }
    }

    /// Returns the fallback target that is jumped to when none of the values match the operand.
    pub fn otherwise(&self) -> BasicBlock {
        *self.targets.last().unwrap()
    }

    /// Returns an iterator over the switch targets.
    ///
    /// The iterator will yield tuples containing the value and corresponding target to jump to, not
    /// including the `otherwise` fallback target.
    ///
    /// Note that this may yield 0 elements. Only the `otherwise` branch is mandatory.
    pub fn iter(&self) -> SwitchTargetsIter<'_> {
        SwitchTargetsIter { inner: iter::zip(&self.values, &self.targets) }
    }

    /// Returns a slice with all possible jump targets (including the fallback target).
    pub fn all_targets(&self) -> &[BasicBlock] {
        &self.targets
    }

    pub fn all_targets_mut(&mut self) -> &mut [BasicBlock] {
        &mut self.targets
    }

    /// Finds the `BasicBlock` to which this `SwitchInt` will branch given the
    /// specific value.  This cannot fail, as it'll return the `otherwise`
    /// branch if there's not a specific match for the value.
    pub fn target_for_value(&self, value: u128) -> BasicBlock {
        self.iter().find_map(|(v, t)| (v == value).then_some(t)).unwrap_or_else(|| self.otherwise())
    }
}

pub struct SwitchTargetsIter<'a> {
    inner: iter::Zip<slice::Iter<'a, u128>, slice::Iter<'a, BasicBlock>>,
}

impl<'a> Iterator for SwitchTargetsIter<'a> {
    type Item = (u128, BasicBlock);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(val, bb)| (*val, *bb))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a> ExactSizeIterator for SwitchTargetsIter<'a> {}

#[derive(Clone, TyEncodable, TyDecodable, Hash, HashStable, PartialEq, PartialOrd)]
pub enum TerminatorKind<'tcx> {
    /// Block should have one successor in the graph; we jump there.
    Goto { target: BasicBlock },

    /// Operand evaluates to an integer; jump depending on its value
    /// to one of the targets, and otherwise fallback to `otherwise`.
    SwitchInt {
        /// The discriminant value being tested.
        discr: Operand<'tcx>,

        /// The type of value being tested.
        /// This is always the same as the type of `discr`.
        /// FIXME: remove this redundant information. Currently, it is relied on by pretty-printing.
        switch_ty: Ty<'tcx>,

        targets: SwitchTargets,
    },

    /// Indicates that the landing pad is finished and unwinding should
    /// continue. Emitted by `build::scope::diverge_cleanup`.
    Resume,

    /// Indicates that the landing pad is finished and that the process
    /// should abort. Used to prevent unwinding for foreign items.
    Abort,

    /// Indicates a normal return. The return place should have
    /// been filled in before this executes. This can occur multiple times
    /// in different basic blocks.
    Return,

    /// Indicates a terminator that can never be reached.
    Unreachable,

    /// Drop the `Place`.
    Drop { place: Place<'tcx>, target: BasicBlock, unwind: Option<BasicBlock> },

    /// Drop the `Place` and assign the new value over it. This ensures
    /// that the assignment to `P` occurs *even if* the destructor for
    /// place unwinds. Its semantics are best explained by the
    /// elaboration:
    ///
    /// ```
    /// BB0 {
    ///   DropAndReplace(P <- V, goto BB1, unwind BB2)
    /// }
    /// ```
    ///
    /// becomes
    ///
    /// ```
    /// BB0 {
    ///   Drop(P, goto BB1, unwind BB2)
    /// }
    /// BB1 {
    ///   // P is now uninitialized
    ///   P <- V
    /// }
    /// BB2 {
    ///   // P is now uninitialized -- its dtor panicked
    ///   P <- V
    /// }
    /// ```
    ///
    /// Note that DropAndReplace is eliminated as part of the `ElaborateDrops` pass.
    DropAndReplace {
        place: Place<'tcx>,
        value: Operand<'tcx>,
        target: BasicBlock,
        unwind: Option<BasicBlock>,
    },

    /// Block ends with a call of a function.
    Call {
        /// The function that’s being called.
        func: Operand<'tcx>,
        /// Arguments the function is called with.
        /// These are owned by the callee, which is free to modify them.
        /// This allows the memory occupied by "by-value" arguments to be
        /// reused across function calls without duplicating the contents.
        args: Vec<Operand<'tcx>>,
        /// Destination for the return value. If some, the call is converging.
        destination: Option<(Place<'tcx>, BasicBlock)>,
        /// Cleanups to be done if the call unwinds.
        cleanup: Option<BasicBlock>,
        /// `true` if this is from a call in HIR rather than from an overloaded
        /// operator. True for overloaded function call.
        from_hir_call: bool,
        /// This `Span` is the span of the function, without the dot and receiver
        /// (e.g. `foo(a, b)` in `x.foo(a, b)`
        fn_span: Span,
    },

    /// Jump to the target if the condition has the expected value,
    /// otherwise panic with a message and a cleanup target.
    Assert {
        cond: Operand<'tcx>,
        expected: bool,
        msg: AssertMessage<'tcx>,
        target: BasicBlock,
        cleanup: Option<BasicBlock>,
    },

    /// A suspend point.
    Yield {
        /// The value to return.
        value: Operand<'tcx>,
        /// Where to resume to.
        resume: BasicBlock,
        /// The place to store the resume argument in.
        resume_arg: Place<'tcx>,
        /// Cleanup to be done if the generator is dropped at this suspend point.
        drop: Option<BasicBlock>,
    },

    /// Indicates the end of the dropping of a generator.
    GeneratorDrop,

    /// A block where control flow only ever takes one real path, but borrowck
    /// needs to be more conservative.
    FalseEdge {
        /// The target normal control flow will take.
        real_target: BasicBlock,
        /// A block control flow could conceptually jump to, but won't in
        /// practice.
        imaginary_target: BasicBlock,
    },
    /// A terminator for blocks that only take one path in reality, but where we
    /// reserve the right to unwind in borrowck, even if it won't happen in practice.
    /// This can arise in infinite loops with no function calls for example.
    FalseUnwind {
        /// The target normal control flow will take.
        real_target: BasicBlock,
        /// The imaginary cleanup block link. This particular path will never be taken
        /// in practice, but in order to avoid fragility we want to always
        /// consider it in borrowck. We don't want to accept programs which
        /// pass borrowck only when `panic=abort` or some assertions are disabled
        /// due to release vs. debug mode builds. This needs to be an `Option` because
        /// of the `remove_noop_landing_pads` and `abort_unwinding_calls` passes.
        unwind: Option<BasicBlock>,
    },

    /// Block ends with an inline assembly block. This is a terminator since
    /// inline assembly is allowed to diverge.
    InlineAsm {
        /// The template for the inline assembly, with placeholders.
        template: &'tcx [InlineAsmTemplatePiece],

        /// The operands for the inline assembly, as `Operand`s or `Place`s.
        operands: Vec<InlineAsmOperand<'tcx>>,

        /// Miscellaneous options for the inline assembly.
        options: InlineAsmOptions,

        /// Source spans for each line of the inline assembly code. These are
        /// used to map assembler errors back to the line in the source code.
        line_spans: &'tcx [Span],

        /// Destination block after the inline assembly returns, unless it is
        /// diverging (InlineAsmOptions::NORETURN).
        destination: Option<BasicBlock>,

        /// Cleanup to be done if the inline assembly unwinds. This is present
        /// if and only if InlineAsmOptions::MAY_UNWIND is set.
        cleanup: Option<BasicBlock>,
    },
}
#[derive(Clone, Debug, TyEncodable, TyDecodable, HashStable)]
pub struct Terminator<'tcx> {
    pub source_info: SourceInfo,
    pub kind: TerminatorKind<'tcx>,
}

impl<'tcx> Terminator<'tcx> {
    pub fn successors(&self) -> Successors<'_> {
        self.kind.successors()
    }

    pub fn successors_mut(&mut self) -> SuccessorsMut<'_> {
        self.kind.successors_mut()
    }

    pub fn unwind(&self) -> Option<&Option<BasicBlock>> {
        self.kind.unwind()
    }

    pub fn unwind_mut(&mut self) -> Option<&mut Option<BasicBlock>> {
        self.kind.unwind_mut()
    }
}

impl<'tcx> TerminatorKind<'tcx> {
    pub fn if_(
        tcx: TyCtxt<'tcx>,
        cond: Operand<'tcx>,
        t: BasicBlock,
        f: BasicBlock,
    ) -> TerminatorKind<'tcx> {
        TerminatorKind::SwitchInt {
            discr: cond,
            switch_ty: tcx.types.bool,
            targets: SwitchTargets::static_if(0, f, t),
        }
    }

    pub fn successors(&self) -> Successors<'_> {
        use self::TerminatorKind::*;
        match *self {
            Resume
            | Abort
            | GeneratorDrop
            | Return
            | Unreachable
            | Call { destination: None, cleanup: None, .. }
            | InlineAsm { destination: None, cleanup: None, .. } => None.into_iter().chain(&[]),
            Goto { target: ref t }
            | Call { destination: None, cleanup: Some(ref t), .. }
            | Call { destination: Some((_, ref t)), cleanup: None, .. }
            | Yield { resume: ref t, drop: None, .. }
            | DropAndReplace { target: ref t, unwind: None, .. }
            | Drop { target: ref t, unwind: None, .. }
            | Assert { target: ref t, cleanup: None, .. }
            | FalseUnwind { real_target: ref t, unwind: None }
            | InlineAsm { destination: Some(ref t), cleanup: None, .. }
            | InlineAsm { destination: None, cleanup: Some(ref t), .. } => {
                Some(t).into_iter().chain(&[])
            }
            Call { destination: Some((_, ref t)), cleanup: Some(ref u), .. }
            | Yield { resume: ref t, drop: Some(ref u), .. }
            | DropAndReplace { target: ref t, unwind: Some(ref u), .. }
            | Drop { target: ref t, unwind: Some(ref u), .. }
            | Assert { target: ref t, cleanup: Some(ref u), .. }
            | FalseUnwind { real_target: ref t, unwind: Some(ref u) }
            | InlineAsm { destination: Some(ref t), cleanup: Some(ref u), .. } => {
                Some(t).into_iter().chain(slice::from_ref(u))
            }
            SwitchInt { ref targets, .. } => None.into_iter().chain(&targets.targets),
            FalseEdge { ref real_target, ref imaginary_target } => {
                Some(real_target).into_iter().chain(slice::from_ref(imaginary_target))
            }
        }
    }

    pub fn successors_mut(&mut self) -> SuccessorsMut<'_> {
        use self::TerminatorKind::*;
        match *self {
            Resume
            | Abort
            | GeneratorDrop
            | Return
            | Unreachable
            | Call { destination: None, cleanup: None, .. }
            | InlineAsm { destination: None, cleanup: None, .. } => None.into_iter().chain(&mut []),
            Goto { target: ref mut t }
            | Call { destination: None, cleanup: Some(ref mut t), .. }
            | Call { destination: Some((_, ref mut t)), cleanup: None, .. }
            | Yield { resume: ref mut t, drop: None, .. }
            | DropAndReplace { target: ref mut t, unwind: None, .. }
            | Drop { target: ref mut t, unwind: None, .. }
            | Assert { target: ref mut t, cleanup: None, .. }
            | FalseUnwind { real_target: ref mut t, unwind: None }
            | InlineAsm { destination: Some(ref mut t), cleanup: None, .. }
            | InlineAsm { destination: None, cleanup: Some(ref mut t), .. } => {
                Some(t).into_iter().chain(&mut [])
            }
            Call { destination: Some((_, ref mut t)), cleanup: Some(ref mut u), .. }
            | Yield { resume: ref mut t, drop: Some(ref mut u), .. }
            | DropAndReplace { target: ref mut t, unwind: Some(ref mut u), .. }
            | Drop { target: ref mut t, unwind: Some(ref mut u), .. }
            | Assert { target: ref mut t, cleanup: Some(ref mut u), .. }
            | FalseUnwind { real_target: ref mut t, unwind: Some(ref mut u) }
            | InlineAsm { destination: Some(ref mut t), cleanup: Some(ref mut u), .. } => {
                Some(t).into_iter().chain(slice::from_mut(u))
            }
            SwitchInt { ref mut targets, .. } => None.into_iter().chain(&mut targets.targets),
            FalseEdge { ref mut real_target, ref mut imaginary_target } => {
                Some(real_target).into_iter().chain(slice::from_mut(imaginary_target))
            }
        }
    }

    pub fn unwind(&self) -> Option<&Option<BasicBlock>> {
        match *self {
            TerminatorKind::Goto { .. }
            | TerminatorKind::Resume
            | TerminatorKind::Abort
            | TerminatorKind::Return
            | TerminatorKind::Unreachable
            | TerminatorKind::GeneratorDrop
            | TerminatorKind::Yield { .. }
            | TerminatorKind::SwitchInt { .. }
            | TerminatorKind::FalseEdge { .. } => None,
            TerminatorKind::Call { cleanup: ref unwind, .. }
            | TerminatorKind::Assert { cleanup: ref unwind, .. }
            | TerminatorKind::DropAndReplace { ref unwind, .. }
            | TerminatorKind::Drop { ref unwind, .. }
            | TerminatorKind::FalseUnwind { ref unwind, .. }
            | TerminatorKind::InlineAsm { cleanup: ref unwind, .. } => Some(unwind),
        }
    }

    pub fn unwind_mut(&mut self) -> Option<&mut Option<BasicBlock>> {
        match *self {
            TerminatorKind::Goto { .. }
            | TerminatorKind::Resume
            | TerminatorKind::Abort
            | TerminatorKind::Return
            | TerminatorKind::Unreachable
            | TerminatorKind::GeneratorDrop
            | TerminatorKind::Yield { .. }
            | TerminatorKind::SwitchInt { .. }
            | TerminatorKind::FalseEdge { .. } => None,
            TerminatorKind::Call { cleanup: ref mut unwind, .. }
            | TerminatorKind::Assert { cleanup: ref mut unwind, .. }
            | TerminatorKind::DropAndReplace { ref mut unwind, .. }
            | TerminatorKind::Drop { ref mut unwind, .. }
            | TerminatorKind::FalseUnwind { ref mut unwind, .. }
            | TerminatorKind::InlineAsm { cleanup: ref mut unwind, .. } => Some(unwind),
        }
    }

    pub fn as_switch(&self) -> Option<(&Operand<'tcx>, Ty<'tcx>, &SwitchTargets)> {
        match self {
            TerminatorKind::SwitchInt { discr, switch_ty, targets } => {
                Some((discr, switch_ty, targets))
            }
            _ => None,
        }
    }

    pub fn as_goto(&self) -> Option<BasicBlock> {
        match self {
            TerminatorKind::Goto { target } => Some(*target),
            _ => None,
        }
    }
}

impl<'tcx> Debug for TerminatorKind<'tcx> {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        self.fmt_head(fmt)?;
        let successor_count = self.successors().count();
        let labels = self.fmt_successor_labels();
        assert_eq!(successor_count, labels.len());

        match successor_count {
            0 => Ok(()),

            1 => write!(fmt, " -> {:?}", self.successors().next().unwrap()),

            _ => {
                write!(fmt, " -> [")?;
                for (i, target) in self.successors().enumerate() {
                    if i > 0 {
                        write!(fmt, ", ")?;
                    }
                    write!(fmt, "{}: {:?}", labels[i], target)?;
                }
                write!(fmt, "]")
            }
        }
    }
}

impl<'tcx> TerminatorKind<'tcx> {
    /// Writes the "head" part of the terminator; that is, its name and the data it uses to pick the
    /// successor basic block, if any. The only information not included is the list of possible
    /// successors, which may be rendered differently between the text and the graphviz format.
    pub fn fmt_head<W: Write>(&self, fmt: &mut W) -> fmt::Result {
        use self::TerminatorKind::*;
        match self {
            Goto { .. } => write!(fmt, "goto"),
            SwitchInt { discr, .. } => write!(fmt, "switchInt({:?})", discr),
            Return => write!(fmt, "return"),
            GeneratorDrop => write!(fmt, "generator_drop"),
            Resume => write!(fmt, "resume"),
            Abort => write!(fmt, "abort"),
            Yield { value, resume_arg, .. } => write!(fmt, "{:?} = yield({:?})", resume_arg, value),
            Unreachable => write!(fmt, "unreachable"),
            Drop { place, .. } => write!(fmt, "drop({:?})", place),
            DropAndReplace { place, value, .. } => {
                write!(fmt, "replace({:?} <- {:?})", place, value)
            }
            Call { func, args, destination, .. } => {
                if let Some((destination, _)) = destination {
                    write!(fmt, "{:?} = ", destination)?;
                }
                write!(fmt, "{:?}(", func)?;
                for (index, arg) in args.iter().enumerate() {
                    if index > 0 {
                        write!(fmt, ", ")?;
                    }
                    write!(fmt, "{:?}", arg)?;
                }
                write!(fmt, ")")
            }
            Assert { cond, expected, msg, .. } => {
                write!(fmt, "assert(")?;
                if !expected {
                    write!(fmt, "!")?;
                }
                write!(fmt, "{:?}, ", cond)?;
                msg.fmt_assert_args(fmt)?;
                write!(fmt, ")")
            }
            FalseEdge { .. } => write!(fmt, "falseEdge"),
            FalseUnwind { .. } => write!(fmt, "falseUnwind"),
            InlineAsm { template, ref operands, options, .. } => {
                write!(fmt, "asm!(\"{}\"", InlineAsmTemplatePiece::to_string(template))?;
                for op in operands {
                    write!(fmt, ", ")?;
                    let print_late = |&late| if late { "late" } else { "" };
                    match op {
                        InlineAsmOperand::In { reg, value } => {
                            write!(fmt, "in({}) {:?}", reg, value)?;
                        }
                        InlineAsmOperand::Out { reg, late, place: Some(place) } => {
                            write!(fmt, "{}out({}) {:?}", print_late(late), reg, place)?;
                        }
                        InlineAsmOperand::Out { reg, late, place: None } => {
                            write!(fmt, "{}out({}) _", print_late(late), reg)?;
                        }
                        InlineAsmOperand::InOut {
                            reg,
                            late,
                            in_value,
                            out_place: Some(out_place),
                        } => {
                            write!(
                                fmt,
                                "in{}out({}) {:?} => {:?}",
                                print_late(late),
                                reg,
                                in_value,
                                out_place
                            )?;
                        }
                        InlineAsmOperand::InOut { reg, late, in_value, out_place: None } => {
                            write!(fmt, "in{}out({}) {:?} => _", print_late(late), reg, in_value)?;
                        }
                        InlineAsmOperand::Const { value } => {
                            write!(fmt, "const {:?}", value)?;
                        }
                        InlineAsmOperand::SymFn { value } => {
                            write!(fmt, "sym_fn {:?}", value)?;
                        }
                        InlineAsmOperand::SymStatic { def_id } => {
                            write!(fmt, "sym_static {:?}", def_id)?;
                        }
                    }
                }
                write!(fmt, ", options({:?}))", options)
            }
        }
    }

    /// Returns the list of labels for the edges to the successor basic blocks.
    pub fn fmt_successor_labels(&self) -> Vec<Cow<'static, str>> {
        use self::TerminatorKind::*;
        match *self {
            Return | Resume | Abort | Unreachable | GeneratorDrop => vec![],
            Goto { .. } => vec!["".into()],
            SwitchInt { ref targets, switch_ty, .. } => ty::tls::with(|tcx| {
                let param_env = ty::ParamEnv::empty();
                let switch_ty = tcx.lift(switch_ty).unwrap();
                let size = tcx.layout_of(param_env.and(switch_ty)).unwrap().size;
                targets
                    .values
                    .iter()
                    .map(|&u| {
                        ty::Const::from_scalar(tcx, Scalar::from_uint(u, size), switch_ty)
                            .to_string()
                            .into()
                    })
                    .chain(iter::once("otherwise".into()))
                    .collect()
            }),
            Call { destination: Some(_), cleanup: Some(_), .. } => {
                vec!["return".into(), "unwind".into()]
            }
            Call { destination: Some(_), cleanup: None, .. } => vec!["return".into()],
            Call { destination: None, cleanup: Some(_), .. } => vec!["unwind".into()],
            Call { destination: None, cleanup: None, .. } => vec![],
            Yield { drop: Some(_), .. } => vec!["resume".into(), "drop".into()],
            Yield { drop: None, .. } => vec!["resume".into()],
            DropAndReplace { unwind: None, .. } | Drop { unwind: None, .. } => {
                vec!["return".into()]
            }
            DropAndReplace { unwind: Some(_), .. } | Drop { unwind: Some(_), .. } => {
                vec!["return".into(), "unwind".into()]
            }
            Assert { cleanup: None, .. } => vec!["".into()],
            Assert { .. } => vec!["success".into(), "unwind".into()],
            FalseEdge { .. } => vec!["real".into(), "imaginary".into()],
            FalseUnwind { unwind: Some(_), .. } => vec!["real".into(), "cleanup".into()],
            FalseUnwind { unwind: None, .. } => vec!["real".into()],
            InlineAsm { destination: Some(_), cleanup: Some(_), .. } => {
                vec!["return".into(), "unwind".into()]
            }
            InlineAsm { destination: Some(_), cleanup: None, .. } => vec!["return".into()],
            InlineAsm { destination: None, cleanup: Some(_), .. } => vec!["unwind".into()],
            InlineAsm { destination: None, cleanup: None, .. } => vec![],
        }
    }
}
