use super::{Local, InstrumentationData, MirError, Body};

pub struct BodyMutationPassState<'tcx, 'cache> {
    values: Vec<Local>,
    instrumentation_data: InstrumentationData<'tcx, 'cache>,
}

impl<'tcx, 'cache> BodyMutationPassState<'tcx, 'cache> {
    pub fn new(values: Vec<Local>, instrumentation_data: InstrumentationData<'tcx, 'cache>) -> Self {
        BodyMutationPassState { values, instrumentation_data }
    }

    pub fn instrument_locals(&mut self) -> Result<(), MirError> {
        self.instrumentation_data.instrument_locals(&self.values)
    }

    pub fn instrument_instructions(&mut self) -> Result<(), MirError> {
        self.instrumentation_data.instrument_instructions()?;
        Ok(())
    }

    pub fn finalize(mut self) -> Body {
        self.instrument_locals().unwrap();
        self.instrumentation_data.body.finalize_prologue();
        self.instrument_instructions().unwrap();
        self.instrumentation_data.body.finalize()
    }
}
