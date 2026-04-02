use super::*;
use std::path::Path;

#[test]
fn persists_lookup_results_in_local_cache() {
    let test_dir = Path::new("target/test-artifacts/listfile");
    std::fs::create_dir_all(test_dir).unwrap();
    let community = test_dir.join("community.csv");
    let local = test_dir.join("local-cache.sqlite");
    let _ = std::fs::remove_file(&local);
    std::fs::write(&community, "123;world/maps/test/test_1_2.adt\n").unwrap();

    let community_cache = test_dir.join("community.sqlite");
    let listfile = Listfile::new(community.clone(), community_cache.clone(), local.clone());
    assert!(listfile.local.lock().unwrap().by_fdid.is_empty());
    assert_eq!(
        listfile.lookup_path("world/maps/test/test_1_2.adt"),
        Some(123)
    );
    assert_eq!(
        listfile.lookup_fdid(123),
        Some("world/maps/test/test_1_2.adt")
    );

    let persisted = listfile_cache::load_local_cache(&local).unwrap();
    assert_eq!(
        persisted.by_fdid.get(&123).copied(),
        Some("world/maps/test/test_1_2.adt")
    );

    std::fs::remove_file(&community).unwrap();
    let cached_only = Listfile::new(community, community_cache, local.clone());
    assert_eq!(
        cached_only.lookup_path("world/maps/test/test_1_2.adt"),
        Some(123)
    );
    assert_eq!(
        cached_only.lookup_fdid(123),
        Some("world/maps/test/test_1_2.adt")
    );

    let _ = std::fs::remove_file(local);
}

#[test]
fn lookup_fdid_persists_reverse_lookup_results_in_local_cache() {
    let test_dir = Path::new("target/test-artifacts/listfile");
    std::fs::create_dir_all(test_dir).unwrap();
    let community = test_dir.join("community-fdid-only.csv");
    let local = test_dir.join("local-cache-fdid-only.sqlite");
    let _ = std::fs::remove_file(&local);
    std::fs::write(&community, "456;creature/test/test.m2\n").unwrap();

    let community_cache = test_dir.join("community-fdid-only.sqlite");
    let listfile = Listfile::new(community.clone(), community_cache.clone(), local.clone());
    assert!(listfile.local.lock().unwrap().by_fdid.is_empty());
    assert_eq!(listfile.lookup_fdid(456), Some("creature/test/test.m2"));
    let persisted = listfile_cache::load_local_cache(&local).unwrap();
    assert_eq!(
        persisted.by_fdid.get(&456).copied(),
        Some("creature/test/test.m2")
    );

    std::fs::remove_file(&community).unwrap();
    let cached_only = Listfile::new(community, community_cache, local.clone());
    assert_eq!(cached_only.lookup_fdid(456), Some("creature/test/test.m2"));
    assert_eq!(cached_only.lookup_path("creature/test/test.m2"), Some(456));

    let _ = std::fs::remove_file(local);
}

#[test]
fn local_sqlite_cache_is_loaded_on_demand() {
    let test_dir = Path::new("target/test-artifacts/listfile");
    std::fs::create_dir_all(test_dir).unwrap();
    let community = test_dir.join("community-on-demand.csv");
    let local = test_dir.join("local-cache-on-demand.sqlite");
    let _ = std::fs::remove_file(&local);
    std::fs::write(&community, "").unwrap();
    listfile_cache::remember_local_cache_entry(&local, 789, "world/test/on_demand.m2").unwrap();

    let community_cache = test_dir.join("community-on-demand.sqlite");
    let listfile = Listfile::new(community, community_cache, local.clone());
    assert!(listfile.local.lock().unwrap().by_fdid.is_empty());

    assert_eq!(listfile.lookup_fdid(789), Some("world/test/on_demand.m2"));
    assert_eq!(listfile.lookup_path("world/test/on_demand.m2"), Some(789));
    assert_eq!(
        listfile.local.lock().unwrap().by_fdid.get(&789).copied(),
        Some("world/test/on_demand.m2")
    );

    let _ = std::fs::remove_file(local);
}
