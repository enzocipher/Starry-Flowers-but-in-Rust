//! Opt-in desktop microbenchmarks. Run with:
//! `cargo test --release desktop_optimization_baseline -- --ignored --nocapture --test-threads=1`
use super::*;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

struct CountingAllocator;
static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        System.alloc(layout)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        System.realloc(ptr, layout, size)
    }
}
#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

fn measured<T>(f: impl FnOnce() -> T) -> (T, u64, std::time::Duration) {
    ALLOCATIONS.store(0, Ordering::SeqCst);
    let start = Instant::now();
    let result = f();
    (result, ALLOCATIONS.load(Ordering::SeqCst), start.elapsed())
}

#[test]
#[ignore = "manual performance baseline; run with --release and one test thread"]
fn desktop_optimization_baseline() {
    let story = Story::load();
    let mut state = State::default();
    state.start(&story, "start");
    let mut dialog_pcs = Vec::new();
    for _ in 0..20_000 {
        match state.run(&story) {
            Stop::Say => dialog_pcs.push(state.pc),
            Stop::Menu => state.pc = story.ops[state.pc - 1].choices[1].target,
            Stop::End => break,
            _ => {}
        }
    }
    assert!(
        dialog_pcs.len() > 1_000,
        "fixture did not traverse enough dialogue"
    );

    let mut preferences = Preferences::default();
    for pc in &dialog_pcs {
        preferences.mark_read(*pc);
    }
    let lookups = 10_000usize;
    let read_entries = preferences.read.clone();
    let (old_hits, old_read_allocs, old_read_time) = measured(|| {
        (0..lookups)
            .filter(|i| {
                let pc = if i % 2 == 0 {
                    dialog_pcs[i % dialog_pcs.len()]
                } else {
                    story.ops.len() + i
                };
                read_entries.contains(&pc)
            })
            .count()
    });
    let (hits, read_allocs, read_time) = measured(|| {
        (0..lookups)
            .filter(|i| {
                let pc = if i % 2 == 0 {
                    dialog_pcs[i % dialog_pcs.len()]
                } else {
                    story.ops.len() + i
                };
                preferences.can_skip(pc)
            })
            .count()
    });

    let sprite = Sprite {
        tag: "pastille".into(),
        attrs: vec!["witch".into(), "smile".into(), "dress".into()],
        position: "center".into(),
    };
    let mut stamp = SpriteStamp::new(&sprite, 0.0);
    let (_, old_age_allocs, old_age_time) = measured(|| {
        let mut sink = 0usize;
        for _ in 0..100_000 {
            let attrs: Vec<_> = sprite_identity_attrs(std::hint::black_box(&sprite)).collect();
            let identity = format!("{attrs:?}:{}", sprite.position);
            sink ^= std::hint::black_box(identity.len());
        }
        sink
    });
    let (_, age_allocs, age_time) = measured(|| {
        for _ in 0..100_000 {
            stamp.refresh_if_changed(std::hint::black_box(&sprite), std::hint::black_box(1.0));
        }
        stamp.started_at
    });

    let mut dialogue_texts: Vec<&str> = story
        .ops
        .iter()
        .filter(|op| op.op == "say" && !op.text.is_empty())
        .map(|op| op.text.as_str())
        .collect();
    dialogue_texts.sort_by_key(|s| std::cmp::Reverse(s.chars().count()));
    dialogue_texts.truncate(80);
    let width = 420.0;
    let (layouts, wrap_allocs, wrap_time) = measured(|| {
        dialogue_texts
            .iter()
            .map(|text| layout::wrap(text, width, |s| s.chars().count() as f32 * 10.0))
            .collect::<Vec<_>>()
    });
    let cached_lines: Vec<layout::Line> = layouts.into_iter().flatten().collect();
    let cache_iterations = 10_000usize;
    let (cloned, cache_allocs, cache_time) = measured(|| {
        let mut total = 0usize;
        for _ in 0..cache_iterations {
            let copy = cached_lines.clone();
            total = total.wrapping_add(copy.iter().map(|line| line.text.len()).sum::<usize>());
        }
        total
    });
    let arc: std::sync::Arc<[layout::Line]> = cached_lines.into();
    let (_, arc_allocs, arc_time) = measured(|| {
        for _ in 0..cache_iterations {
            std::hint::black_box(arc.clone());
        }
    });

    println!("fixture_dialogues={}", dialog_pcs.len());
    println!("read_lookup_count={lookups} hits={old_hits}/{hits} vec_allocations={old_read_allocs} vec_ms={:.3} hash_allocations={read_allocs} hash_ms={:.3}", old_read_time.as_secs_f64() * 1000.0, read_time.as_secs_f64() * 1000.0);
    println!("age_calls=100000 old_allocations={old_age_allocs} old_ms={:.3} current_allocations={age_allocs} current_ms={:.3}", old_age_time.as_secs_f64() * 1000.0, age_time.as_secs_f64() * 1000.0);
    println!(
        "cold_wrap_texts={} allocations={wrap_allocs} elapsed_ms={:.3}",
        dialogue_texts.len(),
        wrap_time.as_secs_f64() * 1000.0
    );
    println!("cached_lines={} clone_iterations={cache_iterations} clone_allocations={cache_allocs} clone_ms={:.3} arc_allocations={arc_allocs} arc_ms={:.3} checksum={cloned}", arc.len(), cache_time.as_secs_f64() * 1000.0, arc_time.as_secs_f64() * 1000.0);
}
