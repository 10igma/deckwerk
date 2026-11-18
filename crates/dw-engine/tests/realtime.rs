//! ENG-06: Im Audio-Pfad wird kein Speicher angefordert oder freigegeben.

mod common;

use common::*;
use dw_core::{Band, Command, DeckId, SyncMode};
use dw_engine::Engine;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

thread_local! {
    static TRACKING: Cell<bool> = const { Cell::new(false) };
    static COUNT: Cell<usize> = const { Cell::new(0) };
}

/// Zählt Speicheranforderungen und -freigaben des Threads, solange `TRACKING` gesetzt ist.
struct Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if TRACKING.with(|t| t.get()) {
            COUNT.with(|c| c.set(c.get() + 1));
        }
        System.alloc(layout)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if TRACKING.with(|t| t.get()) {
            COUNT.with(|c| c.set(c.get() + 1));
        }
        System.dealloc(ptr, layout)
    }
}

#[global_allocator]
static ALLOC: Counting = Counting;

#[test]
fn process_and_commands_do_not_allocate() {
    use DeckId::{A, B};
    let mut e = Engine::new(SR);
    e.load(A, click_track(128.0, 0.0, 200, 2));
    e.load(B, noise_track(3, 60.0, 0.2));
    // Vormerken geschieht außerhalb des Audio-Threads.
    for i in 0..500 {
        e.schedule(4.0 + i as f64 * 0.25, Command::Filter { deck: B, pos: (i % 20) as f32 / 20.0 - 0.5 });
    }
    e.schedule(16.0, Command::Seek { deck: B, pos: 12.0 });
    e.schedule(24.0, Command::Play { deck: A, on: false });
    e.schedule(32.0, Command::Play { deck: A, on: true });
    let mut m = vec![0.0f32; 512];
    let mut c = vec![0.0f32; 512];

    TRACKING.with(|t| t.set(true));
    e.apply(Command::Sync { deck: B, mode: SyncMode::Beat });
    e.apply(Command::Play { deck: A, on: true });
    e.apply(Command::Play { deck: B, on: true });
    for i in 0..6000 {
        if i == 1000 {
            e.apply(Command::Eq { deck: A, band: Band::Low, db: f32::NEG_INFINITY });
            e.apply(Command::Crossfader { pos: 0.5 });
            e.apply(Command::Sync { deck: B, mode: SyncMode::Off });
        }
        if i == 2000 {
            e.apply(Command::Sync { deck: B, mode: SyncMode::Beat });
            e.apply(Command::SetMaster { deck: Some(B) });
        }
        e.process(&mut m, &mut c);
        if i % 100 == 0 {
            let _ = e.snapshot();
        }
    }
    TRACKING.with(|t| t.set(false));

    let count = COUNT.with(|c| c.get());
    assert_eq!(count, 0, "{count} Speicheranforderungen im Audio-Pfad");
    assert!(e.clock_beats() > 60.0);
}
