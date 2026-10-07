//! Integration tests: pages written as llm-wiki writes them, and as poman's
//! files are to be written, read through both views.

use llm_wiki_core::page::{Block, Form, Page};

const PLAN: &str = concat!(
    "# Plan: The Shared Page Reader\n",
    "\n",
    "- Document Class: Plan\n",
    "- Status: Active\n",
    "- Branch: `pm2`\n",
    "- Date: 2026-10-06\n",
    "- Category: poman development\n",
    "- Scope: Carry out PM2 of the poman roadmap.\n",
    "- Sources:\n",
    "  - `wiki/roadmaps/poman.roadmap.md`, PM2\n",
    "  - issue #44\n",
    "\n",
    "## What This Proves\n",
    "\n",
    "Status: not a field, below the block.\n",
);

#[test]
fn a_plan_reads_as_search_reads_it() {
    let view = Page::read(PLAN).wiki_view();
    assert_eq!(view.title(), Some("Plan: The Shared Page Reader"));
    assert_eq!(view.document_class(), Some("Plan"));
    assert_eq!(view.status(), Some("Active"));
    assert_eq!(view.field("Branch"), Some("`pm2`"));
    assert_eq!(
        view.field("Sources"),
        Some("- `wiki/roadmaps/poman.roadmap.md`, PM2 - issue #44")
    );
    assert_eq!(view.fields().len(), 7);
}

#[test]
fn a_plan_is_all_bullet_block() {
    let page = Page::read(PLAN);
    let block = page.bullet_block();
    let lines: Vec<usize> = block.fields().iter().map(|field| field.line()).collect();
    assert_eq!(lines, [3, 4, 5, 6, 7, 8, 9]);
    assert_eq!(block.elsewhere().len(), 0);
}

#[test]
fn front_matter_is_found_with_its_lines_for_poman_to_refuse() {
    let page = Page::read(
        "---\ndue: 2026-11-01\n---\n# Pay the rent\n\n- Status: Todo\n**Due:** 2026-11-01\n",
    );
    let elsewhere: Vec<(&str, usize, Block, Form)> = page
        .bullet_block()
        .elsewhere()
        .iter()
        .map(|field| (field.key(), field.line(), field.block(), field.form()))
        .collect();
    assert_eq!(
        elsewhere,
        [
            ("due", 2, Block::FrontMatter, Form::Bare),
            ("Due", 7, Block::AfterTitle, Form::Bold),
        ]
    );
}
