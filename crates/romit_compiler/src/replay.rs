use crate::diag::Diagnostic;
use crate::interp::{Interpreter, Value};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Minimal line format: `index|EventName|arg1=val,arg2=val`
pub fn run_replay(
    interp: &Interpreter,
    corpus_path: &Path,
    expectations: &[(usize, String)],
) -> Result<(), Vec<Diagnostic>> {
    let text = fs::read_to_string(corpus_path).map_err(|e| {
        vec![Diagnostic::error(
            "E501",
            format!("cannot read replay corpus: {}", e),
            corpus_path.display().to_string(),
            1,
            1,
            "",
        )]
    })?;
    let mut diags = Vec::new();
    let mut rt = interp.init_from_spec();
    for line in text.lines() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let (index, event, args) = parse_replay_line(line).map_err(|e| {
            vec![Diagnostic::error(
                "E501",
                e,
                corpus_path.display().to_string(),
                1,
                1,
                "",
            )]
        })?;
        let result = interp.run_event(&mut rt, &event, &args).map_err(|e| {
            vec![Diagnostic::error(
                "E501",
                e,
                corpus_path.display().to_string(),
                1,
                1,
                "",
            )]
        })?;
        if let Some((_, expected)) = expectations.iter().find(|(i, _)| *i == index) {
            let found = result.outputs.iter().any(|o| o.name == *expected);
            if !found {
                diags.push(Diagnostic::error(
                    "E501",
                    format!(
                        "replay changed behaviour at event {}: expected output {}",
                        index, expected
                    ),
                    corpus_path.display().to_string(),
                    1,
                    1,
                    "",
                ));
            }
        }
    }
    if diags.is_empty() {
        Ok(())
    } else {
        Err(diags)
    }
}

fn parse_replay_line(line: &str) -> Result<(usize, String, HashMap<String, Value>), String> {
    let parts: Vec<&str> = line.splitn(3, '|').collect();
    if parts.len() != 3 {
        return Err("replay line must look like: index|Event|key=val,...".into());
    }
    let index: usize = parts[0]
        .trim()
        .parse()
        .map_err(|_| "invalid replay index".to_string())?;
    let event = parts[1].trim().to_string();
    let mut args = HashMap::new();
    if !parts[2].trim().is_empty() {
        for pair in parts[2].split(',') {
            let (k, v) = pair
                .split_once('=')
                .ok_or_else(|| format!("invalid arg pair: {}", pair))?;
            args.insert(k.trim().to_string(), parse_replay_value(v.trim()));
        }
    }
    Ok((index, event, args))
}

fn parse_replay_value(s: &str) -> Value {
    if s.starts_with('"') && s.ends_with('"') {
        return Value::Text(s[1..s.len() - 1].to_string());
    }
    if let Ok(n) = s.parse::<i64>() {
        return Value::Int(n);
    }
    if s == "true" {
        return Value::Bool(true);
    }
    if s == "false" {
        return Value::Bool(false);
    }
    Value::Text(s.to_string())
}
