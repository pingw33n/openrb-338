//! Task-082: the memory a song's event lists reserve. The DLL takes every node of every list
//! from one pool of `POOL_NODES` nodes (`sub_100142C0`); rb reserves one such pool per song
//! (`rb_engine::seq::Pool`) and one per copy of the tracks the callback hands the UI
//! (`rb_player::Tracks`, three in the triple buffer). This test binary's allocator counts the
//! live bytes of the allocating thread, and the test prints and bounds what a sequencer, a
//! tracks copy and a whole player (its empty song, its three tracks copies, its queues and
//! snapshots) hold.

mod common;

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use common::tables;
use rb_engine::seq::{Event, MERGE_CHUNK, POOL_NODES, Sequencer};
use rb_player::{Player, Tracks};

struct Live;

thread_local! {
    static BYTES: Cell<isize> = const { Cell::new(0) };
}

fn add(n: isize) {
    let _ = BYTES.try_with(|b| b.set(b.get() + n));
}

unsafe impl GlobalAlloc for Live {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        add(l.size() as isize);
        unsafe { System.alloc(l) }
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        add(l.size() as isize);
        unsafe { System.alloc_zeroed(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        add(-(l.size() as isize));
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        add(n as isize - l.size() as isize);
        unsafe { System.realloc(p, l, n) }
    }
}

#[global_allocator]
static A: Live = Live;

/// The bytes `f`'s result holds (allocated by `f` and still live after it).
fn held<T>(f: impl FnOnce() -> T) -> (usize, T) {
    let before = BYTES.with(|b| b.get());
    let v = f();
    ((BYTES.with(|b| b.get()) - before) as usize, v)
}

#[test]
fn a_song_reserves_one_pool() {
    let tables = tables();
    let pool = POOL_NODES * size_of::<Event>();
    let (seq, s) = held(Sequencer::default);
    let (copy, t) = held(Tracks::default);
    let (player, p) = held(|| Player::new(tables.clone(), None));
    println!("one pool {pool} B; Sequencer {seq} B; Tracks {copy} B; Player::new {player} B");
    drop((s, t, p));
    // and the merge's buffer (task-084)
    assert_eq!(
        seq,
        pool + MERGE_CHUNK * size_of::<Event>(),
        "a sequencer holds one pool"
    );
    assert_eq!(copy, pool, "a tracks copy holds one pool");
    // the empty song's pool, the three tracks copies, and the rest (devices, queues, snapshots:
    // 700188 B when measured, task-082)
    assert!(
        player > 4 * pool && player < 4 * pool + 1_000_000,
        "{player}"
    );
}
