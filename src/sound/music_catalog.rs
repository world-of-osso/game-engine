use std::collections::HashMap;

#[cfg(test)]
use crate::csv_util::parse_csv_line;
#[cfg(test)]
use std::collections::HashSet;

pub fn load_zone_music_catalog(
    track_index_by_fdid: &HashMap<u32, usize>,
) -> HashMap<u32, Vec<usize>> {
    match game_engine::sound_music_zone_cache::load_zone_music_catalog(track_index_by_fdid) {
        Ok(by_zone) => by_zone,
        Err(err) => {
            eprintln!("Failed to load music zone catalog: {err}");
            HashMap::new()
        }
    }
}

#[cfg(test)]
fn insert_zone_music_link(
    line: &str,
    track_index_by_fdid: &HashMap<u32, usize>,
    by_zone: &mut HashMap<u32, Vec<usize>>,
    seen: &mut HashMap<u32, HashSet<usize>>,
) {
    let fields = parse_csv_line(line);
    if fields.len() < 12 || fields[2] != "1" {
        return;
    }
    let Ok(fdid) = fields[0].parse::<u32>() else {
        return;
    };
    let Ok(area_id) = fields[9].parse::<u32>() else {
        return;
    };
    let Some(&track_idx) = track_index_by_fdid.get(&fdid) else {
        return;
    };
    if seen.entry(area_id).or_default().insert(track_idx) {
        by_zone.entry(area_id).or_default().push(track_idx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_parser_handles_quotes() {
        let fields =
            parse_csv_line("1,mp3,1,\"sound/music/citymusic/stormwind/stormwind03-moment.mp3\"");
        assert_eq!(fields[0], "1");
        assert_eq!(fields[2], "1");
        assert_eq!(
            fields[3],
            "sound/music/citymusic/stormwind/stormwind03-moment.mp3"
        );
    }
}
