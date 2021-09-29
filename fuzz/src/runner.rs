use std::{
    fmt::Debug,
    time::{Duration, Instant},
};

use rand::prelude::ThreadRng;
use sapling::Lang;

use crate::{utils, Arbitrary, Shrink};

use self::immut::Immut;

/// Run the fuzzer on some parser
pub fn fuzz<'lang, A: Arbitrary<'lang> + Debug>(
    lang: &'lang Lang,
    iteration_limit: Option<usize>,
    config: A::Config,
) {
    Runner::<A>::new(lang, config).run(iteration_limit);
}

/// A thread-local `Runner` for fuzz tests
#[derive(Debug, Clone)]
struct Runner<'lang, A: Arbitrary<'lang>> {
    rng: ThreadRng,
    static_data: Immut<A::StaticData>,
    table: A::SampleTable,
    config: A::Config,

    // Stat printing state
    fuzz_start_time: Immut<Instant>,
    elapsed_secs_for_last_print: u64,

    // Fuzzing loop state
    unparsed_string: String,
    iteration_count: usize,
    total_bytes_tokenized: usize,
    total_time_tokenizing: Duration,
}

impl<'lang, A: Arbitrary<'lang> + Debug> Runner<'lang, A> {
    /// Create a `Runner` which hasn't run any fuzzing iterations.
    fn new(lang: &'lang Lang, config: A::Config) -> Self {
        let static_data = Immut::new(A::gen_static_data(lang, &config));
        let mut rng = rand::thread_rng();
        Self {
            table: A::gen_table(&static_data, &mut rng, &config),
            rng,
            static_data,
            config,

            fuzz_start_time: Instant::now().into(),
            elapsed_secs_for_last_print: 0,

            unparsed_string: String::new(),
            iteration_count: 0,
            total_bytes_tokenized: 0,
            total_time_tokenizing: Duration::ZERO,
        }
    }

    /// Run the mainloop of the fuzzer
    fn run(mut self, iteration_limit: Option<usize>) {
        loop {
            // Re-generate sample tables every couple of thousand iterations.  This is a compromise
            // to increase the generation speed but not always sampling from the same finite set of
            // samples.
            self.table = A::gen_table(&self.static_data, &mut self.rng, &self.config);

            for _ in 0..1_000 {
                // Generate new sample
                let sample = A::gen(&self.static_data, &self.table, &self.config, &mut self.rng);

                if !self.check(&sample) {
                    let shrunk_sample = self.shrink_sample(sample);
                    // Set `self.unparsed_string` so it can be printed
                    self.unparsed_string.clear();
                    shrunk_sample.unparse(&self.static_data, &mut self.unparsed_string);
                    dbg!(shrunk_sample, self.unparsed_string);
                    panic!("Parsing failed!");
                }

                self.iteration_count += 1;
                let reached_iteration_limit = Some(self.iteration_count) >= iteration_limit;

                // Print stats roughly every second, or when the test ends.  We check the times every
                // few loop iterations to speed up the fuzzing loop.
                let elapsed_secs = self.fuzz_start_time.elapsed().as_secs();
                if elapsed_secs > self.elapsed_secs_for_last_print || reached_iteration_limit {
                    self.elapsed_secs_for_last_print = elapsed_secs;
                    println!(
                        "{} iters.  {} in {:?} = {}/s",
                        self.iteration_count,
                        utils::format_big_bytes(self.total_bytes_tokenized as f32),
                        self.total_time_tokenizing,
                        utils::format_big_bytes(
                            self.total_bytes_tokenized as f32
                                / self.total_time_tokenizing.as_secs_f32()
                        )
                    );
                }
                // Exit loop if iteration limit is reached
                if reached_iteration_limit {
                    return;
                }
            }
        }
    }

    /// Shrink a witness sample until a minimal witness is found.  Shrinking works in the following
    /// way:
    /// 1. The sample is converted to an `A::Shrink` (which allows the shrinking strategy to add
    ///    extra state).
    /// 2. This generates an [`Iterator`] of slightly smaller samples
    /// 3. Each of these smaller samples are tested individually.  If any of these fail, then it
    ///    becomes a new witness, and we continue at step `2` with this as the new sample.
    /// 4. If all slightly smaller samples aren't witnesses (or there aren't any), then the current
    ///    sample is deemed minimal and returned.
    fn shrink_sample(&mut self, sample: A) -> A {
        println!("Shrinking...");
        self.shrink(sample.into())
    }

    fn shrink(&mut self, shrink: A::Shrink) -> A {
        for smaller_case in shrink.smaller_cases() {
            // If a smaller case also fails, then make that the new minimal case and keep trying to
            // shrink
            if !self.check(&smaller_case) {
                return self.shrink(smaller_case.into_owned());
            }
        }
        // If no smaller cases failed (or this can't be shrunk), then this is the smallest witness
        shrink.into()
    }

    fn check(&mut self, sample: &A) -> bool {
        // Unparse the sample
        self.unparsed_string.clear();
        sample.unparse(&self.static_data, &mut self.unparsed_string);
        println!("Testing {:?}", self.unparsed_string);
        // Parse the string generated by unparsing `sample` (whilst timing the parser).  This
        // is expected to be the same as `sample`
        let start = Instant::now();
        let parsed_sample = A::parse(&self.static_data, &self.unparsed_string);
        self.total_bytes_tokenized += self.unparsed_string.len();
        self.total_time_tokenizing += start.elapsed();

        parsed_sample.as_ref() == Some(&sample)
    }
}

mod immut {
    /// Wrapper type which only permits immutable references to its contents.  This is equivalent
    /// to `let x: A` but can be used in a type definition.
    #[derive(Debug, Clone)]
    #[repr(transparent)]
    pub(super) struct Immut<T> {
        inner: T,
    }

    impl<T> Immut<T> {
        pub(super) fn new(inner: T) -> Self {
            Self { inner }
        }

        pub(super) fn inner(&self) -> &T {
            &self.inner
        }
    }

    impl<T> std::ops::Deref for Immut<T> {
        type Target = T;

        fn deref(&self) -> &Self::Target {
            self.inner()
        }
    }

    impl<T> std::convert::AsRef<T> for Immut<T> {
        fn as_ref(&self) -> &T {
            self.inner()
        }
    }

    impl<T> From<T> for Immut<T> {
        fn from(v: T) -> Self {
            Self::new(v)
        }
    }
}
