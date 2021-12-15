use rustc_infer::infer::canonical::QueryOutlivesConstraint;
use rustc_infer::infer::canonical::QueryRegionConstraints;
use rustc_infer::infer::outlives::env::RegionBoundPairs;
use rustc_infer::infer::outlives::obligations::{TypeOutlives, TypeOutlivesDelegate};
use rustc_infer::infer::region_constraints::{GenericKind, VerifyBound};
use rustc_infer::infer::{self, InferCtxt, SubregionOrigin};
use rustc_middle::mir::ConstraintCategory;
use rustc_middle::ty::subst::GenericArgKind;
use rustc_middle::ty::TypeFoldable;
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::DUMMY_SP;

use crate::{
    constraints::OutlivesConstraint,
    nll::ToRegionVid,
    region_infer::TypeTest,
    type_check::{Locations, MirTypeckRegionConstraints},
    universal_regions::UniversalRegions,
};

crate struct ConstraintConversion<'a, 'tcx> {
    infcx: &'a InferCtxt<'a, 'tcx>,
    tcx: TyCtxt<'tcx>,
    universal_regions: &'a UniversalRegions<'tcx>,
    region_bound_pairs: &'a RegionBoundPairs<'tcx>,
    implicit_region_bound: Option<ty::Region<'tcx>>,
    param_env: ty::ParamEnv<'tcx>,
    locations: Locations,
    category: ConstraintCategory,
    constraints: &'a mut MirTypeckRegionConstraints<'tcx>,
}

impl<'a, 'tcx> ConstraintConversion<'a, 'tcx> {
    crate fn new(
        infcx: &'a InferCtxt<'a, 'tcx>,
        universal_regions: &'a UniversalRegions<'tcx>,
        region_bound_pairs: &'a RegionBoundPairs<'tcx>,
        implicit_region_bound: Option<ty::Region<'tcx>>,
        param_env: ty::ParamEnv<'tcx>,
        locations: Locations,
        category: ConstraintCategory,
        constraints: &'a mut MirTypeckRegionConstraints<'tcx>,
    ) -> Self {
        Self {
            infcx,
            tcx: infcx.tcx,
            universal_regions,
            region_bound_pairs,
            implicit_region_bound,
            param_env,
            locations,
            category,
            constraints,
        }
    }

    #[instrument(skip(self), level = "debug")]
    pub(super) fn convert_all(&mut self, query_constraints: &QueryRegionConstraints<'tcx>) {
        let QueryRegionConstraints { outlives, member_constraints } = query_constraints;

        // Annoying: to invoke `self.to_region_vid`, we need access to
        // `self.constraints`, but we also want to be mutating
        // `self.member_constraints`. For now, just swap out the value
        // we want and replace at the end.
        let mut tmp = std::mem::take(&mut self.constraints.member_constraints);
        for member_constraint in member_constraints {
            tmp.push_constraint(member_constraint, |r| self.to_region_vid(r));
        }
        self.constraints.member_constraints = tmp;

        for query_constraint in outlives {
            self.convert(query_constraint);
        }
    }

    pub(super) fn convert(&mut self, query_constraint: &QueryOutlivesConstraint<'tcx>) {
        debug!("generate: constraints at: {:#?}", self.locations);

        // Extract out various useful fields we'll need below.
        let ConstraintConversion {
            tcx, region_bound_pairs, implicit_region_bound, param_env, ..
        } = *self;

        // At the moment, we never generate any "higher-ranked"
        // region constraints like `for<'a> 'a: 'b`. At some point
        // when we move to universes, we will, and this assertion
        // will start to fail.
        let ty::OutlivesPredicate(k1, r2) = query_constraint.no_bound_vars().unwrap_or_else(|| {
            bug!("query_constraint {:?} contained bound vars", query_constraint,);
        });

        match k1.unpack() {
            GenericArgKind::Lifetime(r1) => {
                let r1_vid = self.to_region_vid(r1);
                let r2_vid = self.to_region_vid(r2);
                self.add_outlives(r1_vid, r2_vid);
            }

            GenericArgKind::Type(mut t1) => {
                // we don't actually use this for anything, but
                // the `TypeOutlives` code needs an origin.
                let origin = infer::RelateParamBound(DUMMY_SP, t1, None);

                // Placeholder regions need to be converted now because it may
                // create new region variables, which can't be done later when
                // verifying these bounds.
                if t1.has_placeholders() {
                    t1 = tcx.fold_regions(&t1, &mut false, |r, _| match *r {
                        ty::RegionKind::RePlaceholder(placeholder) => {
                            self.constraints.placeholder_region(self.infcx, placeholder)
                        }
                        _ => r,
                    });
                }

                TypeOutlives::new(
                    &mut *self,
                    tcx,
                    region_bound_pairs,
                    implicit_region_bound,
                    param_env,
                )
                .type_must_outlive(origin, t1, r2);
            }

            GenericArgKind::Const(_) => {
                // Consts cannot outlive one another, so we
                // don't need to handle any relations here.
            }
        }
    }

    fn verify_to_type_test(
        &mut self,
        generic_kind: GenericKind<'tcx>,
        region: ty::Region<'tcx>,
        verify_bound: VerifyBound<'tcx>,
    ) -> TypeTest<'tcx> {
        let lower_bound = self.to_region_vid(region);

        TypeTest { generic_kind, lower_bound, locations: self.locations, verify_bound }
    }

    fn to_region_vid(&mut self, r: ty::Region<'tcx>) -> ty::RegionVid {
        if let ty::RePlaceholder(placeholder) = r {
            self.constraints.placeholder_region(self.infcx, *placeholder).to_region_vid()
        } else {
            self.universal_regions.to_region_vid(r)
        }
    }

    fn add_outlives(&mut self, sup: ty::RegionVid, sub: ty::RegionVid) {
        self.constraints.outlives_constraints.push(OutlivesConstraint {
            locations: self.locations,
            category: self.category,
            sub,
            sup,
            variance_info: ty::VarianceDiagInfo::default(),
        });
    }

    fn add_type_test(&mut self, type_test: TypeTest<'tcx>) {
        debug!("add_type_test(type_test={:?})", type_test);
        self.constraints.type_tests.push(type_test);
    }
}

impl<'a, 'b, 'tcx> TypeOutlivesDelegate<'tcx> for &'a mut ConstraintConversion<'b, 'tcx> {
    fn push_sub_region_constraint(
        &mut self,
        _origin: SubregionOrigin<'tcx>,
        a: ty::Region<'tcx>,
        b: ty::Region<'tcx>,
    ) {
        let b = self.to_region_vid(b);
        let a = self.to_region_vid(a);
        self.add_outlives(b, a);
    }

    fn push_verify(
        &mut self,
        _origin: SubregionOrigin<'tcx>,
        kind: GenericKind<'tcx>,
        a: ty::Region<'tcx>,
        bound: VerifyBound<'tcx>,
    ) {
        let type_test = self.verify_to_type_test(kind, a, bound);
        self.add_type_test(type_test);
    }
}
