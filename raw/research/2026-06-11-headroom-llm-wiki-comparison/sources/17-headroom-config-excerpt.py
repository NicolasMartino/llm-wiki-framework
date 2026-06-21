# Partial capture of `headroom/config.py` from `chopratejas/headroom` at the
# `main` branch tip on 2026-06-15. Captured via WebFetch summarization plus a
# line-numbered blame probe; verbatim quotes of the literal data structures
# below were cross-checked against the line-numbered view, but the rest of the
# file is intentionally not transcribed here. Treat this file as evidence for
# the `DEFAULT_EXCLUDE_TOOLS` and `DEFAULT_TOOL_PROFILES` shapes only.
#
# Capture method: two WebFetch calls against
#   https://raw.githubusercontent.com/chopratejas/headroom/main/headroom/config.py
#   https://github.com/chopratejas/headroom/blame/main/headroom/config.py
# The blame view was used to recover line numbers; the raw view was used to
# recover string contents. Both views were asked for verbatim quotes.
#
# Upstream inconsistency noted at capture time:
#   - Line 210 comment reads: "# Bash is NOT excluded -- its outputs (build
#     logs, test output) are ideal compression targets."
#   - Line 219 inside the frozenset literal reads: "Bash",
#   The frozenset literal is what runs. The comment is stale or never
#   reconciled. As of capture, agents using the Bash tool ARE protected from
#   compression by default, contrary to the comment's claim. A future upstream
#   commit that re-aligns the literal with the comment would silently remove
#   that protection. The llm-wiki Headroom Runtime Companion proposal handles
#   this by defensively re-adding the full default set into
#   HEADROOM_EXCLUDE_TOOLS regardless of upstream defaults.


# Verified comment block immediately above DEFAULT_EXCLUDE_TOOLS
# (lines 202-210, blame view, quoted verbatim per WebFetch):
#
# 202  # Default tools to exclude from compression (local file/code tools)
# 203  # Read: Returns exact file content needed for Edit tool's old_string matching.
# 204  #   Compressing would break the edit workflow.
# 205  # Glob: Returns compact file path lists used for navigation. Low token count,
# 206  #   not worth compressing.
# 207  # Tool outputs that are reference data and must NOT be compressed.
# 208  # Read/Glob/Grep contain exact file contents/search results the agent needs for edits.
# 209  # Write/Edit record what changes were made -- compressing them causes duplicate/conflicting edits.
# 210  # Bash is NOT excluded -- its outputs (build logs, test output) are ideal compression targets.


# Verified DEFAULT_EXCLUDE_TOOLS literal
# (lines 211-227, blame view, quoted verbatim per WebFetch):

DEFAULT_EXCLUDE_TOOLS = frozenset(
    {
        "Read",
        "Glob",
        "Grep",
        "Write",
        "Edit",
        "Bash",
        # Lowercase variants for case-insensitive matching
        "read",
        "glob",
        "grep",
        "write",
        "edit",
        "bash",
    }
)


# Verified DEFAULT_TOOL_PROFILES literal (line numbers not recovered;
# verbatim per the raw-view WebFetch):
#
# Note: Bash and Grep are already in DEFAULT_EXCLUDE_TOOLS, so their per-tool
# compression profile is dead at the routing layer (the exclusion check
# happens before bias is applied). Only the WebFetch entry is active given
# current defaults.

DEFAULT_TOOL_PROFILES = {
    # Search results: keep more matches for accuracy
    "Grep": "PROFILE_PRESETS[conservative]",
    "grep": "PROFILE_PRESETS[conservative]",
    # Logs/output: balanced compression
    "Bash": "PROFILE_PRESETS[moderate]",
    "bash": "PROFILE_PRESETS[moderate]",
    # Web pages are verbose, compress aggressively
    "WebFetch": "PROFILE_PRESETS[aggressive]",
    "webfetch": "PROFILE_PRESETS[aggressive]",
}


# PROFILE_PRESETS bias and floor values, paraphrased from the raw-view fetch
# (presets are CompressionProfile instances; the meaningful fields are bias
# and min_k):
#
#   conservative -> bias=1.5, min_k=5
#   moderate     -> bias=1.0, min_k=3
#   aggressive   -> bias=0.7, min_k=3
#
# Unmapped tool names fall back to moderate.
