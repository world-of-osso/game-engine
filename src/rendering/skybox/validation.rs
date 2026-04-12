pub struct SkyboxValidationCase {
    pub slug: &'static str,
    pub light_skybox_id: u32,
    pub description: &'static str,
    pub output_filename: &'static str,
}

const CASES: &[SkyboxValidationCase] = &[
    SkyboxValidationCase {
        slug: "ohnahran-authored-alt-slot",
        light_skybox_id: 628,
        description: "Alternate LightParams slot authored skybox used by Ohn'ahran Overlook",
        output_filename: "skyboxdebug-ohnahran-628.webp",
    },
    SkyboxValidationCase {
        slug: "freywold-modern-authored",
        light_skybox_id: 653,
        description: "Modern authored cloud skybox used by Freywold Spring",
        output_filename: "skyboxdebug-freywold-653.webp",
    },
];

pub fn cases() -> &'static [SkyboxValidationCase] {
    CASES
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::cases;

    #[test]
    fn skybox_validation_cases_use_unique_slugs() {
        let mut seen = HashSet::new();
        for case in cases() {
            assert!(seen.insert(case.slug), "duplicate case slug: {}", case.slug);
        }
    }

    #[test]
    fn skybox_validation_cases_use_unique_output_files() {
        let mut seen = HashSet::new();
        for case in cases() {
            assert!(
                seen.insert(case.output_filename),
                "duplicate output filename: {}",
                case.output_filename
            );
        }
    }

    #[test]
    fn skybox_validation_cases_match_known_light_skybox_ids() {
        let ids: Vec<_> = cases().iter().map(|case| case.light_skybox_id).collect();
        assert_eq!(ids, vec![628, 653]);
    }
}
