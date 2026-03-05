use crate::model::{
    normalize_script, normalize_to_name, Diff, DiffKind, DiffSeverity, MapComparison, MapIndex,
    MapStatus, ReportSummary, ValidationReport,
};
use std::collections::BTreeSet;

pub struct CompareOptions {
    pub target_maps: Option<BTreeSet<i32>>,
    pub strict: bool,
}

pub fn compare_maps(
    nx_maps: &MapIndex,
    cosmic_maps: &MapIndex,
    options: &CompareOptions,
) -> ValidationReport {
    let map_ids = target_map_set(nx_maps, cosmic_maps, options.target_maps.as_ref());
    let mut maps = Vec::with_capacity(map_ids.len());

    let mut hard_count = 0usize;
    let mut soft_count = 0usize;
    let mut warning_count = 0usize;
    let mut exact_maps = 0usize;
    let mut maps_with_hard = 0usize;
    let mut maps_with_soft = 0usize;
    let mut maps_with_warnings = 0usize;

    for map_id in map_ids {
        let mut diffs = Vec::new();
        let nx_map = nx_maps.get(&map_id);
        let cosmic_map = cosmic_maps.get(&map_id);
        let existence_severity = if options.target_maps.is_some() {
            DiffSeverity::Hard
        } else {
            DiffSeverity::Warning
        };

        match (nx_map, cosmic_map) {
            (None, None) => {
                push_missing_map(
                    &mut diffs,
                    map_id,
                    existence_severity,
                    "map missing in both NX and Cosmic".to_string(),
                    None,
                    None,
                );
            }
            (Some(_), None) => {
                push_missing_map(
                    &mut diffs,
                    map_id,
                    existence_severity,
                    "map missing in Cosmic".to_string(),
                    Some("present".to_string()),
                    None,
                );
            }
            (None, Some(_)) => {
                push_missing_map(
                    &mut diffs,
                    map_id,
                    existence_severity,
                    "map missing in NX".to_string(),
                    None,
                    Some("present".to_string()),
                );
            }
            (Some(nx), Some(cosmic)) => {
                let portal_ids =
                    union_portal_ids(nx.portals.keys().copied(), cosmic.portals.keys().copied());
                for portal_id in portal_ids {
                    let nx_portal = nx.portals.get(&portal_id);
                    let cosmic_portal = cosmic.portals.get(&portal_id);

                    match (nx_portal, cosmic_portal) {
                        (None, Some(cosmic_portal)) => push_missing_portal(
                            &mut diffs,
                            map_id,
                            portal_id,
                            Some(cosmic_portal.name.clone()),
                            existence_severity,
                            "portal missing in NX".to_string(),
                        ),
                        (Some(nx_portal), None) => push_missing_portal(
                            &mut diffs,
                            map_id,
                            portal_id,
                            Some(nx_portal.name.clone()),
                            existence_severity,
                            "portal missing in Cosmic".to_string(),
                        ),
                        (Some(nx_portal), Some(cosmic_portal)) => {
                            compare_field(
                                &mut diffs,
                                map_id,
                                portal_id,
                                nx_portal.name.as_str(),
                                "tm",
                                nx_portal.to_map.to_string(),
                                cosmic_portal.to_map.to_string(),
                                DiffSeverity::Hard,
                            );
                            compare_field(
                                &mut diffs,
                                map_id,
                                portal_id,
                                nx_portal.name.as_str(),
                                "tn",
                                normalize_to_name(nx_portal.to_name.as_str()),
                                normalize_to_name(cosmic_portal.to_name.as_str()),
                                DiffSeverity::Hard,
                            );
                            compare_field(
                                &mut diffs,
                                map_id,
                                portal_id,
                                nx_portal.name.as_str(),
                                "x",
                                nx_portal.x.to_string(),
                                cosmic_portal.x.to_string(),
                                DiffSeverity::Soft,
                            );
                            compare_field(
                                &mut diffs,
                                map_id,
                                portal_id,
                                nx_portal.name.as_str(),
                                "y",
                                nx_portal.y.to_string(),
                                cosmic_portal.y.to_string(),
                                DiffSeverity::Soft,
                            );
                            compare_field(
                                &mut diffs,
                                map_id,
                                portal_id,
                                nx_portal.name.as_str(),
                                "pt",
                                nx_portal.portal_type.to_string(),
                                cosmic_portal.portal_type.to_string(),
                                DiffSeverity::Soft,
                            );
                            compare_field(
                                &mut diffs,
                                map_id,
                                portal_id,
                                nx_portal.name.as_str(),
                                "script",
                                format!("{:?}", normalize_script(nx_portal.script.clone())),
                                format!("{:?}", normalize_script(cosmic_portal.script.clone())),
                                DiffSeverity::Soft,
                            );
                        }
                        (None, None) => {}
                    }
                }
            }
        }

        let map_hard = diffs
            .iter()
            .filter(|d| matches!(d.severity, DiffSeverity::Hard))
            .count();
        let map_soft = diffs
            .iter()
            .filter(|d| matches!(d.severity, DiffSeverity::Soft))
            .count();
        let map_warnings = diffs
            .iter()
            .filter(|d| matches!(d.severity, DiffSeverity::Warning))
            .count();

        hard_count += map_hard;
        soft_count += map_soft;
        warning_count += map_warnings;

        let status = if map_hard > 0 {
            maps_with_hard += 1;
            MapStatus::HardMismatch
        } else if map_soft > 0 {
            maps_with_soft += 1;
            MapStatus::SoftMismatch
        } else if map_warnings > 0 {
            maps_with_warnings += 1;
            MapStatus::Warning
        } else {
            exact_maps += 1;
            MapStatus::Exact
        };

        maps.push(MapComparison {
            map_id,
            status,
            diffs,
            hard_count: map_hard,
            soft_count: map_soft,
            warning_count: map_warnings,
        });
    }

    let should_fail = hard_count > 0 || (options.strict && soft_count > 0);

    ValidationReport {
        summary: ReportSummary {
            maps_compared: maps.len(),
            exact_maps,
            maps_with_hard,
            maps_with_soft,
            maps_with_warnings,
            hard_count,
            soft_count,
            warning_count,
            should_fail,
        },
        maps,
    }
}

fn target_map_set(
    nx_maps: &MapIndex,
    cosmic_maps: &MapIndex,
    explicit_targets: Option<&BTreeSet<i32>>,
) -> BTreeSet<i32> {
    if let Some(targets) = explicit_targets {
        return targets.clone();
    }

    nx_maps
        .keys()
        .copied()
        .chain(cosmic_maps.keys().copied())
        .collect()
}

fn union_portal_ids(
    nx_portals: impl Iterator<Item = u32>,
    cosmic_portals: impl Iterator<Item = u32>,
) -> BTreeSet<u32> {
    nx_portals.chain(cosmic_portals).collect()
}

fn compare_field(
    diffs: &mut Vec<Diff>,
    map_id: i32,
    portal_id: u32,
    portal_name: &str,
    field: &str,
    nx: String,
    cosmic: String,
    severity: DiffSeverity,
) {
    if nx == cosmic {
        return;
    }

    diffs.push(Diff {
        severity,
        kind: DiffKind::FieldMismatch,
        map_id,
        portal_id: Some(portal_id),
        portal_name: Some(portal_name.to_string()),
        field: Some(field.to_string()),
        nx: Some(nx.clone()),
        cosmic: Some(cosmic.clone()),
        message: format!("field mismatch for portal {portal_id} ({portal_name}): {field}"),
    });
}

fn push_missing_map(
    diffs: &mut Vec<Diff>,
    map_id: i32,
    severity: DiffSeverity,
    message: String,
    nx: Option<String>,
    cosmic: Option<String>,
) {
    diffs.push(Diff {
        severity,
        kind: DiffKind::MissingMap,
        map_id,
        portal_id: None,
        portal_name: None,
        field: None,
        nx,
        cosmic,
        message,
    });
}

fn push_missing_portal(
    diffs: &mut Vec<Diff>,
    map_id: i32,
    portal_id: u32,
    portal_name: Option<String>,
    severity: DiffSeverity,
    message: String,
) {
    diffs.push(Diff {
        severity,
        kind: DiffKind::MissingPortal,
        map_id,
        portal_id: Some(portal_id),
        portal_name,
        field: None,
        nx: None,
        cosmic: None,
        message,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{MapRecord, PortalRecord};
    use std::collections::BTreeMap;

    #[test]
    fn explicit_targets_make_missing_map_hard() {
        let nx_maps = BTreeMap::new();
        let cosmic_maps = BTreeMap::new();

        let report = compare_maps(
            &nx_maps,
            &cosmic_maps,
            &CompareOptions {
                target_maps: Some([100000000].into_iter().collect()),
                strict: false,
            },
        );

        assert_eq!(report.summary.hard_count, 1);
        assert!(report.summary.should_fail);
    }

    #[test]
    fn soft_mismatch_fails_only_in_strict_mode() {
        let nx_portal = PortalRecord {
            id: 1,
            name: "in00".to_string(),
            portal_type: 2,
            x: 10,
            y: 20,
            to_map: 100000001,
            to_name: "out00".to_string(),
            script: None,
        };
        let cosmic_portal = PortalRecord {
            x: 11,
            ..nx_portal.clone()
        };

        let nx_maps = BTreeMap::from([(
            100000000,
            MapRecord {
                map_id: 100000000,
                portals: BTreeMap::from([(1, nx_portal)]),
            },
        )]);
        let cosmic_maps = BTreeMap::from([(
            100000000,
            MapRecord {
                map_id: 100000000,
                portals: BTreeMap::from([(1, cosmic_portal)]),
            },
        )]);

        let non_strict = compare_maps(
            &nx_maps,
            &cosmic_maps,
            &CompareOptions {
                target_maps: None,
                strict: false,
            },
        );
        assert_eq!(non_strict.summary.soft_count, 1);
        assert!(!non_strict.summary.should_fail);

        let strict = compare_maps(
            &nx_maps,
            &cosmic_maps,
            &CompareOptions {
                target_maps: None,
                strict: true,
            },
        );
        assert!(strict.summary.should_fail);
    }

    #[test]
    fn hard_mismatch_when_tm_differs() {
        let nx_portal = PortalRecord {
            id: 1,
            name: "in00".to_string(),
            portal_type: 2,
            x: 10,
            y: 20,
            to_map: 100000001,
            to_name: "out00".to_string(),
            script: None,
        };
        let cosmic_portal = PortalRecord {
            to_map: 100000002,
            ..nx_portal.clone()
        };

        let nx_maps = BTreeMap::from([(
            100000000,
            MapRecord {
                map_id: 100000000,
                portals: BTreeMap::from([(1, nx_portal)]),
            },
        )]);
        let cosmic_maps = BTreeMap::from([(
            100000000,
            MapRecord {
                map_id: 100000000,
                portals: BTreeMap::from([(1, cosmic_portal)]),
            },
        )]);

        let report = compare_maps(
            &nx_maps,
            &cosmic_maps,
            &CompareOptions {
                target_maps: None,
                strict: false,
            },
        );

        assert_eq!(report.summary.hard_count, 1);
        assert!(report.summary.should_fail);
    }
}
