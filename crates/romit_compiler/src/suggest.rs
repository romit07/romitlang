/// Smallest edit distance — for short script variable names only.
fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut dp = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for (i, row) in dp.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, val) in dp[0].iter_mut().enumerate() {
        *val = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }
    dp[a.len()][b.len()]
}

pub fn did_you_mean(name: &str, candidates: &[String]) -> Option<String> {
    let mut best: Option<(usize, String)> = None;
    for c in candidates {
        if c == name {
            return None;
        }
        let d = edit_distance(name, c);
        let max = name.len().max(c.len()).max(1);
        if d <= max / 2 + 1 {
            if best.as_ref().map_or(true, |(bd, _)| d < *bd) {
                best = Some((d, c.clone()));
            }
        }
    }
    best.map(|(_, s)| s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggests_close_name() {
        let scope = vec!["total".into(), "count".into()];
        assert_eq!(did_you_mean("totl", &scope).as_deref(), Some("total"));
    }
}
