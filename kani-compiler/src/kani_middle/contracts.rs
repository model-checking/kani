use rustc_hir::def_id::DefId;

#[derive(Default)]
pub struct GFnContract<C> {
    requires: Option<C>,
    ensures: Option<C>,
    assigns: Option<C>,
}

pub type FnContract = GFnContract<DefId>;

impl<C> GFnContract<C> {
    pub fn requires(&self) -> &Option<C> {
        &self.requires
    }

    pub fn ensures(&self) -> &Option<C> {
        &self.ensures
    }

    pub fn new(requires: Option<C>, ensures: Option<C>, assigns: Option<C>) -> Self {
        Self { requires, ensures, assigns }
    }

    pub fn map<C0, F: FnMut(&C) -> C0>(&self, mut f: F) -> GFnContract<C0> {
        GFnContract {
            requires: self.requires.as_ref().map(&mut f),
            ensures: self.ensures.as_ref().map(&mut f),
            assigns: self.assigns.as_ref().map(&mut f),
        }
    }

    pub fn enforceable(&self) -> bool {
        self.requires().is_some() || self.ensures().is_some()
    }
}
