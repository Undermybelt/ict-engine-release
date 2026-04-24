use anyhow::{bail, Result};

use crate::factors::FactorRegistry;
use crate::state::FactorMutationSpec;

pub fn apply_factor_mutation_spec(
    registry: &mut FactorRegistry,
    spec: &FactorMutationSpec,
) -> Result<()> {
    if !spec.base_factor.is_empty() && registry.get(&spec.base_factor).is_none() {
        bail!("unknown mutation base_factor '{}'", spec.base_factor);
    }
    for (factor, enabled) in &spec.enabled_overrides {
        if !registry.set_enabled(factor, *enabled) {
            bail!("unknown factor '{}' in enabled_overrides", factor);
        }
    }
    for (parameter, value) in &spec.parameter_overrides {
        if spec.base_factor.is_empty() {
            bail!("parameter_overrides require a base_factor");
        }
        if !registry.set_parameter(&spec.base_factor, parameter, *value) {
            bail!(
                "unknown factor '{}' for parameter override '{}'",
                spec.base_factor,
                parameter
            );
        }
    }
    Ok(())
}
