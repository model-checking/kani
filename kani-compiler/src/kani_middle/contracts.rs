use rustc_hir::def_id::DefId;

#[derive(Default)]
pub struct GFnContract<C> {
    requires: Vec<C>,
    ensures: Vec<C>,
    assigns: Vec<C>,
}

pub type FnContract = GFnContract<DefId>;

impl<C> GFnContract<C> {
    pub fn requires(&self) -> &[C] {
        &self.requires
    }

    pub fn ensures(&self) -> &[C] {
        &self.ensures
    }

    pub fn new(requires: Vec<C>, ensures: Vec<C>, assigns: Vec<C>) -> Self {
        Self { requires, ensures, assigns }
    }

    pub fn map<C0, F: FnMut(&C) -> C0>(&self, mut f: F) -> GFnContract<C0> {
        GFnContract {
            requires: self.requires.iter().map(&mut f).collect(),
            ensures: self.ensures.iter().map(&mut f).collect(),
            assigns: self.assigns.iter().map(&mut f).collect(),
        }
    }

    pub fn enforceable(&self) -> bool {
        !self.requires().is_empty() || !self.ensures().is_empty()
    }
}
