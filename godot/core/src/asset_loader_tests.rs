use std::sync::mpsc;
use std::time::{Duration, Instant};

use super::*;

/// Poll until `count` results arrive or two seconds pass.
fn wait_for<K, T>(loader: &mut AssetLoader<K, T>, count: usize) -> Vec<(K, Result<T, String>)>
where
    K: Clone + Eq + Hash + Send + 'static,
    T: Send + 'static,
{
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut finished = Vec::new();
    while finished.len() < count && Instant::now() < deadline {
        finished.extend(loader.poll());
        thread::sleep(Duration::from_millis(1));
    }
    finished
}

#[test]
fn a_requested_key_loads_off_the_calling_thread_and_poll_hands_it_out_once() {
    let caller = thread::current().id();
    let mut loader = AssetLoader::new("test", 1, move |&fdid: &u32| {
        if thread::current().id() == caller {
            return Err("loaded on the caller".into());
        }
        Ok(fdid * 10)
    });
    assert!(loader.request(1_237_495, Priority::Now));
    assert_eq!(loader.state(&1_237_495), Some(LoadState::Loading));
    assert_eq!(wait_for(&mut loader, 1), vec![(1_237_495, Ok(12_374_950))]);
    assert_eq!(loader.state(&1_237_495), Some(LoadState::Done));
    assert!(loader.poll().is_empty());
}

#[test]
fn polling_never_blocks_on_a_load_in_progress() {
    let (release, gate) = mpsc::channel::<()>();
    let gate = Mutex::new(gate);
    let mut loader = AssetLoader::new("test", 1, move |&fdid: &u32| {
        gate.lock().unwrap().recv().unwrap();
        Ok(fdid)
    });
    loader.request(1_713_682, Priority::Now);
    let started = Instant::now();
    assert!(loader.poll().is_empty());
    assert!(started.elapsed() < Duration::from_millis(100));
    assert_eq!(loader.loading(), 1);
    release.send(()).unwrap();
    assert_eq!(wait_for(&mut loader, 1), vec![(1_713_682, Ok(1_713_682))]);
    assert_eq!(loader.loading(), 0);
}

#[test]
fn a_key_is_loaded_once_however_often_it_is_requested() {
    let (count, counted) = mpsc::channel();
    let mut loader = AssetLoader::new("test", 2, move |&fdid: &u32| {
        count.send(fdid).unwrap();
        Ok(())
    });
    assert!(loader.request(2_467_327, Priority::Now));
    assert!(!loader.request(2_467_327, Priority::Now));
    wait_for(&mut loader, 1);
    // Done keys stay known: a later use does not load them again.
    assert!(!loader.request(2_467_327, Priority::Now));
    thread::sleep(Duration::from_millis(20));
    assert_eq!(counted.try_iter().collect::<Vec<_>>(), vec![2_467_327]);
}

#[test]
fn a_failed_load_is_reported_with_its_error() {
    let mut loader = AssetLoader::new("test", 1, |&fdid: &u32| -> Result<(), String> {
        Err(format!("Failed to cache local CASC FDID {fdid}"))
    });
    loader.request(99, Priority::Now);
    assert_eq!(
        wait_for(&mut loader, 1),
        vec![(99, Err("Failed to cache local CASC FDID 99".to_string()))]
    );
    assert_eq!(loader.state(&99), Some(LoadState::Done));
}

#[test]
fn a_run_job_runs_before_loads_queued_after_it() {
    let (order, ordered) = mpsc::channel();
    let warm = order.clone();
    let mut loader = AssetLoader::new("test", 1, move |&fdid: &u32| {
        order.send(fdid).unwrap();
        Ok(())
    });
    loader.run(move || warm.send(0).unwrap(), Priority::Now);
    loader.request(7, Priority::Now);
    wait_for(&mut loader, 1);
    assert_eq!(ordered.try_iter().collect::<Vec<_>>(), vec![0, 7]);
}

#[test]
fn dropping_the_loader_skips_queued_jobs() {
    let (release, gate) = mpsc::channel::<()>();
    let gate = Mutex::new(gate);
    let (began, begun) = mpsc::channel();
    let began = Mutex::new(began);
    let (ran, runs) = mpsc::channel();
    let mut loader = AssetLoader::new("test", 1, move |&fdid: &u32| {
        if fdid == 1 {
            began.lock().unwrap().send(()).unwrap();
            gate.lock().unwrap().recv().unwrap();
        }
        ran.send(fdid).unwrap();
        Ok(())
    });
    for fdid in 1..50 {
        loader.request(fdid, Priority::Now);
    }
    // The worker holds the first load while the rest wait in the queue.
    begun.recv_timeout(Duration::from_secs(2)).unwrap();
    let releaser = thread::spawn(move || {
        thread::sleep(Duration::from_millis(20));
        release.send(()).unwrap();
    });
    drop(loader);
    releaser.join().unwrap();
    // The worker ends after its load, dropping the load function and its sender.
    assert_eq!(runs.iter().collect::<Vec<_>>(), vec![1]);
}

/// Dropping the loader returns while a worker still holds a load: the owner (the
/// character select campsite, left on Enter World) never waits for it.
#[test]
fn dropping_the_loader_does_not_wait_for_the_load_in_hand() {
    let (release, gate) = mpsc::channel::<()>();
    let gate = Mutex::new(gate);
    let (began, begun) = mpsc::channel();
    let began = Mutex::new(began);
    let (ran, runs) = mpsc::channel();
    let mut loader = AssetLoader::new("test", 1, move |&fdid: &u32| {
        if fdid == 189_077 {
            began.lock().unwrap().send(()).unwrap();
            gate.lock().unwrap().recv().unwrap();
        }
        ran.send(fdid).unwrap();
        Ok(())
    });
    loader.request(189_077, Priority::Now);
    loader.request(189_078, Priority::Now);
    begun.recv_timeout(Duration::from_secs(2)).unwrap();
    let releaser = thread::spawn(move || {
        thread::sleep(Duration::from_secs(1));
        release.send(()).unwrap();
    });
    let dropping = Instant::now();
    drop(loader);
    let waited = dropping.elapsed();
    releaser.join().unwrap();
    assert_eq!(runs.iter().collect::<Vec<_>>(), vec![189_077]);
    assert!(
        waited < Duration::from_millis(500),
        "drop waited {waited:?} for the load in hand"
    );
}

#[test]
fn loads_needed_now_go_ahead_of_prefetches_and_a_needed_prefetch_moves_up() {
    let (release, gate) = mpsc::channel::<()>();
    let gate = Mutex::new(gate);
    let (began, begun) = mpsc::channel();
    let began = Mutex::new(began);
    let (order, ordered) = mpsc::channel();
    let mut loader = AssetLoader::new("test", 1, move |&fdid: &u32| {
        if fdid == 1 {
            began.lock().unwrap().send(()).unwrap();
            gate.lock().unwrap().recv().unwrap();
        }
        order.send(fdid).unwrap();
        Ok(())
    });
    loader.request(1, Priority::Now);
    begun.recv_timeout(Duration::from_secs(2)).unwrap();
    // Prefetches of two spells' models, then a cast's own sound.
    loader.request(2, Priority::Later);
    loader.request(3, Priority::Later);
    loader.request(4, Priority::Now);
    // The cast now needs prefetched model 3.
    assert!(!loader.request(3, Priority::Now));
    release.send(()).unwrap();
    wait_for(&mut loader, 4);
    assert_eq!(ordered.try_iter().collect::<Vec<_>>(), vec![1, 4, 3, 2]);
}

/// The loading screen's center tile parses after its neighbours have queued their models:
/// its loads, new or already queued behind theirs, run before every other queued load.
#[test]
fn loads_the_loading_screen_waits_for_go_ahead_of_every_queued_load() {
    let (release, gate) = mpsc::channel::<()>();
    let gate = Mutex::new(gate);
    let (began, begun) = mpsc::channel();
    let began = Mutex::new(began);
    let (order, ordered) = mpsc::channel();
    let mut loader = AssetLoader::new("test", 1, move |&fdid: &u32| {
        if fdid == 1 {
            began.lock().unwrap().send(()).unwrap();
            gate.lock().unwrap().recv().unwrap();
        }
        order.send(fdid).unwrap();
        Ok(())
    });
    loader.request(1, Priority::Now);
    begun.recv_timeout(Duration::from_secs(2)).unwrap();
    // Neighbour tiles' models, and a prefetch.
    for fdid in [2, 3, 4] {
        loader.request(fdid, Priority::Now);
    }
    loader.request(5, Priority::Later);
    // The center tile needs model 4, already queued, and model 6, not yet requested.
    assert!(!loader.request(4, Priority::First));
    assert!(loader.request(6, Priority::First));
    release.send(()).unwrap();
    wait_for(&mut loader, 6);
    assert_eq!(
        ordered.try_iter().collect::<Vec<_>>(),
        vec![1, 6, 4, 2, 3, 5]
    );
}
