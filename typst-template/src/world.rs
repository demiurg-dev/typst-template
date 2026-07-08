//! The Typst [`World`] split into a reusable base and per-generation concrete
//! worlds.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{fmt, fs};

use typst::diag::{FileError, FileResult, SourceResult, Warned};
use typst::foundations::{Bytes, Datetime, Dict, Duration, Str};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};
use typst_kit::fonts::{self, FontStore};
use typst_layout::PagedDocument;
use typst_pdf::{PdfOptions, pdf};

use crate::value::{ToDict, ToValue};

/// A long-lived, cheaply cloneable Typst world.
///
/// Holds the project root and the loaded font set. Build a `WorldBase` once and
/// derive a [`ConcreteWorld`] from it for each document via
/// [`concrete`](Self::concrete). Cloning shares the fonts through an [`Arc`].
#[derive(Clone)]
#[must_use]
pub struct WorldBase {
    inner: Arc<WorldBaseInner>,
}

struct WorldBaseInner {
    root: PathBuf,
    fonts: FontStore,
}

impl fmt::Debug for WorldBase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WorldBase")
            .field("root", &self.inner.root)
            .finish_non_exhaustive()
    }
}

/// Builder for a [`WorldBase`].
///
/// By default both system and embedded fonts are searched; add extra font
/// directories with [`font_path`](Self::font_path).
#[derive(Debug, Clone)]
#[must_use]
pub struct WorldBaseConfig {
    root: PathBuf,
    font_paths: Vec<PathBuf>,
    system_fonts: bool,
    embedded_fonts: bool,
}

impl WorldBaseConfig {
    /// Starts a config rooted at `root`. Real files are resolved relative to it.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into(), font_paths: Vec::new(), system_fonts: true, embedded_fonts: true }
    }

    /// Adds a directory to search for fonts (searched recursively).
    pub fn font_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.font_paths.push(path.into());
        self
    }

    /// Adds several font directories.
    pub fn font_paths<P: Into<PathBuf>>(mut self, paths: impl IntoIterator<Item = P>) -> Self {
        self.font_paths.extend(paths.into_iter().map(Into::into));
        self
    }

    /// Whether to include fonts installed on the system (default `true`).
    pub fn system_fonts(mut self, include: bool) -> Self {
        self.system_fonts = include;
        self
    }

    /// Whether to include the fonts embedded in `typst-assets` (default `true`).
    pub fn embedded_fonts(mut self, include: bool) -> Self {
        self.embedded_fonts = include;
        self
    }

    /// Builds the base, searching for fonts now.
    pub fn build(self) -> WorldBase {
        // Priority follows load order: font paths, then system, then embedded.
        let mut store = FontStore::new();
        for path in &self.font_paths {
            store.extend(fonts::scan(path));
        }
        if self.system_fonts {
            store.extend(fonts::system());
        }
        if self.embedded_fonts {
            store.extend(fonts::embedded());
        }
        let inner = WorldBaseInner { root: self.root, fonts: store };
        WorldBase { inner: Arc::new(inner) }
    }
}

impl WorldBase {
    /// Builds a base rooted at `root` with the default font configuration.
    ///
    /// Use [`WorldBaseConfig`] for finer control.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        WorldBaseConfig::new(root).build()
    }

    /// Starts building a [`ConcreteWorld`] whose main file is `main` (a path
    /// relative to the root, or a virtual path added with
    /// [`file`](ConcreteWorldBuilder::file)).
    pub fn concrete(&self, main: impl AsRef<Path>) -> ConcreteWorldBuilder {
        ConcreteWorldBuilder {
            base: self.clone(),
            main: file_id(main),
            inputs: Dict::new(),
            files: HashMap::new(),
            today: Today::default(),
        }
    }
}

impl WorldBaseInner {
    fn load_file(&self, id: FileId) -> FileResult<Vec<u8>> {
        let path = id
            .vpath()
            .realize(&self.root)
            .map_err(|_| FileError::AccessDenied)?;
        // TODO: Cache reads in the base and re-validate against mtime on the
        // next request. See TODO.md ("Disk file read cache").
        fs::read(&path).map_err(|err| match err.kind() {
            std::io::ErrorKind::NotFound => FileError::NotFound(path),
            std::io::ErrorKind::PermissionDenied => FileError::AccessDenied,
            std::io::ErrorKind::IsADirectory => FileError::IsDirectory,
            _ => FileError::Other(Some(format!("{err}").into())),
        })
    }
}

/// Builder for a [`ConcreteWorld`].
#[derive(Clone)]
#[must_use]
pub struct ConcreteWorldBuilder {
    base: WorldBase,
    main: FileId,
    inputs: Dict,
    files: HashMap<FileId, Bytes>,
    today: Today,
}

impl fmt::Debug for ConcreteWorldBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConcreteWorldBuilder")
            .field("main", &self.main)
            .field("inputs", &self.inputs)
            .finish_non_exhaustive()
    }
}

impl ConcreteWorldBuilder {
    /// Sets the whole `sys.inputs` dictionary, replacing any earlier inputs.
    pub fn inputs(mut self, inputs: impl ToDict) -> Self {
        self.inputs = inputs.into_dict();
        self
    }

    /// Adds a single `sys.inputs` entry.
    pub fn input(mut self, key: impl Into<Str>, value: impl ToValue) -> Self {
        self.inputs.insert(key.into(), value.into_value());
        self
    }

    /// Adds a virtual in-memory file at `path`, visible to the document instead
    /// of any real file at that path.
    pub fn file(mut self, path: impl AsRef<Path>, content: impl Into<Vec<u8>>) -> Self {
        self.files.insert(file_id(path), Bytes::new(content.into()));
        self
    }

    /// Pins `datetime.today()` to a fixed value.
    ///
    /// Answers only offset-less (`auto`) requests; `datetime.today(offset: N)`
    /// returns nothing.
    pub fn today(mut self, today: Datetime) -> Self {
        self.today = Today::Fixed(today);
        self
    }

    /// Makes `datetime.today()` read the system clock, treating "local" as the
    /// operating system's time zone.
    ///
    /// With the `time` backend, "local" falls back to UTC when the OS offset is
    /// unavailable (e.g. in a multithreaded process); use
    /// [`today_system_in`](Self::today_system_in) for a specific zone.
    #[cfg(any(feature = "chrono", feature = "time"))]
    #[cfg_attr(docsrs, doc(cfg(any(feature = "chrono", feature = "time"))))]
    pub fn today_system(mut self) -> Self {
        self.today = Today::SystemLocal;
        self
    }

    /// Makes `datetime.today()` read the system clock, treating "local" as the
    /// given named time zone (DST-aware).
    ///
    /// Accepts a [`chrono_tz::Tz`](crate::chrono_tz::Tz) (feature `chrono-tz`)
    /// or a `&'static` [`time_tz::Tz`](crate::time_tz::Tz) (feature `time-tz`)
    /// via [`Into<Zone>`](Zone).
    #[cfg(any(feature = "chrono-tz", feature = "time-tz"))]
    #[cfg_attr(docsrs, doc(cfg(any(feature = "chrono-tz", feature = "time-tz"))))]
    pub fn today_system_in(mut self, zone: impl Into<Zone>) -> Self {
        self.today = Today::SystemZone(zone.into());
        self
    }

    /// Makes `datetime.today()` return nothing.
    pub fn today_disabled(mut self) -> Self {
        self.today = Today::Disabled;
        self
    }

    /// Builds the concrete world.
    pub fn build(self) -> ConcreteWorld {
        let library = Library::builder().with_inputs(self.inputs).build();
        ConcreteWorld {
            base: self.base.inner,
            library: LazyHash::new(library),
            files: self.files,
            main: self.main,
            today: self.today,
        }
    }
}

/// A single-document world: a [`WorldBase`] plus this generation's inputs and
/// virtual files.
///
/// Build one via [`WorldBase::concrete`], then call
/// [`render_pdf`](Self::render_pdf).
#[must_use]
pub struct ConcreteWorld {
    base: Arc<WorldBaseInner>,
    library: LazyHash<Library>,
    files: HashMap<FileId, Bytes>,
    main: FileId,
    today: Today,
}

impl fmt::Debug for ConcreteWorld {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConcreteWorld")
            .field("main", &self.main)
            .finish_non_exhaustive()
    }
}

impl ConcreteWorld {
    /// Adds (or replaces) a virtual in-memory file after construction.
    pub fn add_file(&mut self, path: impl AsRef<Path>, content: impl Into<Vec<u8>>) {
        self.files.insert(file_id(path), Bytes::new(content.into()));
    }

    /// Compiles the document, returning the paged document (or errors) together
    /// with any warnings.
    pub fn compile(&self) -> Warned<SourceResult<PagedDocument>> {
        typst::compile(self)
    }

    /// Compiles and exports to PDF, returning the bytes (or errors) together
    /// with any compile warnings.
    pub fn render_pdf(&self, options: &PdfOptions) -> Warned<SourceResult<Vec<u8>>> {
        let Warned { output, warnings } = self.compile();
        let output = output.and_then(|document| pdf(&document, options));
        Warned { output, warnings }
    }

    /// Like [`render_pdf`](Self::render_pdf) with [`PdfOptions::default`].
    pub fn render_pdf_default(&self) -> Warned<SourceResult<Vec<u8>>> {
        self.render_pdf(&PdfOptions::default())
    }

    fn read(&self, id: FileId) -> FileResult<Bytes> {
        match self.files.get(&id) {
            Some(bytes) => Ok(bytes.clone()),
            None => Ok(Bytes::new(self.base.load_file(id)?)),
        }
    }
}

impl World for ConcreteWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        self.base.fonts.book()
    }

    fn main(&self) -> FileId {
        self.main
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        let bytes = self.read(id)?;
        let text = std::str::from_utf8(&bytes).map_err(|_| FileError::InvalidUtf8)?;
        Ok(Source::new(id, text.to_owned()))
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.read(id)
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.base.fonts.font(index)
    }

    fn today(&self, offset: Option<Duration>) -> Option<Datetime> {
        // typst treats an integer `datetime.today` offset as a duration in hours.
        self.today.resolve(offset.map(|d| d.hours() as i64))
    }
}

/// How the world answers `datetime.today()`.
#[derive(Clone)]
enum Today {
    /// `datetime.today()` returns nothing.
    Disabled,
    /// A fixed value; answers only offset-less requests.
    Fixed(Datetime),
    /// The system clock, with "local" meaning the operating system's time zone.
    #[cfg(any(feature = "chrono", feature = "time"))]
    SystemLocal,
    /// The system clock, with "local" meaning a named time zone.
    #[cfg(any(feature = "chrono-tz", feature = "time-tz"))]
    SystemZone(Zone),
}

/// Defaults to the system clock when a `chrono`/`time` feature is enabled,
/// otherwise leaves `datetime.today()` disabled.
impl Default for Today {
    fn default() -> Self {
        #[cfg(any(feature = "chrono", feature = "time"))]
        {
            Today::SystemLocal
        }
        #[cfg(not(any(feature = "chrono", feature = "time")))]
        {
            Today::Disabled
        }
    }
}

impl Today {
    fn resolve(&self, offset: Option<i64>) -> Option<Datetime> {
        match self {
            Today::Disabled => None,
            // A fixed value answers only offset-less requests.
            Today::Fixed(today) => match offset {
                None => Some(*today),
                Some(_) => None,
            },
            #[cfg(any(feature = "chrono", feature = "time"))]
            Today::SystemLocal => match offset {
                Some(hours) => offset_now(hours),
                None => local_now(),
            },
            #[cfg(any(feature = "chrono-tz", feature = "time-tz"))]
            // An explicit offset overrides the configured zone.
            Today::SystemZone(zone) => match offset {
                Some(hours) => offset_now(hours),
                None => zone.now(),
            },
        }
    }
}

/// A named time zone for [`today_system_in`](ConcreteWorldBuilder::today_system_in).
///
/// Build one with `.into()` from a [`chrono_tz::Tz`](crate::chrono_tz::Tz)
/// (feature `chrono-tz`) or a `&'static` [`time_tz::Tz`](crate::time_tz::Tz)
/// (feature `time-tz`). Both features may be enabled at once.
#[cfg(any(feature = "chrono-tz", feature = "time-tz"))]
#[derive(Clone)]
pub struct Zone(ZoneRepr);

#[cfg(any(feature = "chrono-tz", feature = "time-tz"))]
impl fmt::Debug for Zone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Zone").finish_non_exhaustive()
    }
}

#[cfg(any(feature = "chrono-tz", feature = "time-tz"))]
#[derive(Clone)]
enum ZoneRepr {
    #[cfg(feature = "chrono-tz")]
    Chrono(chrono_tz::Tz),
    #[cfg(feature = "time-tz")]
    Time(&'static time_tz::Tz),
}

#[cfg(feature = "chrono-tz")]
impl From<chrono_tz::Tz> for Zone {
    fn from(tz: chrono_tz::Tz) -> Self {
        Zone(ZoneRepr::Chrono(tz))
    }
}

#[cfg(feature = "time-tz")]
impl From<&'static time_tz::Tz> for Zone {
    fn from(tz: &'static time_tz::Tz) -> Self {
        Zone(ZoneRepr::Time(tz))
    }
}

#[cfg(any(feature = "chrono-tz", feature = "time-tz"))]
impl Zone {
    fn now(&self) -> Option<Datetime> {
        match &self.0 {
            #[cfg(feature = "chrono-tz")]
            ZoneRepr::Chrono(tz) => {
                use chrono::{Datelike, Timelike, Utc};
                let now = Utc::now().with_timezone(tz);
                crate::convert::datetime(
                    now.year(),
                    now.month() as u8,
                    now.day() as u8,
                    now.hour() as u8,
                    now.minute() as u8,
                    now.second() as u8,
                )
            }
            #[cfg(feature = "time-tz")]
            ZoneRepr::Time(tz) => {
                use time::OffsetDateTime;
                use time_tz::OffsetDateTimeExt;
                let now = OffsetDateTime::now_utc().to_timezone(*tz);
                crate::convert::datetime(
                    now.year(),
                    u8::from(now.month()),
                    now.day(),
                    now.hour(),
                    now.minute(),
                    now.second(),
                )
            }
        }
    }
}

/// The current time in the OS-local time zone, via `chrono`.
#[cfg(feature = "chrono")]
fn local_now() -> Option<Datetime> {
    use chrono::{Datelike, Local, Timelike};
    let now = Local::now().naive_local();
    crate::convert::datetime(
        now.year(),
        now.month() as u8,
        now.day() as u8,
        now.hour() as u8,
        now.minute() as u8,
        now.second() as u8,
    )
}

/// The current time via `time`. Falls back to UTC when the OS-local offset is
/// unavailable (e.g. in a multithreaded process); use
/// [`today_system_in`](ConcreteWorldBuilder::today_system_in) for a specific
/// zone.
#[cfg(all(feature = "time", not(feature = "chrono")))]
fn local_now() -> Option<Datetime> {
    use time::OffsetDateTime;
    let now = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
    crate::convert::datetime(now.year(), u8::from(now.month()), now.day(), now.hour(), now.minute(), now.second())
}

/// The current time at a fixed UTC offset (in hours). Uses `chrono` if enabled,
/// otherwise `time`. Returns `None` if the offset is too large to apply.
#[cfg(feature = "chrono")]
fn offset_now(hours: i64) -> Option<Datetime> {
    use chrono::{Datelike, Duration, Timelike, Utc};
    let now = Utc::now()
        .checked_add_signed(Duration::try_hours(hours)?)?
        .naive_utc();
    crate::convert::datetime(
        now.year(),
        now.month() as u8,
        now.day() as u8,
        now.hour() as u8,
        now.minute() as u8,
        now.second() as u8,
    )
}

#[cfg(all(feature = "time", not(feature = "chrono")))]
fn offset_now(hours: i64) -> Option<Datetime> {
    use time::OffsetDateTime;
    let seconds = hours.checked_mul(3600)?;
    let now = OffsetDateTime::now_utc().checked_add(time::Duration::seconds(seconds))?;
    crate::convert::datetime(now.year(), u8::from(now.month()), now.day(), now.hour(), now.minute(), now.second())
}

fn file_id(path: impl AsRef<Path>) -> FileId {
    let vpath = VirtualPath::new(path.as_ref().to_string_lossy()).expect("valid virtual path");
    FileId::new(RootedPath::new(VirtualRoot::Project, vpath))
}
