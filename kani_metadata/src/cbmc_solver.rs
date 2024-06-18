// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use serde::{Deserialize, Serialize};
use strum_macros::{AsRefStr, EnumString, VariantNames};

/// An enum for CBMC solver options. All variants are handled by Kani, except for
/// the `Binary` one, which it passes as is to CBMC's `--external-sat-solver`
/// option.
#[derive(Debug, Clone, AsRefStr, EnumString, VariantNames, PartialEq, Eq, Serialize, Deserialize)]
#[strum(serialize_all = "snake_case")]
pub enum CbmcSolver {
    /// CaDiCaL which is available in CBMC as of version 5.77.0
    Cadical,

    /// The kissat solver that is included in the Kani bundle
    Kissat,

    /// MiniSAT (CBMC's default solver)
    Minisat,

    /// A solver binary variant whose argument gets passed to
    /// `--external-sat-solver`. The specified binary must exist in path.
    #[strum(disabled, serialize = "bin=<SAT_SOLVER_BINARY>")]
    Binary(String),
}
