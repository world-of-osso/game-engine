use std::{collections::HashSet, slice::Iter, time::Duration};

use super::{RemovalRequest, SystemWorld};

const SELECTOR: &str = "--remove-system-after";
const RETIRED_FLAGS: [&str; 6] = [
    "--freeze-indirect-parameters-after",
    "--freeze-batched-instances-after",
    "--freeze-gpu-clusters-after",
    "--freeze-mesh-collection-after",
    "--freeze-camera-follow-after",
    "--freeze-message-send-after",
];

pub(super) fn parse_requests(arguments: &[String]) -> Result<Vec<RemovalRequest>, String> {
    let mut requests = Vec::new();
    let mut seen = HashSet::new();
    let mut arguments = arguments.iter();
    while let Some(argument) = arguments.next() {
        if RETIRED_FLAGS.contains(&argument.as_str()) {
            return Err(format!("{argument} was removed; use {SELECTOR}"));
        }
        if argument != SELECTOR {
            continue;
        }
        let request = parse_request(&mut arguments)?;
        let key = (
            request.world,
            request.schedule.clone(),
            request.system.clone(),
        );
        if !seen.insert(key) {
            return Err(format!(
                "duplicate removal for {:?}:{} {:?}",
                request.world, request.schedule, request.system
            ));
        }
        requests.push(request);
    }
    requests.sort_by_key(|request| request.deadline);
    Ok(requests)
}

fn parse_request(arguments: &mut Iter<'_, String>) -> Result<RemovalRequest, String> {
    let location = arguments
        .next()
        .ok_or("missing main:SCHEDULE or render:SCHEDULE")?;
    let system = arguments.next().ok_or("missing exact system name")?;
    let seconds = arguments.next().ok_or("missing removal seconds")?;
    let (owner, schedule) = location
        .split_once(':')
        .ok_or("schedule must use main:SCHEDULE or render:SCHEDULE")?;
    let world = match owner {
        "main" => SystemWorld::Main,
        "render" => SystemWorld::Render,
        _ => return Err(format!("unknown world {owner:?}; expected main or render")),
    };
    if schedule.trim().is_empty() || system.trim().is_empty() {
        return Err("schedule and exact system name must be nonempty".into());
    }
    let seconds = seconds
        .parse::<u64>()
        .map_err(|_| "removal seconds must be an unsigned integer")?;
    Ok(RemovalRequest {
        world,
        schedule: schedule.to_owned(),
        system: system.clone(),
        deadline: Duration::from_secs(seconds),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(ToString::to_string).collect()
    }

    fn request(world: SystemWorld, schedule: &str, system: &str, seconds: u64) -> RemovalRequest {
        RemovalRequest {
            world,
            schedule: schedule.to_string(),
            system: system.to_string(),
            deadline: Duration::from_secs(seconds),
        }
    }

    #[test]
    fn returns_no_requests_for_unrelated_or_empty_arguments() {
        assert_eq!(parse_requests(&[]).unwrap(), Vec::<RemovalRequest>::new());
        assert_eq!(
            parse_requests(&args(&[
                "--screen",
                "inworld",
                "--server",
                "127.0.0.1:5000"
            ]))
            .unwrap(),
            Vec::<RemovalRequest>::new()
        );
    }

    #[test]
    fn parses_colon_containing_names_and_preserves_schedule() {
        let parsed = parse_requests(&args(&[
            "--remove-system-after",
            "main:FixedUpdate:Combat",
            "my_crate::combat::tick<foo::Bar>",
            "17",
        ]))
        .unwrap();

        assert_eq!(
            parsed,
            vec![request(
                SystemWorld::Main,
                "FixedUpdate:Combat",
                "my_crate::combat::tick<foo::Bar>",
                17,
            )]
        );
    }

    #[test]
    fn sorts_repeated_requests_stably_by_deadline() {
        let parsed = parse_requests(&args(&[
            "--remove-system-after",
            "render:Update",
            "later",
            "9",
            "--remove-system-after",
            "main:Update",
            "first-equal",
            "2",
            "--remove-system-after",
            "render:Last",
            "second-equal",
            "2",
        ]))
        .unwrap();

        assert_eq!(
            parsed,
            vec![
                request(SystemWorld::Main, "Update", "first-equal", 2),
                request(SystemWorld::Render, "Last", "second-equal", 2),
                request(SystemWorld::Render, "Update", "later", 9),
            ]
        );
    }

    #[test]
    fn rejects_malformed_requests() {
        for values in [
            vec!["--remove-system-after"],
            vec!["--remove-system-after", "main:Update"],
            vec!["--remove-system-after", "main:Update", "system"],
            vec!["--remove-system-after", "other:Update", "system", "1"],
            vec!["--remove-system-after", "main:", "system", "1"],
            vec!["--remove-system-after", "main:   ", "system", "1"],
            vec!["--remove-system-after", "main:Update", "", "1"],
            vec!["--remove-system-after", "main:Update", "  ", "1"],
            vec!["--remove-system-after", "main:Update", "system", "-1"],
            vec!["--remove-system-after", "main:Update", "system", "one"],
        ] {
            assert!(parse_requests(&args(&values)).is_err(), "{values:?}");
        }
    }

    #[test]
    fn rejects_duplicate_requests() {
        assert!(
            parse_requests(&args(&[
                "--remove-system-after",
                "main:Update",
                "system",
                "1",
                "--remove-system-after",
                "main:Update",
                "system",
                "2",
            ]))
            .is_err()
        );
    }

    #[test]
    fn rejects_retired_flags() {
        for flag in [
            "--freeze-indirect-parameters-after",
            "--freeze-batched-instances-after",
            "--freeze-gpu-clusters-after",
            "--freeze-mesh-collection-after",
            "--freeze-camera-follow-after",
            "--freeze-message-send-after",
        ] {
            assert!(parse_requests(&args(&[flag])).is_err(), "{flag}");
        }
    }
}
