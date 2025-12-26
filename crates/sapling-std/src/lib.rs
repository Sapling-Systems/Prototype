/*
#[sapling_func("Sum", "result")]
fn math_sum(
    #[num_index]
    values: Vec<u64>
) -> u64 {
    values.iter().sum()
}

#[sapling_func("GreaterThan", "result")]
fn math_gt(
    #[subject]
    left: u64,
    #[subject]
    right: u64,
) -> bool {
    left > right
}

#[derive(SaplingSchema)]
struct ExplainResult {}

#[sapling_func("Explain", "result")]
fn explain(
    context: SaplingFuncContext,
    #[subject]
    query: Subject,
    #[num_index("fact")]
    facts: Vec<usize>,
) -> ExplainResult {}

*/
