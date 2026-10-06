# Search Eval Wiki

A frozen copy of this repository's `wiki/` at commit `7940130`, indexed by
`search::qmd_rs::tests::fixed_eval_queries_keep_expected_targets_in_top_two`.

The test asserts that each fixed query finds one of its expected pages in the
top two. Against the live `wiki/` it failed whenever new pages shifted the
scores, not when search got worse, so it reads this copy instead. Leave the
copy as it is: refresh it only together with the test's queries and expected
targets, and only from a commit where every expected target held.
