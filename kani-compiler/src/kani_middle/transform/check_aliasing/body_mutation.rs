use stable_mir::mir::Body;
use super::MirError;
use super::InstrumentationData;
use super::MutatorIndex;

pub struct BodyMutationPassState<'tcx, 'cache> {
    instrumentation_data: InstrumentationData<'tcx, 'cache>,
}

impl<'tcx, 'cache> BodyMutationPassState<'tcx, 'cache> {
    pub fn new(instrumentation_data: InstrumentationData<'tcx, 'cache>) -> Self {
        BodyMutationPassState { instrumentation_data }
    }

    pub fn instrument_locals(&mut self) -> Result<(), MirError> {
        self.instrumentation_data.instrument_locals()
    }

    pub fn instrument_instructions(&mut self) -> Result<(), MirError> {
        self.instrumentation_data.instrument_instructions()?;
        Ok(())
    }

    pub fn finalize(mut self) -> Body {
        self.instrument_locals().unwrap();
        self.instrumentation_data.finalize_prologue();
        self.instrument_instructions().unwrap();
        self.instrumentation_data.finalize()
    }
}
