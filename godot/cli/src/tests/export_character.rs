use super::*;

#[test]
fn export_character_command_maps_to_request() {
    let request = export_character_request(
        PathBuf::from("data/exports/thrall.json"),
        Some("Thrall".into()),
        Some(7),
    );
    assert_eq!(
        request,
        Request::ExportCharacter {
            output_path: "data/exports/thrall.json".into(),
            character_name: Some("Thrall".into()),
            character_id: Some(7),
        }
    );
}

#[test]
fn export_character_cli_command_parses_output_path() {
    let cli = crate::Cli::try_parse_from([
        "game-engine-cli",
        "export-character",
        "--name",
        "Thrall",
        "--character-id",
        "7",
        "data/exports/thrall.json",
    ])
    .expect("cli args should parse");

    assert!(matches!(
        cli.command,
        crate::Cmd::ExportCharacter {
            output,
            name,
            character_id,
        }
        if output == std::path::Path::new("data/exports/thrall.json")
            && name == Some("Thrall".into())
            && character_id == Some(7)
    ));
}
