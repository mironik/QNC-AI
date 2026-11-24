//! A base read as a card by the camera catalog and Select's scanner (read only):
//! `scan_base <camera catalog .sqlite> <base folder>`.
use qnc_camera_detector::SourceScope;
use qnc_scanner::{scan_roles, ScanLimits};
use qnc_source_reader::SourceReader;
use qnc_transport_resolver::ResolverConfig;
use std::path::{Path, PathBuf};

const CATALOG: &str = "qnc://local/catalog/camera-patterns";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let resolver = ResolverConfig::new(PathBuf::new()).with_local_binding(CATALOG, Path::new(&args[0]));
    let catalog = qnc_camera_patterns::read_uri(&resolver, CATALOG, None)?;
    let source = SourceReader::local("qnc://local/source/verification-base", Path::new(&args[1]))?;
    let card = qnc_camera_qnc_base::QncBaseCard::new();
    let report = scan_roles(&catalog, &source, SourceScope::CardRelative, &[&card], ScanLimits::default())?;
    let groups = &report.grouping.groups;
    println!(
        "dataset={} indexes_read={} groups={} blocked={} resolved={} issues={:?}",
        catalog.dataset_version,
        report.indexes_read,
        groups.len(),
        report.grouping.blocked.len(),
        report.relationships_resolved(),
        report.issues.iter().take(3).collect::<Vec<_>>()
    );
    let proposals: Vec<_> = groups.iter().map(|group| group.proposal.clone()).collect();
    let (declared, failed) = records(&card, &source, &proposals);
    println!("records declared={declared} failed={failed} of {}", proposals.len());
    if let Some(group) = groups.first() {
        println!("first: {} -> {}", group.proposal.recording_identity, group.proposal.original.relative_path());
    }
    Ok(())
}

/// Every clip's record read through the adapter, as Select reads it.
#[allow(dead_code)]
fn records(card: &qnc_camera_qnc_base::QncBaseCard, source: &SourceReader, groups: &[qnc_source_groups::GroupProposal]) -> (usize, usize) {
    use qnc_camera_adapter::{CameraAdapter, MetadataSufficiency};
    let (mut declared, mut failed) = (0, 0);
    for group in groups {
        let documents: Vec<_> = card
            .documents(group)
            .into_iter()
            .filter_map(|reference| source.read_text(&reference, qnc_source_reader::MAX_TEXT_BYTES).ok().map(|document| qnc_source_groups::IndexDocument { reference, text: document.text }))
            .collect();
        match card.metadata("verification", group, &documents) {
            Ok(metadata) if card.sufficiency(&metadata) == MetadataSufficiency::Declared => declared += 1,
            Ok(_) => {}
            Err(error) => {
                if failed == 0 {
                    println!("record failed: {error}");
                }
                failed += 1;
            }
        }
    }
    (declared, failed)
}
