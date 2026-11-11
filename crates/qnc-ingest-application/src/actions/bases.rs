//! QNC-AI: the Ingest bases of the application in the browser (user 2026-11-11: "kako smo
//! odlucili da imamo globalni direktorij za spremanje ingest baza trebamo dodati i to kao direktni
//! izbornik"). The list and the open are the block `qnc-ingest-base`'s, off this thread; the
//! browser shows the bases as its entries, and Select (or a click) makes one the base Ingest works
//! in, after which Ingest loads it as it loads a project.

use crate::*;
use qnc_ingest_base::{BaseNews, IngestBase};

impl IngestApplication {
    /// The bases listed (the tab, Disks, Up).
    pub(crate) fn browse_bases(&mut self) -> IngestDispatchResult {
        let Some(root) = self.root.clone() else {
            return IngestDispatchResult::rejected(qnc_ui_text::text("ingest.msg.source_not_connected"));
        };
        self.view.source_kind = SourceKind::Bases;
        self.clear_source_browser_state();
        self.view.browser.busy = true;
        self.bases.list(&root);
        IngestDispatchResult::accepted(None, true)
    }

    /// The base of `uri` made the one Ingest works in.
    pub(crate) fn open_base(&mut self, uri: &str) -> IngestDispatchResult {
        let (Some(root), Some(id)) = (self.root.clone(), self.base_ids.get(uri).cloned()) else {
            return IngestDispatchResult::rejected(qnc_ui_text::text("ingest.msg.pick_disk_or_folder"));
        };
        self.view.browser.busy = true;
        self.bases.open(&root, &id);
        IngestDispatchResult::accepted(None, true)
    }

    /// A list or an open finished.
    pub(crate) fn poll_bases(&mut self) -> bool {
        let Some(news) = self.bases.poll() else { return false };
        self.view.browser.busy = false;
        match news {
            BaseNews::Listed(Ok(bases)) => {
                self.base_ids = bases.iter().map(|base| (base.uri.clone(), base.id.clone())).collect();
                self.view.browser.roots = true;
                self.view.browser.error = None;
                self.view.browser.entries = bases.iter().map(entry).collect();
            }
            BaseNews::Listed(Err(error)) | BaseNews::Opened(Err(error)) => {
                self.view.browser.error = Some(error.clone());
                self.view.message = error;
            }
            BaseNews::Opened(Ok(base)) => {
                self.view.message = base.name.clone();
                self.load_work_settings(None);
                return self.browse_bases().request_repaint;
            }
        }
        true
    }
}

/// A base as an entry of the browser: its name, the project that took it, and whether Ingest works
/// in it now.
fn entry(base: &IngestBase) -> qnc_source_browse::BrowseLocationEntry {
    let project = base.project_name.clone().unwrap_or_else(|| qnc_ui_text::text("source.base_no_project").to_string());
    qnc_source_browse::BrowseLocationEntry {
        name: if base.active { format!("● {}", base.name) } else { base.name.clone() },
        qnc_uri: base.uri.clone(),
        serial_number: String::new(),
        volume_name: project,
    }
}
