use raug::prelude::*;
use raug_macros::processor;

#[processor(derive(Copy, Debug, Default))]
pub fn add_to_counter(
    #[state] counter: &mut i64,
    #[input] a: &i64,
    #[input] b: &i64,
    #[output] out: &mut i64,
) -> ProcResult<()> {
    *counter += a + b;
    *out = *counter;
    Ok(())
}

impl AddToCounter {
    pub fn new(initial_count: i64) -> Self {
        Self {
            counter: initial_count,
            a: 0,
            b: 0,
        }
    }
}
