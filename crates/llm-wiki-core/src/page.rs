//! Reading a page's title and metadata fields from its text.
//!
//! One pass finds everything a page says about itself, each field with its
//! line and the form it was written in, and two views serve the two readers:
//! [`Page::wiki_view`] gives llm-wiki's search the title and fields it has
//! always read, and [`Page::bullet_block`] gives poman the `- Key: Value`
//! block under the title and every field found anywhere else.
//!
//! The reader reads text, never a file, so it reads any project's pages alike.

use std::collections::BTreeMap;

#[cfg(test)]
mod tests;

/// Where on a page a field was found.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Block {
    /// Between a `---` line at the very top of the page and the next `---`
    /// line, every line of it read.
    FrontMatter,
    /// In the lines just before the title, blank lines between allowed.
    BeforeTitle,
    /// In the lines just after the title, blank lines between allowed.
    AfterTitle,
    /// At the top of a page with no title, after its front matter if it has
    /// one.
    Untitled,
}

/// How a field's line was written.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Form {
    /// `- Key: Value`, the form llm-wiki writes and poman reads.
    Bullet,
    /// `* Key: Value`, a list item marked with an asterisk, not a dash.
    Asterisk,
    /// `Key: Value`, with no bullet.
    Bare,
    /// `**Key:** Value`, `- **Key:** Value` or `- Key: **Value**`:
    /// asterisks around the key or at the start of the value. Only those at
    /// the start of the value are dropped: `**Value**` reads `Value**`.
    Bold,
}

/// A page's title: the text of its first `# ` line.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Title {
    text: String,
    line: usize,
}

impl Title {
    /// The title's text, without the `# ` and the spaces around it.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The title's line, counted from 1.
    #[must_use]
    pub const fn line(&self) -> usize {
        self.line
    }
}

/// One metadata field as the page wrote it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Field {
    key: String,
    value: String,
    line: usize,
    block: Block,
    form: Form,
}

impl Field {
    /// The key, without a bullet, asterisks or spaces around it.
    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }

    /// The value, with each continuation line joined to it by one space.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// The line the key is on, counted from 1.
    #[must_use]
    pub const fn line(&self) -> usize {
        self.line
    }

    /// Where on the page the field was found.
    #[must_use]
    pub const fn block(&self) -> Block {
        self.block
    }

    /// How the field's line was written.
    #[must_use]
    pub const fn form(&self) -> Form {
        self.form
    }
}

/// Everything a page says about itself: its title and its metadata fields,
/// in page order.
///
/// ```
/// use llm_wiki_core::page::{Block, Form, Page};
///
/// let page = Page::read("# Strict Gates\n\n- Status: Active\n- Scope: The gates\n  and their script.\n");
/// assert_eq!(page.title().map(|title| (title.text(), title.line())), Some(("Strict Gates", 1)));
/// let scope = page.fields().get(1).ok_or("no second field")?;
/// assert_eq!((scope.key(), scope.value()), ("Scope", "The gates and their script."));
/// assert_eq!((scope.line(), scope.block(), scope.form()), (4, Block::AfterTitle, Form::Bullet));
/// # Ok::<(), &str>(())
/// ```
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Page {
    title: Option<Title>,
    fields: Vec<Field>,
    byte_order_mark: bool,
}

impl Page {
    /// Reads a page's text.
    ///
    /// The title is the first line, after any front matter, that starts with
    /// `# ` once trimmed. Fields are read from three blocks, in page order:
    ///
    /// - front matter: a `---` line at the very top and the next `---` line,
    ///   every line between read, lines that are not fields skipped;
    /// - the lines just before the title, back to the first that is neither
    ///   a field nor a continuation;
    /// - the lines just after the title, or from the top of a page with no
    ///   title, up to the first that is neither, the first one a field.
    ///
    /// A field is a line not indented that holds a colon, its key what comes
    /// before the colon, with or without a leading `- ` and with asterisks
    /// around the key and at the start of the value dropped. A line indented
    /// by two spaces or a tab continues the field above it. Any such line
    /// right under the title is a field, prose included: "a ratio of 4:1"
    /// gives a field named after the text before its colon.
    ///
    /// A byte-order mark at the start is read as part of the first line, as
    /// llm-wiki's search always read it, so it hides front matter and a title
    /// on that line; [`BulletBlock::byte_order_mark`] reports it.
    #[must_use]
    pub fn read(text: &str) -> Self {
        let lines: Vec<&str> = text.lines().collect();
        let mut fields = Vec::new();
        let (front_matter, start) = front_matter(&lines);
        read_block(front_matter, Block::FrontMatter, &mut fields);

        let title = lines
            .iter()
            .enumerate()
            .skip(start)
            .find_map(|(index, line)| {
                line.trim()
                    .strip_prefix("# ")
                    .map(|text| (index, text.trim()))
            });

        let after_start = match title {
            Some((title_index, _)) => {
                let mut before: Vec<(usize, &str)> = numbered(&lines)
                    .take(title_index)
                    .skip(start)
                    .rev()
                    .skip_while(|(_, line)| is_blank(line))
                    .take_while(|(_, line)| field_line(line).is_some() || is_continuation(line))
                    .collect();
                before.reverse();
                read_block(before, Block::BeforeTitle, &mut fields);
                title_index + 1
            }
            None => start,
        };

        let mut first = true;
        let after = numbered(&lines)
            .skip(after_start)
            .skip_while(|(_, line)| is_blank(line))
            .take_while(|(_, line)| {
                let in_block = field_line(line).is_some() || (!first && is_continuation(line));
                first = false;
                in_block
            });
        let block = if title.is_some() {
            Block::AfterTitle
        } else {
            Block::Untitled
        };
        read_block(after, block, &mut fields);

        Self {
            title: title.map(|(index, text)| Title {
                text: text.to_owned(),
                line: index + 1,
            }),
            fields,
            byte_order_mark: text.starts_with('\u{FEFF}'),
        }
    }

    /// The page's title, if it has one.
    #[must_use]
    pub const fn title(&self) -> Option<&Title> {
        self.title.as_ref()
    }

    /// Every field found, in page order, a key written twice found twice.
    #[must_use]
    pub fn fields(&self) -> &[Field] {
        &self.fields
    }

    /// The wiki's view, the one llm-wiki's search reads: the title, and each
    /// key's value, a later field overriding an earlier one with its key.
    ///
    /// ```
    /// use llm_wiki_core::page::Page;
    ///
    /// let view = Page::read("---\nStatus: Draft\n---\n# Plan\n\n- Status: Active\n").wiki_view();
    /// assert_eq!(view.title(), Some("Plan"));
    /// assert_eq!(view.status(), Some("Active"));
    /// assert_eq!(view.document_class(), None);
    /// ```
    #[must_use]
    pub fn wiki_view(&self) -> WikiView {
        WikiView {
            title: self.title.as_ref().map(|title| title.text.clone()),
            fields: self
                .fields
                .iter()
                .map(|field| (field.key.clone(), field.value.clone()))
                .collect(),
        }
    }

    /// The bullet-block view, poman's: the `- Key: Value` fields just after
    /// the title, and every other field found, each with its line.
    ///
    /// ```
    /// use llm_wiki_core::page::{Field, Page};
    ///
    /// fn keys<'page>(fields: &[&'page Field]) -> Vec<(&'page str, usize)> {
    ///     fields.iter().map(|field| (field.key(), field.line())).collect()
    /// }
    ///
    /// let page = Page::read("# Deadline\n\n- Due: 2026-11-01\n**Owner:** Ana\n");
    /// let block = page.bullet_block();
    /// assert_eq!(keys(block.fields()), [("Due", 3)]);
    /// assert_eq!(keys(block.elsewhere()), [("Owner", 4)]);
    /// ```
    #[must_use]
    pub fn bullet_block(&self) -> BulletBlock<'_> {
        let (fields, elsewhere) = self
            .fields
            .iter()
            .partition(|field| field.block == Block::AfterTitle && field.form == Form::Bullet);
        BulletBlock {
            fields,
            elsewhere,
            byte_order_mark: self.byte_order_mark,
        }
    }
}

/// The title and fields llm-wiki's search reads of a page.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WikiView {
    title: Option<String>,
    fields: BTreeMap<String, String>,
}

impl WikiView {
    /// The page's title, if it has one.
    #[must_use]
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// Each key's value, sorted by key.
    #[must_use]
    pub const fn fields(&self) -> &BTreeMap<String, String> {
        &self.fields
    }

    /// The value of the field `key`, if the page has it.
    #[must_use]
    pub fn field(&self, key: &str) -> Option<&str> {
        self.fields.get(key).map(String::as_str)
    }

    /// The `Document Class` field.
    #[must_use]
    pub fn document_class(&self) -> Option<&str> {
        self.field("Document Class")
    }

    /// The `Status` field.
    #[must_use]
    pub fn status(&self) -> Option<&str> {
        self.field("Status")
    }
}

/// The bullet block just after a page's title, and every field found
/// anywhere else, both in page order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BulletBlock<'page> {
    fields: Vec<&'page Field>,
    elsewhere: Vec<&'page Field>,
    byte_order_mark: bool,
}

impl<'page> BulletBlock<'page> {
    /// The `- Key: Value` fields just after the title.
    #[must_use]
    pub fn fields(&self) -> &[&'page Field] {
        &self.fields
    }

    /// The fields found in any other block or form: front matter, before the
    /// title, on a page with no title, bare, bold or marked with `* `.
    #[must_use]
    pub fn elsewhere(&self) -> &[&'page Field] {
        &self.elsewhere
    }

    /// Whether the page starts with a byte-order mark. The reader keeps the
    /// mark in the first line, so front matter or a title there is not read:
    /// a page with one may hold fields neither list shows.
    ///
    /// ```
    /// use llm_wiki_core::page::Page;
    ///
    /// let page = Page::read("\u{FEFF}---\ndue: 2026-11-01\n---\n# Rent\n\n- Status: Todo\n");
    /// let block = page.bullet_block();
    /// assert!(block.byte_order_mark());
    /// assert!(block.elsewhere().is_empty());
    /// ```
    #[must_use]
    pub const fn byte_order_mark(&self) -> bool {
        self.byte_order_mark
    }
}

/// Each line with its index.
fn numbered<'text>(
    lines: &[&'text str],
) -> impl DoubleEndedIterator<Item = (usize, &'text str)> + ExactSizeIterator {
    lines.iter().copied().enumerate()
}

/// The front matter's lines, and the index of the line after it: 0 when the
/// page has none.
fn front_matter<'text>(lines: &[&'text str]) -> (Vec<(usize, &'text str)>, usize) {
    if let Some((first, rest)) = lines.split_first()
        && first.trim() == "---"
        && let Some(closing) = rest.iter().position(|line| line.trim() == "---")
    {
        return (numbered(lines).skip(1).take(closing).collect(), closing + 2);
    }
    (Vec::new(), 0)
}

/// Reads one block's lines into fields, each continuation line joined to
/// the field above it.
fn read_block<'text>(
    lines: impl IntoIterator<Item = (usize, &'text str)>,
    block: Block,
    fields: &mut Vec<Field>,
) {
    let mut current: Option<Field> = None;
    for (index, line) in lines {
        if let Some((key, value, form)) = field_line(line) {
            fields.extend(current.replace(Field {
                key: key.to_owned(),
                value: value.to_owned(),
                line: index + 1,
                block,
                form,
            }));
        } else if is_continuation(line)
            && let Some(field) = current.as_mut()
        {
            let continuation = line.trim();
            if !continuation.is_empty() {
                if !field.value.is_empty() {
                    field.value.push(' ');
                }
                field.value.push_str(continuation);
            }
        }
    }
    fields.extend(current);
}

/// A field's key, value and form, when the line is a field.
fn field_line(line: &str) -> Option<(&str, &str, Form)> {
    if line.starts_with([' ', '\t']) {
        return None;
    }
    let line = line.trim_end();
    let (rest, bullet) = line
        .strip_prefix("- ")
        .map_or((line, false), |rest| (rest, true));
    let (written_key, value) = rest.split_once(':')?;
    let written_key = written_key.trim();
    let key = written_key
        .trim_start_matches('*')
        .trim_end_matches('*')
        .trim();
    if key.is_empty() {
        return None;
    }
    let value = value.trim();
    let (value, bold_value) = value
        .strip_prefix("**")
        .map_or((value, false), |value| (value.trim(), true));
    // The key keeps whatever a `* ` marker leaves of it, as search has always
    // read it; only the form tells the marker apart from bold asterisks.
    let asterisk = if bullet {
        None
    } else {
        written_key.strip_prefix("* ").map(str::trim_start)
    };
    let unmarked = asterisk.unwrap_or(written_key);
    let bold_key = unmarked
        .trim_start_matches('*')
        .trim_end_matches('*')
        .trim()
        .len()
        != unmarked.len();
    let form = if bold_value || bold_key {
        Form::Bold
    } else if bullet {
        Form::Bullet
    } else if asterisk.is_some() {
        Form::Asterisk
    } else {
        Form::Bare
    };
    Some((key, value, form))
}

fn is_continuation(line: &str) -> bool {
    line.starts_with("  ") || line.starts_with('\t')
}

fn is_blank(line: &str) -> bool {
    line.trim().is_empty()
}
