//! The functions of an ingest the user stopped (user 2026-11-12: "na info prozoru za svaku radnju
//! dodaj ikonu X što obustavlja taj proces, ako korisniku nije potreban"). The window writes them
//! into the base's own database (one runtime record of the project, the only link between the
//! window and the worker); the worker reads them and leaves those functions alone, the ones
//! running stop. Stopped is stopped for that base: the next card into the same base does not do
//! them either.

use std::collections::BTreeSet;

use qnc_content_store::ContentTarget;

/// The runtime record of the base: the codes stopped, a comma between them.
pub const KEY: &str = "stopped_work";

pub const IMPORT: &str = "import";
pub const POSTERS: &str = "posters";
pub const FILMSTRIPS: &str = "filmstrips";
pub const WAVES: &str = "waves";
pub const SHOTS: &str = "shots";
pub const CONTENT: &str = "content";
pub const CAPTIONS: &str = "captions";

/// Every function that can be stopped.
pub const ALL: [&str; 7] = [IMPORT, POSTERS, FILMSTRIPS, WAVES, SHOTS, CONTENT, CAPTIONS];

/// The codes of a record (unknown ones left out).
pub fn parse(value: &str) -> BTreeSet<String> {
    value.split(',').map(str::trim).filter(|code| ALL.contains(code)).map(String::from).collect()
}

/// The functions stopped in the base of `target` (none when nothing is written).
pub fn read(target: &ContentTarget) -> BTreeSet<String> {
    qnc_playback_activity::read(target, KEY).map(|value| parse(&value)).unwrap_or_default()
}

/// Stops `code` in the base of `target` (the others stopped stay stopped).
pub fn stop(target: ContentTarget, code: &str) -> Result<(), String> {
    if !ALL.contains(&code) {
        return Err(format!("no function {code} to stop"));
    }
    let mut stopped = read(&target);
    stopped.insert(code.into());
    let value = stopped.into_iter().collect::<Vec<_>>().join(",");
    qnc_playback_activity::Writer::start(target)?.set(KEY, &value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_record_gives_the_known_codes_only() {
        let stopped = parse("waves, captions,nothing,,import");
        assert_eq!(stopped.into_iter().collect::<Vec<_>>(), vec!["captions", "import", "waves"]);
    }
}
