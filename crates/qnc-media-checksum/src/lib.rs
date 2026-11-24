//! The checksum of a medium as it is copied (user 2026-11-16: "s obzirom da aplikaciju razvijamo
//! na kockama-modulima, postavi osnovno, a ako neko traži nešto bolje ili drugi način, to preko
//! kocki lako dodamo"). A copy of a card is the card's only when its checksum is the source's: the
//! copier feeds the bytes it reads to a [`Checksum`] and those it wrote to another, and compares.
//!
//! The basic kind is xxHash64 (seed 0), the one DIT offload tools and the ASC MHL use: fast enough
//! to keep up with a card reader. Another kind (MD5, SHA-256, xxh128) is another implementation of
//! [`Checksum`] in a block of its own, named in [`kind`]. No I/O.

/// A checksum fed in pieces, in order.
pub trait Checksum: Send {
    /// Its name as the MHL and the database write it (`xxh64`).
    fn name(&self) -> &'static str;
    fn update(&mut self, bytes: &[u8]);
    /// The checksum of everything fed, in lower case hex.
    fn hex(&self) -> String;
}

/// The basic kind.
pub const XXH64: &str = "xxh64";

/// A new checksum of the kind named `name`; `None` for a kind no block gives.
pub fn kind(name: &str) -> Option<Box<dyn Checksum>> {
    match name {
        XXH64 => Some(Box::new(XxHash64::new())),
        _ => None,
    }
}

/// `name:hex`, as a copy's checksum is kept.
pub fn label(checksum: &dyn Checksum) -> String {
    format!("{}:{}", checksum.name(), checksum.hex())
}

const P1: u64 = 0x9E37_79B1_85EB_CA87;
const P2: u64 = 0xC2B2_AE3D_27D4_EB4F;
const P3: u64 = 0x1656_67B1_9E37_79F9;
const P4: u64 = 0x85EB_CA77_C2B2_AE63;
const P5: u64 = 0x27D4_EB2F_1656_67C5;

/// xxHash64 with seed 0, fed in pieces of any size.
#[derive(Debug, Clone)]
pub struct XxHash64 {
    lanes: [u64; 4],
    buffer: [u8; 32],
    buffered: usize,
    total: u64,
}

impl Default for XxHash64 {
    fn default() -> Self {
        Self::new()
    }
}

fn round(acc: u64, input: u64) -> u64 {
    acc.wrapping_add(input.wrapping_mul(P2)).rotate_left(31).wrapping_mul(P1)
}

fn merge(acc: u64, lane: u64) -> u64 {
    (acc ^ round(0, lane)).wrapping_mul(P1).wrapping_add(P4)
}

fn word(bytes: &[u8]) -> u64 {
    u64::from_le_bytes(bytes[..8].try_into().expect("eight bytes"))
}

impl XxHash64 {
    pub fn new() -> Self {
        Self { lanes: [P1.wrapping_add(P2), P2, 0, 0u64.wrapping_sub(P1)], buffer: [0; 32], buffered: 0, total: 0 }
    }

    fn stripe(&mut self, stripe: &[u8]) {
        for (at, lane) in self.lanes.iter_mut().enumerate() {
            *lane = round(*lane, word(&stripe[at * 8..]));
        }
    }

    pub fn digest(&self) -> u64 {
        let [v1, v2, v3, v4] = self.lanes;
        let mut hash = if self.total >= 32 {
            let hash = v1.rotate_left(1).wrapping_add(v2.rotate_left(7)).wrapping_add(v3.rotate_left(12)).wrapping_add(v4.rotate_left(18));
            [v1, v2, v3, v4].into_iter().fold(hash, merge)
        } else {
            P5
        };
        hash = hash.wrapping_add(self.total);
        let mut rest = &self.buffer[..self.buffered];
        while rest.len() >= 8 {
            hash = (hash ^ round(0, word(rest))).rotate_left(27).wrapping_mul(P1).wrapping_add(P4);
            rest = &rest[8..];
        }
        if rest.len() >= 4 {
            let half = u64::from(u32::from_le_bytes(rest[..4].try_into().expect("four bytes")));
            hash = (hash ^ half.wrapping_mul(P1)).rotate_left(23).wrapping_mul(P2).wrapping_add(P3);
            rest = &rest[4..];
        }
        for &byte in rest {
            hash = (hash ^ u64::from(byte).wrapping_mul(P5)).rotate_left(11).wrapping_mul(P1);
        }
        hash ^= hash >> 33;
        hash = hash.wrapping_mul(P2);
        hash ^= hash >> 29;
        hash = hash.wrapping_mul(P3);
        hash ^ (hash >> 32)
    }
}

impl Checksum for XxHash64 {
    fn name(&self) -> &'static str {
        XXH64
    }

    fn update(&mut self, mut bytes: &[u8]) {
        self.total += bytes.len() as u64;
        if self.buffered > 0 {
            let take = (32 - self.buffered).min(bytes.len());
            self.buffer[self.buffered..self.buffered + take].copy_from_slice(&bytes[..take]);
            self.buffered += take;
            bytes = &bytes[take..];
            if self.buffered < 32 {
                return;
            }
            let stripe = self.buffer;
            self.stripe(&stripe);
            self.buffered = 0;
        }
        while bytes.len() >= 32 {
            self.stripe(&bytes[..32]);
            bytes = &bytes[32..];
        }
        self.buffer[..bytes.len()].copy_from_slice(bytes);
        self.buffered = bytes.len();
    }

    fn hex(&self) -> String {
        format!("{:016x}", self.digest())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn of(bytes: &[u8]) -> u64 {
        let mut hash = XxHash64::new();
        hash.update(bytes);
        hash.digest()
    }

    #[test]
    fn xxhash64_gives_the_known_values_whatever_the_pieces() {
        assert_eq!(of(b""), 0xEF46_DB37_51D8_E999);
        assert_eq!(of(b"a"), 0xD24E_C4F1_A98C_6E5B);
        assert_eq!(of(b"abc"), 0x44BC_2CF5_AD77_0999);
        assert_eq!(of(b"Nobody inspects the spammish repetition"), 0xFBCE_A83C_8A37_8BF1);
        let long: Vec<u8> = (0..10_000u32).map(|n| (n * 7 % 251) as u8).collect();
        let mut pieces = XxHash64::new();
        for piece in long.chunks(37) {
            pieces.update(piece);
        }
        assert_eq!(pieces.digest(), of(&long), "fed in pieces as at once");
        assert_eq!(label(kind(XXH64).as_deref().unwrap()), "xxh64:ef46db3751d8e999");
        assert!(kind("crc32").is_none(), "a kind no block gives");
    }
}

/// The MHL (media hash list, version 1.1) of copies, as DIT offload tools write and read it: each
/// copied file with its size, last change and checksum. Text only; the caller writes it.
pub mod mhl {
    /// One copied file of the list.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Entry {
        /// Its path relative to the folder the list is in, with `/`.
        pub file: String,
        pub size: u64,
        /// Its last change, `YYYY-MM-DDThh:mm:ssZ`.
        pub modified: String,
        /// `kind:hex`, as [`crate::label`] gives it.
        pub checksum: String,
        /// When the checksum was made, `YYYY-MM-DDThh:mm:ssZ`.
        pub hashed: String,
    }

    /// The list's text. `tool` and `when` (`YYYY-MM-DDThh:mm:ssZ`) say who made it and when. A
    /// checksum of a kind MHL 1.1 has no element for is left out.
    pub fn document(tool: &str, when: &str, entries: &[Entry]) -> String {
        let mut text = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<hashlist version=\"1.1\">\n");
        text.push_str(&format!("  <creatorinfo>\n    <tool>{}</tool>\n    <startdate>{when}</startdate>\n    <finishdate>{when}</finishdate>\n  </creatorinfo>\n", escape(tool)));
        for entry in entries {
            let Some((kind, hex)) = entry.checksum.split_once(':') else { continue };
            let element = match kind {
                crate::XXH64 => "xxhash64be",
                "md5" => "md5",
                "sha1" => "sha1",
                _ => continue,
            };
            text.push_str(&format!(
                "  <hash>\n    <file>{}</file>\n    <size>{}</size>\n    <lastmodificationdate>{}</lastmodificationdate>\n    <{element}>{}</{element}>\n    <hashdate>{}</hashdate>\n  </hash>\n",
                escape(&entry.file),
                entry.size,
                entry.modified,
                escape(hex),
                entry.hashed
            ));
        }
        text.push_str("</hashlist>\n");
        text
    }

    fn escape(text: &str) -> String {
        text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn a_list_has_each_file_its_size_and_its_xxhash() {
            let entry = Entry { file: "original/XDROOT/Clip/A & B.MXF".into(), size: 42, modified: "2026-11-16T10:00:00Z".into(), checksum: "xxh64:ef46db3751d8e999".into(), hashed: "2026-11-16T10:01:00Z".into() };
            let text = document("QNC-AI", "2026-11-16T10:02:00Z", &[entry]);
            assert!(text.contains("<file>original/XDROOT/Clip/A &amp; B.MXF</file>"));
            assert!(text.contains("<size>42</size>"));
            assert!(text.contains("<xxhash64be>ef46db3751d8e999</xxhash64be>"));
            assert!(text.starts_with("<?xml") && text.trim_end().ends_with("</hashlist>"));
        }
    }
}
