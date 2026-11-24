//! The keyboard on a text of timed words, in any form that shows one (the transcript, the
//! script; user 2026-11-15: "u tekst panelu nemamo kursora, pa ne možemo selektirati tekst
//! tipkovnicom"; "kocka treba biti univerzalna"). The text is its rows of words, each word by
//! its frames `[from, to)`. The cursor stands in a gap between words, as in a text editor (user
//! 2026-11-15: "kursor bi unutar rečenice trebao skakati s razmaka na razmak"): before a word,
//! or after the last word of a row. The arrows move it a gap left or right, a row up or down;
//! Shift with the arrows selects from the gap the selection started in to the cursor. What a
//! selection marks (the source IN/OUT) is the caller's.

/// Where an arrow moves the cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextStep {
    Left,
    Right,
    Up,
    Down,
}

/// A word by its frames `[from, to)`.
pub type WordSpan = (u64, u64);

/// A gap of the text: before the word that starts at `frame`, or (`end`) after the last word
/// of a row, which ends at `frame`. The playhead of a gap is its frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gap {
    pub frame: u64,
    pub end: bool,
}

/// The rows a text is shown in, as its panel laid them out on screen (a line of the text
/// wraps into several; user 2026-11-15: the cursor skipped rows), and the cursor's gap the
/// panel draws. The panel that paints the text writes the rows, the keys write the cursor; a
/// handle shared by the clones of a view, not part of what the view says (two views are equal
/// whatever it holds).
#[derive(Debug, Clone, Default)]
pub struct ShownRows(std::sync::Arc<std::sync::Mutex<(Vec<Vec<WordSpan>>, Option<Gap>)>>);

impl ShownRows {
    /// The rows painted now, top to bottom.
    pub fn set(&self, rows: Vec<Vec<WordSpan>>) {
        if let Ok(mut shown) = self.0.lock() {
            shown.0 = rows;
        }
    }

    /// The rows last painted (empty before the first paint).
    pub fn get(&self) -> Vec<Vec<WordSpan>> {
        self.0.lock().map(|shown| shown.0.clone()).unwrap_or_default()
    }

    /// The cursor the panel draws (`None`: the keyboard is not on the text).
    pub fn set_caret(&self, caret: Option<Gap>) {
        if let Ok(mut shown) = self.0.lock() {
            shown.1 = caret;
        }
    }

    pub fn caret(&self) -> Option<Gap> {
        self.0.lock().ok().and_then(|shown| shown.1)
    }
}

impl PartialEq for ShownRows {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

/// The keyboard on a text: the gap of the cursor until the playhead goes elsewhere (a player
/// takes a moment to reach a cued gap, quick presses go on from the cursor), and the gap a
/// selection started in, while Shift moves on from it.
#[derive(Debug, Default, Clone)]
pub struct TextCursor {
    at: Option<(Gap, Option<u64>)>,
    anchor: Option<Gap>,
}

impl TextCursor {
    /// The cursor in `gap` (a click on a word puts it before the word); `playhead` where it is
    /// now.
    pub fn place(&mut self, gap: Gap, playhead: Option<u64>) {
        self.at = Some((gap, playhead));
    }

    /// The gap of the cursor: the one placed while the playhead has not gone elsewhere, else
    /// the gap at the playhead.
    pub fn gap(&self, rows: &[Vec<WordSpan>], playhead: Option<u64>) -> Option<Gap> {
        match self.at {
            Some((gap, then)) if then == playhead || playhead == Some(gap.frame) => Some(gap),
            _ => cursor_gap(rows, playhead),
        }
    }

    /// The cursor moved without Shift: no selection goes on.
    pub fn moved(&mut self) {
        self.anchor = None;
    }

    /// One arrow on `rows`: the gap the cursor goes to, and with `select` (Shift) the frames
    /// `[from, to)` selected from where the selection started to it (`None` while empty).
    /// `None` at the edge of the text or without words.
    pub fn step(&mut self, rows: &[Vec<WordSpan>], playhead: Option<u64>, step: TextStep, select: bool) -> Option<(Gap, Option<(u64, u64)>)> {
        let cursor = self.gap(rows, playhead)?;
        let to = gap_beside(rows, cursor, step)?;
        let selection = if select {
            let anchor = *self.anchor.get_or_insert(cursor);
            let (from, till) = (anchor.frame.min(to.frame), anchor.frame.max(to.frame));
            (from < till).then_some((from, till))
        } else {
            self.moved();
            None
        };
        self.place(to, playhead);
        Some((to, selection))
    }
}

/// The gaps of each row: before each word, and after its last.
fn gaps(rows: &[Vec<WordSpan>]) -> Vec<Vec<Gap>> {
    rows.iter()
        .filter(|row| !row.is_empty())
        .map(|row| row.iter().map(|word| Gap { frame: word.0, end: false }).chain(row.last().map(|word| Gap { frame: word.1, end: true })).collect())
        .collect()
}

/// The gap of the cursor at `playhead`: before the word said there, else the end of a row that
/// ends there, else before the last word that started before it, else the first gap.
pub fn cursor_gap(rows: &[Vec<WordSpan>], playhead: Option<u64>) -> Option<Gap> {
    let at = playhead.unwrap_or(0);
    let words = || rows.iter().flatten().copied();
    if let Some(word) = words().find(|word| (word.0..word.1).contains(&at)) {
        return Some(Gap { frame: word.0, end: false });
    }
    if rows.iter().filter_map(|row| row.last()).any(|word| word.1 == at) {
        return Some(Gap { frame: at, end: true });
    }
    words().filter(|word| word.0 <= at).last().or_else(|| words().next()).map(|word| Gap { frame: word.0, end: false })
}

/// The gap an arrow moves the cursor to from `cursor`: the next or previous gap (past a row's
/// end on to the next row), or the gap at the same place in the row above or below; `None` at
/// the edge of the text.
pub fn gap_beside(rows: &[Vec<WordSpan>], cursor: Gap, step: TextStep) -> Option<Gap> {
    let rows = gaps(rows);
    let (row_at, gap_at) = rows.iter().enumerate().find_map(|(r, row)| row.iter().position(|gap| *gap == cursor).map(|g| (r, g)))?;
    let gap = |r: usize, g: usize| rows.get(r).map(|row| row[g.min(row.len() - 1)]);
    match step {
        TextStep::Right if gap_at + 1 < rows[row_at].len() => gap(row_at, gap_at + 1),
        TextStep::Right => gap(row_at + 1, 0),
        TextStep::Left if gap_at > 0 => gap(row_at, gap_at - 1),
        TextStep::Left => row_at.checked_sub(1).and_then(|r| gap(r, usize::MAX)),
        TextStep::Down => gap(row_at + 1, gap_at),
        TextStep::Up => row_at.checked_sub(1).and_then(|r| gap(r, gap_at)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn before(frame: u64) -> Gap {
        Gap { frame, end: false }
    }

    #[test]
    fn the_cursor_goes_from_gap_to_gap_and_shift_selects_the_words_between() {
        let rows = vec![vec![(0, 10), (10, 20), (20, 30)], vec![(30, 40), (40, 50)]];
        let end = |frame| Gap { frame, end: true };
        assert_eq!(cursor_gap(&rows, Some(15)), Some(before(10)));
        assert_eq!(cursor_gap(&rows, Some(50)), Some(end(50)), "at the end of a row");
        assert_eq!(gap_beside(&rows, before(20), TextStep::Right), Some(end(30)), "after the row's last word");
        assert_eq!(gap_beside(&rows, end(30), TextStep::Right), Some(before(30)), "on to the next row, though its frame is the same");
        assert_eq!(gap_beside(&rows, before(30), TextStep::Left), Some(end(30)));
        assert_eq!(gap_beside(&rows, before(40), TextStep::Up), Some(before(10)));
        assert_eq!(gap_beside(&rows, end(30), TextStep::Down), Some(end(50)), "the same place, or the row's last gap");
        assert_eq!(gap_beside(&rows, before(0), TextStep::Left), None);
        let mut cursor = TextCursor::default();
        assert_eq!(cursor.step(&rows, Some(10), TextStep::Right, true), Some((before(20), Some((10, 20)))), "one word selected");
        assert_eq!(cursor.step(&rows, Some(10), TextStep::Right, true), Some((end(30), Some((10, 30)))), "quick presses go on from the cursor");
        assert_eq!(cursor.step(&rows, Some(10), TextStep::Left, false), Some((before(20), None)), "an arrow without Shift ends the selection");
    }
}
