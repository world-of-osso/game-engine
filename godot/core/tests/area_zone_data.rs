use game_engine_core::area_zone_data::{load_area_parents, root_area};
use std::io::{self, BufReader, Cursor, Read};
use std::path::Path;

const AREA_TABLE: &str = "AreaName_lang,ParentAreaID,ID\nNorthshire Valley,6170,9\nNorthshire,12,6170\nElwynn Forest,0,12\nGoldshire,12,87\n";

#[test]
fn reordered_columns_and_northshire_ancestors() {
    let parents = load_area_parents(Cursor::new(AREA_TABLE), Path::new("AreaTable.csv")).unwrap();
    assert_eq!(parents.len(), 3);
    assert_eq!(parents.get(&9), Some(&6170));
    assert_eq!(parents.get(&6170), Some(&12));
    assert!(!parents.contains_key(&12));
    assert_eq!(root_area(&parents, 9), 12);
    assert_eq!(root_area(&parents, 6170), 12);
    assert_eq!(root_area(&parents, 87), 12);
    assert_eq!(root_area(&parents, 12), 12);
    assert_eq!(root_area(&parents, 99999), 99999);
}

#[test]
fn malformed_rows_are_skipped_without_rejecting_valid_rows() {
    let csv = "ID,ParentAreaID\nwrong,12\n20,nope\n21\n22,0\n23,12\n";
    let parents = load_area_parents(Cursor::new(csv), Path::new("AreaTable.csv")).unwrap();
    assert_eq!(parents.len(), 1);
    assert_eq!(parents.get(&23), Some(&12));
}

#[test]
fn missing_columns_report_the_source_path() {
    for csv in ["ID,Other\n1,2\n", "ParentAreaID,Other\n1,2\n", ""] {
        let err =
            load_area_parents(Cursor::new(csv), Path::new("missing/AreaTable.csv")).unwrap_err();
        assert!(err.contains("missing/AreaTable.csv"), "{err}");
        assert!(err.contains("missing "), "{err}");
    }
}

struct FailingRead {
    bytes: &'static [u8],
    position: usize,
}

impl Read for FailingRead {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if self.position == self.bytes.len() {
            return Err(io::Error::other("injected read failure"));
        }
        let len = output.len().min(self.bytes.len() - self.position);
        output[..len].copy_from_slice(&self.bytes[self.position..self.position + len]);
        self.position += len;
        Ok(len)
    }
}

#[test]
fn read_failures_keep_header_and_row_diagnostics() {
    for (data, stage) in [
        ("ID,ParentAreaID", "header"),
        ("ID,ParentAreaID\n1,2\n", "row"),
    ] {
        let reader = BufReader::new(FailingRead {
            bytes: data.as_bytes(),
            position: 0,
        });
        let err = load_area_parents(reader, Path::new("broken/AreaTable.csv")).unwrap_err();
        assert!(
            err.contains(&format!("read broken/AreaTable.csv {stage}:")),
            "{err}"
        );
        assert!(err.contains("injected read failure"), "{err}");
    }
}

#[test]
fn traversal_is_bounded_to_sixteen_links() {
    let parents = (1..=20).map(|id| (id, id + 1)).collect();
    assert_eq!(root_area(&parents, 1), 17);
}
