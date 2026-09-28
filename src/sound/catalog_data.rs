use std::collections::{HashMap, HashSet};
use std::io::BufRead;
use std::path::Path;

use crate::csv_util::{header_index, parse_csv_line};

pub type TracksByZone = HashMap<u32, Vec<usize>>;

/// The authored music-zone row stores FileDataID in column 0 and area ID in column 9.
pub fn parse_music_zone_link(line: &str) -> Option<(u32, u32)> {
    if line.is_empty() {
        return None;
    }
    let fields = parse_csv_line(line);
    if fields.len() < 12 || fields[2] != "1" {
        return None;
    }
    Some((fields[0].parse().ok()?, fields[9].parse().ok()?))
}

pub fn read_music_zone_catalog<R: BufRead>(
    mut reader: R,
    track_index_by_fdid: &HashMap<u32, usize>,
) -> Result<TracksByZone, String> {
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .map_err(|err| format!("read music_zone_links header: {err}"))?;
    let mut by_zone = HashMap::new();
    let mut seen: HashMap<u32, HashSet<usize>> = HashMap::new();
    loop {
        line.clear();
        if reader
            .read_line(&mut line)
            .map_err(|err| format!("read music_zone_links row: {err}"))?
            == 0
        {
            break;
        }
        let Some((fdid, area_id)) = parse_music_zone_link(line.trim_end_matches(['\r', '\n']))
        else {
            continue;
        };
        let Some(&track_idx) = track_index_by_fdid.get(&fdid) else {
            continue;
        };
        insert_zone_track(&mut by_zone, &mut seen, area_id, track_idx);
    }
    Ok(by_zone)
}

pub fn read_ambient_zone_catalog<R: BufRead>(
    mut reader: R,
    path: &Path,
    track_index_by_fdid: &HashMap<u32, usize>,
) -> Result<TracksByZone, String> {
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .map_err(|err| format!("read {} header: {err}", path.display()))?;
    let headers = parse_csv_line(line.trim_end_matches(['\r', '\n']));
    let fdid_idx = header_index(&headers, "fdid", path)?;
    let extracted_idx = header_index(&headers, "extracted", path)?;
    let wow_path_idx = header_index(&headers, "wow_path", path)?;
    let area_ids_idx = header_index(&headers, "area_ids", path)?;

    let mut by_zone = HashMap::new();
    let mut seen: HashMap<u32, HashSet<usize>> = HashMap::new();
    loop {
        line.clear();
        if reader
            .read_line(&mut line)
            .map_err(|err| format!("read {} row: {err}", path.display()))?
            == 0
        {
            break;
        }
        let fields = parse_csv_line(line.trim_end_matches(['\r', '\n']));
        let Some(track_idx) = ambient_track_index(
            &fields,
            wow_path_idx,
            extracted_idx,
            fdid_idx,
            track_index_by_fdid,
        ) else {
            continue;
        };
        let Some(area_ids) = fields.get(area_ids_idx) else {
            continue;
        };
        for area_id in area_ids
            .split('|')
            .filter_map(|value| value.parse::<u32>().ok())
        {
            insert_zone_track(&mut by_zone, &mut seen, area_id, *track_idx);
        }
    }
    Ok(by_zone)
}

fn ambient_track_index<'a>(
    fields: &[String],
    wow_path_idx: usize,
    extracted_idx: usize,
    fdid_idx: usize,
    track_index_by_fdid: &'a HashMap<u32, usize>,
) -> Option<&'a usize> {
    let wow_path = fields.get(wow_path_idx)?;
    if !wow_path.to_ascii_lowercase().contains("ambient")
        || fields.get(extracted_idx).map(String::as_str) != Some("1")
    {
        return None;
    }
    fields
        .get(fdid_idx)
        .and_then(|field| field.parse::<u32>().ok())
        .and_then(|fdid| track_index_by_fdid.get(&fdid))
}

fn insert_zone_track(
    by_zone: &mut TracksByZone,
    seen: &mut HashMap<u32, HashSet<usize>>,
    area_id: u32,
    track_idx: usize,
) {
    if seen.entry(area_id).or_default().insert(track_idx) {
        by_zone.entry(area_id).or_default().push(track_idx);
    }
}

pub fn strip_ambient_tracks_from_music_catalog(
    music_tracks_by_zone: &mut TracksByZone,
    ambient_tracks_by_zone: &TracksByZone,
) {
    for (zone_id, music_indices) in music_tracks_by_zone.iter_mut() {
        let Some(ambient_indices) = ambient_tracks_by_zone.get(zone_id) else {
            continue;
        };
        music_indices.retain(|track_idx| !ambient_indices.contains(track_idx));
    }
    music_tracks_by_zone.retain(|_, track_indices| !track_indices.is_empty());
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn music_link_requires_twelve_fields_extracted_and_numeric_ids() {
        assert_eq!(
            parse_music_zone_link("42,x,1,a,b,c,d,e,f,7210,g,h"),
            Some((42, 7210))
        );
        assert_eq!(parse_music_zone_link("42,x,0,a,b,c,d,e,f,7210,g,h"), None);
        assert_eq!(parse_music_zone_link("42,x,1,a,b,c,d,e,f,7210,g"), None);
        assert_eq!(parse_music_zone_link("bad,x,1,a,b,c,d,e,f,7210,g,h"), None);
        assert_eq!(parse_music_zone_link("42,x,1,a,b,c,d,e,f,bad,g,h"), None);
    }

    #[test]
    fn music_reader_skips_header_and_preserves_encounter_order() {
        let csv = "fdid,unused,extracted,a,b,c,d,e,f,area,g,h\n\
                   20,x,1,a,b,c,d,e,f,5,g,h\n\
                   10,x,1,a,b,c,d,e,f,5,g,h\n\
                   20,x,1,a,b,c,d,e,f,5,g,h\n\
                   30,x,1,a,b,c,d,e,f,5,g,h\n";
        let indices = HashMap::from([(20, 2), (10, 1)]);
        let zones = read_music_zone_catalog(Cursor::new(csv), &indices).unwrap();
        assert_eq!(zones.get(&5), Some(&vec![2, 1]));
    }

    #[test]
    fn ambient_reader_uses_reordered_named_headers_and_deduplicates_in_encounter_order() {
        let csv = "area_ids,wow_path,extracted,fdid\n\
                   5|6|bad|5,sound/AMBIENT/wind.mp3,1,20\n\
                   5,sound/ambient/birds.mp3,1,10\n\
                   5,sound/ambient/wind.mp3,1,20\n\
                   5,sound/music/song.mp3,1,10\n\
                   5,sound/ambient/missing.mp3,1,30\n\
                   5,sound/ambient/off.mp3,0,10\n\
                   malformed\n";
        let indices = HashMap::from([(20, 2), (10, 1)]);
        let zones =
            read_ambient_zone_catalog(Cursor::new(csv), Path::new("manifest.csv"), &indices)
                .unwrap();
        assert_eq!(zones.get(&5), Some(&vec![2, 1]));
        assert_eq!(zones.get(&6), Some(&vec![2]));
    }

    #[test]
    fn ambient_reader_reports_missing_named_header() {
        let csv = "fdid,extracted,wow_path\n20,1,sound/ambient/wind.mp3\n";
        let err =
            read_ambient_zone_catalog(Cursor::new(csv), Path::new("manifest.csv"), &HashMap::new())
                .unwrap_err();
        assert!(
            err.contains("manifest.csv missing area_ids column"),
            "{err}"
        );
    }

    #[test]
    fn overlap_removal_preserves_music_order_and_drops_empty_zones() {
        let mut music = HashMap::from([(5, vec![1, 2, 3, 2]), (6, vec![4]), (7, vec![8])]);
        let ambient = HashMap::from([(5, vec![2]), (6, vec![4]), (8, vec![8])]);
        strip_ambient_tracks_from_music_catalog(&mut music, &ambient);
        assert_eq!(music, HashMap::from([(5, vec![1, 3]), (7, vec![8])]));
    }
}
