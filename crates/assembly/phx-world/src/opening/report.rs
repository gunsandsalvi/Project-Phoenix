//! The opening's report listed to the run's directory as it is made: each balancing write and each apportionment a
//! line of text, the text compressed a frame at a time, so the world never holds the lists.

use std::fmt::Write as _;
use std::fs::File;
use std::io::Write as _;
use std::path::Path;

use phx_core::{Apportioned, ReportSink, WriteRecord};
use phx_macros::clause;
use phx_store::consts::FRAME_BYTES;

/// One list's file and the text waiting to fill its next frame; the first failure, kept for the report's close.
#[derive(Debug)]
struct Listing {
    file: File,
    text: String,
    failed: Option<String>,
}

impl Listing {
    fn create(path: &Path, header: &str) -> Result<Listing, String> {
        let file = File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(Listing { file, text: format!("{header}\n"), failed: None })
    }

    /// The text written as a frame once it fills one, or at the end.
    fn put(&mut self, whole: bool) {
        if self.failed.is_some() || (!whole && self.text.len() < FRAME_BYTES) {
            return;
        }
        let written =
            phx_store::save::compress_frame(self.text.as_bytes()).and_then(|frame| self.file.write_all(&frame));
        if let Err(e) = written {
            self.failed = Some(e.to_string());
        }
        self.text.clear();
    }

    fn finish(mut self) -> Result<(), String> {
        self.put(true);
        match self.failed {
            Some(e) => Err(format!("the opening's report: {e}")),
            None => self.file.sync_all().map_err(|e| format!("the opening's report: {e}")),
        }
    }
}

/// The opening's writes and apportionments, each listed in a compressed file of the run's directory.
#[clause("GEN.4")]
#[derive(Debug)]
pub struct Listed {
    writes: Listing,
    apportioned: Listing,
}

impl Listed {
    /// The listings begun in `dir`.
    ///
    /// # Errors
    /// A file that cannot be made.
    pub fn create(dir: &Path) -> Result<Listed, String> {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        Ok(Listed {
            writes: Listing::create(&dir.join("opening-writes.csv.zst"), "party,amount,identity,counter")?,
            apportioned: Listing::create(&dir.join("opening-apportioned.csv.zst"), "stratum,party,drawn,realised")?,
        })
    }
}

impl ReportSink for Listed {
    fn write(&mut self, w: &WriteRecord) {
        let text = &mut self.writes.text;
        let _ = writeln!(text, "{},{},{},{}", w.party.get(), w.amount, w.identity, w.counter.get());
        self.writes.put(false);
    }

    fn apportioned(&mut self, a: &Apportioned) {
        let text = &mut self.apportioned.text;
        let _ = writeln!(text, "{},{},{},{}", a.stratum, a.party.get(), a.drawn, a.realised);
        self.apportioned.put(false);
    }

    fn finish(self: Box<Self>) -> Result<(), String> {
        let Listed { writes, apportioned } = *self;
        writes.finish()?;
        apportioned.finish()
    }
}
