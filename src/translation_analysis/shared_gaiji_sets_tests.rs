use std::path::Path;

use super::*;
use crate::translation_analysis::audit_translation_glyphs;

#[test]
#[ignore = "requires the translation segments in assets/translations"]
fn mad_demand_sets_keep_individual_and_combined_lifetimes_distinct() {
    let translations = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/translations");
    let report = audit_translation_glyphs(&translations).unwrap();

    assert_eq!(
        report
            .shared_gaiji_demand_sets
            .iter()
            .map(|set| set.scope)
            .collect::<Vec<_>>(),
        SCOPES
    );
    assert_eq!(
        report.shared_gaiji_demand_sets[0].member_segment_ids,
        ["mad-fixed-gaiji"]
    );
    assert_eq!(
        report.shared_gaiji_demand_sets[1].member_segment_ids,
        ["mad-interface"]
    );
    assert_eq!(
        report.shared_gaiji_demand_sets[2].member_segment_ids.len(),
        11
    );
    assert_eq!(
        report.shared_gaiji_demand_sets[3].member_segment_ids,
        ["mad-system", "mad-interface"]
    );
    assert_eq!(
        report.shared_gaiji_demand_sets[4].member_segment_ids.len(),
        12
    );
    assert!(
        report.shared_gaiji_demand_sets[5]
            .member_segment_ids
            .iter()
            .any(|id| id == "mad-battle-callouts")
    );
    assert_eq!(
        report.shared_gaiji_demand_sets[6].member_segment_ids,
        [
            "external-bplay6",
            "external-bsamp",
            "external-fplay6",
            "external-nmouse"
        ]
    );
    assert_eq!(
        report.shared_gaiji_demand_sets[2]
            .demand
            .available_translation_capacity,
        Some(174)
    );
    assert!(
        report
            .shared_gaiji_demand_sets
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != 2)
            .all(|(_, set)| set.demand.available_translation_capacity == Some(170))
    );
    assert!(
        report.shared_gaiji_demand_sets[3].demand.slot_glyph_count
            <= report.shared_gaiji_demand_sets[3]
                .demand
                .available_translation_capacity
    );
    assert_eq!(
        report.shared_gaiji_demand_sets[6].demand.slot_glyph_count,
        Some(200)
    );
    assert_eq!(
        report.shared_gaiji_demand_sets[3]
            .demand
            .available_capacity_fit,
        Some(true)
    );
    assert_eq!(
        report.shared_gaiji_demand_sets[2]
            .demand
            .available_capacity_fit,
        Some(true)
    );
    assert!([4, 5, 6].into_iter().all(|index| {
        report.shared_gaiji_demand_sets[index]
            .demand
            .available_capacity_fit
            == Some(false)
    }));
}
