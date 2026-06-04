use anyhow::Error;

#[must_use = "the error string must be used"]
pub fn render_error(error: &Error) -> String {
    let messages = error.chain().map(ToString::to_string).collect::<Vec<_>>();
    if messages.len() <= 1 {
        let full = error.to_string();
        if let Some((primary, context)) = split_categorized(&full) {
            let mut rendered = format!("alur: {primary}");
            if let Some(context) = context {
                rendered.push('\n');
                rendered.push_str("context: ");
                rendered.push_str(context);
            }
            return rendered;
        }

        return format!("alur: {error}");
    }

    let Some(primary_index) = messages.iter().position(|message| is_categorized(message)) else {
        return format!("alur: {error}");
    };

    let primary = &messages[primary_index];
    let mut rendered = format!("alur: {primary}");

    let contexts = messages
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != primary_index)
        .map(|(_, message)| message.as_str())
        .collect::<Vec<_>>();

    if !contexts.is_empty() {
        rendered.push('\n');
        rendered.push_str("context: ");
        rendered.push_str(&contexts.join(" | "));
    }

    rendered
}

fn split_categorized(message: &str) -> Option<(&str, Option<&str>)> {
    categorized_index(message).map(|index| {
        let primary = &message[index..];
        let context = if index == 0 {
            None
        } else {
            Some(message[..index].trim_end_matches(": ").trim())
        };
        (primary, context.filter(|value| !value.is_empty()))
    })
}

fn is_categorized(message: &str) -> bool {
    categorized_index(message).is_some()
}

fn categorized_index(message: &str) -> Option<usize> {
    [
        "parse error",
        "config error",
        "detection error",
        "execution error",
    ]
    .iter()
    .filter_map(|prefix| message.find(prefix))
    .min()
}

#[cfg(test)]
mod tests {
    use anyhow::{Context, anyhow};

    use super::render_error;

    #[test]
    fn renders_context_chain_under_primary_category() {
        let error = Err::<(), _>(anyhow!("runner detail"))
            .context("execution error")
            .unwrap_err();

        assert_eq!(
            render_error(&error),
            "alur: execution error\ncontext: runner detail"
        );
    }
}
