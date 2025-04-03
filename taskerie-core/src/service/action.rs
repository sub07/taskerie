use anyhow::anyhow;

use crate::model::{ParamContext, action};

pub fn render_argument_parts(
    parts: &[action::ArgumentPart],
    param_context: &ParamContext,
) -> anyhow::Result<String> {
    let mut result = String::new();

    render_argument_parts_in(parts, param_context, &mut result)?;

    Ok(result)
}
pub fn render_argument_parts_in(
    parts: &[action::ArgumentPart],
    param_context: &ParamContext,
    buffer: &mut String,
) -> anyhow::Result<()> {
    for part in parts {
        match part {
            action::ArgumentPart::Literal(literal) => {
                buffer.push_str(literal);
            }
            action::ArgumentPart::Variable(param) => {
                let value = param_context
                    .get(param)
                    .ok_or(anyhow!("Param `{param}` is not defined"))?;
                buffer.push_str(value);
            }
        }
    }

    Ok(())
}
