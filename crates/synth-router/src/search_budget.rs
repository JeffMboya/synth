// SPDX-License-Identifier: Apache-2.0

//! A wall-clock budget for a placement search that routes several candidates.
//!
//! Each candidate is a full routing run with its own timeout, and the export
//! then routes the winner once more. A loop that only checks the clock between
//! runs can still hold a call for the sum of every timeout: 900 s of runs, then
//! a run started at 899 s that is given 1200 s, then 1200 s more for the final
//! route. The budget caps each run at what is left, keeps back the time the
//! final route is likely to need, and says whether it cut the search short.

use std::time::{Duration, Instant};

/// Seconds a placement search may spend when the caller names none.
pub const DEFAULT_BUDGET_SECS: u64 = 900;

/// The shortest routing run worth starting, in seconds. A run given less than
/// this cannot finish, so it is not started.
pub const MIN_RUN_SECS: u64 = 30;

/// The least the final route is given, in seconds, however much is left: the
/// board must still be delivered.
pub const MIN_FINAL_SECS: u64 = 60;

/// The wall-clock budget of one placement search.
#[derive(Debug, Clone)]
pub struct SearchBudget {
    started: Instant,
    total: Duration,
    min_run: Duration,
    longest_run: Duration,
}

impl SearchBudget {
    /// A budget of `total_secs` seconds starting now; zero means the default,
    /// as a zero timeout means "no override" everywhere else.
    #[must_use]
    pub fn start(total_secs: u64) -> Self {
        let secs = if total_secs == 0 {
            DEFAULT_BUDGET_SECS
        } else {
            total_secs
        };
        Self::with_limits(Duration::from_secs(secs), Duration::from_secs(MIN_RUN_SECS))
    }

    /// A budget with an explicit shortest run, for tests that cannot wait
    /// seconds.
    #[must_use]
    pub fn with_limits(total: Duration, min_run: Duration) -> Self {
        Self {
            started: Instant::now(),
            total,
            min_run,
            longest_run: Duration::ZERO,
        }
    }

    /// The whole budget.
    #[must_use]
    pub fn total(&self) -> Duration {
        self.total
    }

    /// How long the search has been running.
    #[must_use]
    pub fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }

    /// What is left of the budget.
    #[must_use]
    pub fn remaining(&self) -> Duration {
        self.total.saturating_sub(self.elapsed())
    }

    /// Records how long a run took, so the time the final route will need is
    /// kept back from the next one.
    pub fn note_run(&mut self, took: Duration) {
        self.longest_run = self.longest_run.max(took);
    }

    /// The wall-clock limit for the next candidate run: `requested`, or what is
    /// left after the final route's share, whichever is less. `None` when what
    /// is left is too short for a run to finish.
    #[must_use]
    pub fn candidate_limit(&self, requested: Duration) -> Option<Duration> {
        let available = self.remaining().saturating_sub(self.longest_run);
        (available >= self.min_run).then(|| requested.min(available))
    }

    /// The wall-clock limit for the final route: `requested`, or what is left,
    /// but never less than [`MIN_FINAL_SECS`]. The final route counts against
    /// the budget, and it must still be able to deliver the board.
    #[must_use]
    pub fn final_limit(&self, requested: Duration) -> Duration {
        requested.min(self.remaining().max(Duration::from_secs(MIN_FINAL_SECS)))
    }
}

/// One candidate's result: its cost (lower is better) and the routing order the
/// router recommends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evaluated<C> {
    pub cost: C,
    pub order: Vec<String>,
}

/// What a search over candidates found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Search<T, C> {
    /// The cheapest candidate routed, with its cost and routing order, if any.
    pub best: Option<(C, T, Vec<String>)>,
    /// How many candidates were routed.
    pub routed: usize,
    /// How many there were.
    pub total: usize,
    /// Whether the budget stopped the search before every candidate was routed.
    pub cut_short: bool,
}

/// Routes `candidates` in order, each under the budget, and keeps the cheapest.
/// A tie keeps the earlier candidate, so the choice is deterministic.
///
/// `evaluate` routes one candidate with the wall-clock limit it is given and
/// returns `None` when the engine is unavailable, which ends the search with
/// `Ok(None)`: the caller keeps its own placement.
///
/// # Errors
/// Whatever `evaluate` returns.
pub fn route_candidates<T: Clone, C: Ord, E>(
    candidates: &[T],
    budget: &mut SearchBudget,
    requested: Duration,
    mut evaluate: impl FnMut(&T, usize, Duration) -> Result<Option<Evaluated<C>>, E>,
) -> Result<Option<Search<T, C>>, E> {
    let mut best: Option<(C, T, Vec<String>)> = None;
    let mut routed = 0;
    let mut cut_short = false;
    for (index, candidate) in candidates.iter().enumerate() {
        let Some(limit) = budget.candidate_limit(requested) else {
            cut_short = true;
            break;
        };
        let began = Instant::now();
        let evaluated = evaluate(candidate, index, limit)?;
        budget.note_run(began.elapsed());
        let Some(Evaluated { cost, order }) = evaluated else {
            return Ok(None);
        };
        routed += 1;
        if best
            .as_ref()
            .is_none_or(|(previous, _, _)| cost < *previous)
        {
            best = Some((cost, candidate.clone(), order));
        }
    }
    Ok(Some(Search {
        best,
        routed,
        total: candidates.len(),
        cut_short,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MS: fn(u64) -> Duration = Duration::from_millis;

    #[test]
    fn zero_means_the_default_budget() {
        assert_eq!(SearchBudget::start(0).total(), Duration::from_secs(900));
        assert_eq!(SearchBudget::start(120).total(), Duration::from_secs(120));
    }

    #[test]
    fn a_run_is_capped_at_what_is_left_not_at_the_routers_full_timeout() {
        let budget = SearchBudget::with_limits(Duration::from_secs(900), MS(10));
        let limit = budget
            .candidate_limit(Duration::from_secs(1200))
            .expect("room");
        assert!(limit <= Duration::from_secs(900), "{limit:?}");
        let small = budget
            .candidate_limit(Duration::from_secs(60))
            .expect("room");
        assert_eq!(
            small,
            Duration::from_secs(60),
            "never raised above the request"
        );
    }

    #[test]
    fn no_run_starts_when_too_little_is_left() {
        let budget = SearchBudget::with_limits(MS(5), Duration::from_secs(1));
        assert_eq!(budget.candidate_limit(Duration::from_secs(10)), None);
    }

    #[test]
    fn the_final_routes_share_is_kept_back_from_the_candidates() {
        let mut budget =
            SearchBudget::with_limits(Duration::from_secs(100), Duration::from_secs(1));
        budget.note_run(Duration::from_secs(70));
        // 100 s left, 70 s kept for the final route: a candidate gets 30 s.
        let limit = budget
            .candidate_limit(Duration::from_secs(1200))
            .expect("room");
        assert!(limit <= Duration::from_secs(30), "{limit:?}");
        // And once the reserve is more than is left, nothing starts.
        budget.note_run(Duration::from_secs(120));
        assert_eq!(budget.candidate_limit(Duration::from_secs(1200)), None);
    }

    #[test]
    fn the_final_route_counts_against_the_budget_but_is_never_starved() {
        let budget = SearchBudget::with_limits(Duration::from_secs(900), MS(10));
        assert!(budget.final_limit(Duration::from_secs(1200)) <= Duration::from_secs(900));
        let spent = SearchBudget::with_limits(Duration::ZERO, MS(10));
        assert_eq!(
            spent.final_limit(Duration::from_secs(1200)),
            Duration::from_secs(MIN_FINAL_SECS),
            "with the budget gone the board still gets a run"
        );
    }

    /// Runs that each take 80 ms against a 200 ms budget: the search stops
    /// starting runs once the budget is spent, and returns the best found.
    #[test]
    fn the_search_stops_starting_runs_when_the_budget_is_spent_and_keeps_the_best() {
        let mut budget = SearchBudget::with_limits(MS(200), MS(10));
        let mut limits = Vec::new();
        let candidates = [0_u8, 1, 2, 3, 4, 5];
        let costs = [9_u32, 4, 7, 1, 1, 1];
        let search = route_candidates::<_, _, ()>(
            &candidates,
            &mut budget,
            Duration::from_secs(1200),
            |&candidate, index, limit| {
                limits.push(limit);
                std::thread::sleep(MS(80));
                Ok(Some(Evaluated {
                    cost: costs[index],
                    order: vec![format!("net{candidate}")],
                }))
            },
        )
        .expect("no error")
        .expect("an engine");
        assert!(search.cut_short, "the budget ended the search");
        assert_eq!(search.total, 6);
        assert!(
            (2..=3).contains(&search.routed),
            "200 ms of 80 ms runs fits two or three: {}",
            search.routed
        );
        let (cost, candidate, order) = search.best.expect("the best so far is kept");
        assert_eq!((cost, candidate), (4, 1), "the cheapest of those routed");
        assert_eq!(order, ["net1"]);
        assert!(
            limits.windows(2).all(|pair| pair[1] <= pair[0]),
            "each run is given no more than the one before: {limits:?}"
        );
        assert!(
            limits[0] <= MS(200),
            "the first run is capped at the budget"
        );
    }

    #[test]
    fn a_search_that_fits_its_budget_routes_every_candidate_and_ties_keep_the_earlier() {
        let mut budget = SearchBudget::with_limits(Duration::from_secs(60), MS(10));
        let search = route_candidates::<_, _, ()>(
            &["a", "b", "c"],
            &mut budget,
            Duration::from_secs(10),
            |_, index, _| {
                Ok(Some(Evaluated {
                    cost: [2_u32, 1, 1][index],
                    order: Vec::new(),
                }))
            },
        )
        .expect("no error")
        .expect("an engine");
        assert!(!search.cut_short);
        assert_eq!(search.routed, 3);
        assert_eq!(
            search.best.expect("best").1,
            "b",
            "the earlier of two equal costs"
        );
    }

    #[test]
    fn an_unavailable_engine_ends_the_search_and_keeps_the_callers_placement() {
        let mut budget = SearchBudget::with_limits(Duration::from_secs(60), MS(10));
        let search = route_candidates::<_, u32, ()>(
            &["a", "b"],
            &mut budget,
            Duration::from_secs(10),
            |_, _, _| Ok(None),
        )
        .expect("no error");
        assert!(search.is_none());
    }

    #[test]
    fn a_budget_too_small_for_one_run_routes_nothing_and_says_so() {
        let mut budget = SearchBudget::with_limits(MS(5), Duration::from_secs(1));
        let search = route_candidates::<_, u32, ()>(
            &["a", "b"],
            &mut budget,
            Duration::from_secs(10),
            |_, _, _| panic!("no run may start"),
        )
        .expect("no error")
        .expect("an engine");
        assert!(search.cut_short);
        assert_eq!(search.routed, 0);
        assert!(search.best.is_none());
    }
}
