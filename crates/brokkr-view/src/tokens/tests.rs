use super::*;

fn cells_of(input: Option<u64>, output: Option<u64>, read: Option<u64>) -> TokenCells {
    let usage = crate::finish_usage(TokenUsage {
        input_tokens: input,
        output_tokens: output,
        cache_read_tokens: read,
        ..TokenUsage::default()
    });
    usage_cells(usage.as_ref(), "").1
}

#[test]
fn tokens_read_with_arrows_and_the_cache_hit_beside_them() {
    let cells = cells_of(Some(1_190_000), Some(48_000), Some(1_188_810));
    assert_eq!(cells.arrows(), "Σ 1.24M tok ↑ 1.19M ↓ 48k");
    assert_eq!(cells.line(), "Σ 1.24M tok ↑ 1.19M ↓ 48k · 99.9% cache");
}

#[test]
fn no_cache_read_reported_leaves_the_arrows_alone() {
    let cells = cells_of(Some(5_000), Some(1_000), None);
    assert_eq!(cells.line(), "Σ 6k tok ↑ 5k ↓ 1k");
}

#[test]
fn nothing_reported_reads_as_the_totals_own_words() {
    let cells = usage_cells(None, "").1;
    assert_eq!(
        (cells.arrows(), cells.line()),
        (crate::ABSENT.to_string(), crate::ABSENT.to_string())
    );
}
