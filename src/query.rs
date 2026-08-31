use anyhow::{Result, bail};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryStage {
    Fuzzy(String),
    Literal(String),
    Regex(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryPlan {
    pub stages: Vec<QueryStage>,
}

impl QueryPlan {
    pub fn parse(input: &str) -> Result<Self> {
        let raw_stages = split_stages(input)?;
        let stages = raw_stages
            .into_iter()
            .map(|stage| parse_stage(&stage))
            .collect::<Result<Vec<_>>>()?;
        for stage in &stages {
            if let QueryStage::Regex(pattern) = stage {
                regex::Regex::new(pattern)
                    .map_err(|error| anyhow::anyhow!("invalid query regex {pattern:?}: {error}"))?;
            }
        }
        Ok(Self { stages })
    }

    pub fn is_empty(&self) -> bool {
        self.stages.len() == 1
            && matches!(&self.stages[0], QueryStage::Fuzzy(value) if value.is_empty())
    }

    pub fn last(&self) -> &QueryStage {
        self.stages.last().expect("query always has one stage")
    }

    pub fn narrows_from(&self, previous: &Self) -> bool {
        if self.stages.len() < previous.stages.len() {
            return false;
        }
        if self.stages.len() > previous.stages.len() {
            return previous
                .stages
                .iter()
                .eq(self.stages.iter().take(previous.stages.len()));
        }
        if self.stages.len() == 1 {
            return stage_extends(self.last(), previous.last());
        }
        self.stages[..self.stages.len() - 1] == previous.stages[..previous.stages.len() - 1]
            && stage_extends(self.last(), previous.last())
    }
}

fn split_stages(input: &str) -> Result<Vec<String>> {
    let mut stages = Vec::new();
    let mut current = String::new();
    let mut escaped = false;
    let mut quote = false;
    for character in input.chars() {
        if escaped {
            current.push(character);
            escaped = false;
            continue;
        }
        match character {
            '\\' => escaped = true,
            '"' => {
                quote = !quote;
                current.push(character);
            }
            '#' if !quote => {
                stages.push(current.trim().to_string());
                current.clear();
            }
            character => current.push(character),
        }
    }
    if escaped {
        bail!("query ends with an unpaired escape");
    }
    if quote {
        bail!("query contains an unterminated quote");
    }
    stages.push(current.trim().to_string());
    Ok(stages)
}

fn parse_stage(stage: &str) -> Result<QueryStage> {
    if stage.len() >= 2 && stage.starts_with('"') && stage.ends_with('"') {
        return Ok(QueryStage::Literal(unescape(&stage[1..stage.len() - 1])?));
    }
    if stage.len() >= 2 && stage.starts_with('/') && stage.ends_with('/') {
        return Ok(QueryStage::Regex(unescape(&stage[1..stage.len() - 1])?));
    }
    Ok(QueryStage::Fuzzy(unescape(stage)?))
}

fn stage_extends(next: &QueryStage, previous: &QueryStage) -> bool {
    match (previous, next) {
        (QueryStage::Fuzzy(left), QueryStage::Fuzzy(right)) => {
            let mut remaining = right.chars();
            left.chars()
                .all(|character| remaining.by_ref().any(|next| next == character))
        }
        (QueryStage::Literal(left), QueryStage::Literal(right))
        | (QueryStage::Regex(left), QueryStage::Regex(right)) => right.starts_with(left),
        _ => false,
    }
}

fn unescape(input: &str) -> Result<String> {
    let mut output = String::with_capacity(input.len());
    let mut escaped = false;
    for character in input.chars() {
        if escaped {
            output.push(character);
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else {
            output.push(character);
        }
    }
    if escaped {
        bail!("query stage ends with an unpaired escape");
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_and_literal_stages() {
        assert_eq!(
            QueryPlan::parse(r#"cargo#"test suite""#).unwrap().stages,
            vec![
                QueryStage::Fuzzy("cargo".into()),
                QueryStage::Literal("test suite".into())
            ]
        );
    }

    #[test]
    fn escaped_hash_stays_literal() {
        let plan = QueryPlan::parse(r#"foo\#bar"#).unwrap();
        assert_eq!(plan.stages, vec![QueryStage::Fuzzy("foo#bar".into())]);
    }

    #[test]
    fn regex_stage_is_explicit_and_bounded_to_one_stage() {
        let plan = QueryPlan::parse("cargo#/test [0-9]+/").unwrap();
        assert_eq!(plan.stages.len(), 2);
        assert_eq!(plan.stages[1], QueryStage::Regex("test [0-9]+".into()));
    }

    #[test]
    fn malformed_quotes_and_escapes_are_errors() {
        assert!(QueryPlan::parse("foo#\"bar").is_err());
        assert!(QueryPlan::parse("foo\\").is_err());
    }

    #[test]
    fn extension_detection_supports_subfilters() {
        let first = QueryPlan::parse("cargo").unwrap();
        let second = QueryPlan::parse("cargo#t").unwrap();
        let third = QueryPlan::parse("cargo#te").unwrap();
        assert!(second.narrows_from(&first));
        assert!(third.narrows_from(&second));
    }
}
