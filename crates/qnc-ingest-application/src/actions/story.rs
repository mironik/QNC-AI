//! QNC-AI (user 2026-11-12: "kad na ingestu, ako ima selektiranih kadrova, korisnik klikne UVEZI,
//! ingest se zatvara, a Novi Projekt se otvara"): Uvezi writes the selection and wakes the import
//! at once; once the worker has it, the host closes Ingest and opens the new project's own window
//! (`qnc-ai-new-project`). Ingest draws no project sheet.

use crate::*;

impl IngestApplication {
    /// Uvezi asked: the selection goes to the import as it always did.
    pub(crate) fn import_asked(&mut self) -> IngestDispatchResult {
        self.start_import()
    }
}
