//! Filmstrip thumbnails handed over inside one process (user 2026-11-18: "dok tražimo AI kadrove u
//! istom prolazu kreiramo i filmstrip"). The shot analysis decodes every intra picture of a clip;
//! the 13 nearest the filmstrip's places are handed here as finished thumbnails, and the filmstrip
//! worker of the same process writes them instead of decoding them again. Only a helper inside one
//! process: the filmstrip is what the database and the files say; a thumbnail never taken is lost
//! harmlessly (the worker then decodes as before).

use std::collections::HashMap;
use std::sync::Mutex;

/// The size of a filmstrip thumbnail, the same as the pictures the shot analysis looks at (user
/// 2026-11-18: "može ista veličina sličica da ne moraš pretvarati"): a decoded picture is the
/// thumbnail as it is (one place for every block that makes them).
pub const THUMB_SIZE: [u32; 2] = [160, 90];

static HANDED: Mutex<Option<HashMap<String, Vec<Vec<u8>>>>> = Mutex::new(None);

/// The clip `clip_id`'s thumbnails as JPEG bytes, one for each filmstrip place in order.
pub fn hand(clip_id: &str, jpegs: Vec<Vec<u8>>) {
    let mut handed = HANDED.lock().unwrap_or_else(|e| e.into_inner());
    handed.get_or_insert_with(HashMap::new).insert(clip_id.to_string(), jpegs);
}

/// The thumbnails handed for the clip `clip_id`, taken (once).
pub fn take(clip_id: &str) -> Option<Vec<Vec<u8>>> {
    HANDED.lock().unwrap_or_else(|e| e.into_inner()).as_mut()?.remove(clip_id)
}

#[cfg(test)]
mod tests {
    #[test]
    fn handed_thumbnails_are_taken_once() {
        super::hand("c1", vec![vec![1], vec![2]]);
        assert_eq!(super::take("c1"), Some(vec![vec![1], vec![2]]));
        assert_eq!(super::take("c1"), None);
    }
}
