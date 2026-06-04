use std::{
    cell::RefCell,
    collections::BTreeMap,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

#[derive(Debug, Default)]
struct ProfileStats {
    spans: BTreeMap<&'static str, SpanStats>,
}

#[derive(Debug, Default)]
struct SpanStats {
    calls: u64,
    total: Duration,
}

thread_local! {
    static ACTIVE: RefCell<Option<ProfileStats>> = const { RefCell::new(None) };
}

static ENABLED: AtomicBool = AtomicBool::new(false);

pub fn start() {
    ACTIVE.with(|active| {
        *active.borrow_mut() = Some(ProfileStats::default());
    });
    ENABLED.store(true, Ordering::Relaxed);
}

#[inline(always)]
pub fn measure<T>(label: &'static str, f: impl FnOnce() -> T) -> T {
    if !ENABLED.load(Ordering::Relaxed) {
        return f();
    }

    let started = Instant::now();
    let out = f();
    let elapsed = started.elapsed();

    ACTIVE.with(|active| {
        if let Some(stats) = active.borrow_mut().as_mut() {
            let span = stats.spans.entry(label).or_default();
            span.calls += 1;
            span.total += elapsed;
        }
    });

    out
}

pub fn finish(iterations: usize) -> Option<String> {
    ENABLED.store(false, Ordering::Relaxed);
    ACTIVE.with(|active| {
        let stats = active.borrow_mut().take()?;
        Some(render(&stats, iterations))
    })
}

fn render(stats: &ProfileStats, iterations: usize) -> String {
    let mut spans = stats.spans.iter().collect::<Vec<_>>();
    spans.sort_by_key(|(_, span)| std::cmp::Reverse(span.total));
    let measured_total = spans
        .iter()
        .map(|(_, span)| span.total)
        .max()
        .unwrap_or_default();

    let mut lines = vec![
        "alur profile-loop timings".to_string(),
        format!("iterations: {iterations}"),
        "span,calls,total_us,mean_us,per_iter_us,pct_of_max_span".to_string(),
    ];

    for (label, span) in spans {
        let total_us = span.total.as_secs_f64() * 1_000_000.0;
        let mean_us = total_us / span.calls.max(1) as f64;
        let per_iter_us = total_us / iterations.max(1) as f64;
        let pct = if measured_total.is_zero() {
            0.0
        } else {
            span.total.as_secs_f64() / measured_total.as_secs_f64() * 100.0
        };

        lines.push(format!(
            "{label},{},{total_us:.2},{mean_us:.3},{per_iter_us:.3},{pct:.1}",
            span.calls
        ));
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::{finish, measure, start};

    #[test]
    fn finish_returns_none_when_profile_was_not_started() {
        assert!(finish(1).is_none());
    }

    #[test]
    fn started_profile_records_measured_spans() {
        start();
        measure("demo.span", || {});
        measure("demo.span", || {});

        let output = finish(2).unwrap();
        assert!(output.contains("alur profile-loop timings"));
        assert!(output.contains("iterations: 2"));
        assert!(output.contains("demo.span,2,"));
    }
}
