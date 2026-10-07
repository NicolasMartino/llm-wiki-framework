use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use serde_json::{Value, json};
use tempfile::TempDir;

mod support;

fn mcp_tool_name(base: &'static str) -> &'static str {
    if option_env!("LLM_WIKI_COMPILED_INSTANCE") != Some("test") {
        return base;
    }

    match base {
        "llm_wiki_read" => "llm_wiki_read_test",
        "llm_wiki_search" => "llm_wiki_search_test",
        "llm_wiki_search_all" => "llm_wiki_search_all_test",
        "llm_wiki_index" => "llm_wiki_index_test",
        "llm_wiki_register" => "llm_wiki_register_test",
        "llm_wiki_status" => "llm_wiki_status_test",
        _ => base,
    }
}

fn managed_home_dir_name() -> &'static str {
    if option_env!("LLM_WIKI_COMPILED_INSTANCE") == Some("test") {
        ".llm_wiki-test"
    } else {
        ".llm_wiki"
    }
}

fn binary_stem() -> &'static str {
    if option_env!("LLM_WIKI_COMPILED_INSTANCE") == Some("test") {
        "llm-wiki-test"
    } else {
        "llm-wiki"
    }
}

#[test]
fn mcp_lists_and_calls_wiki_read_tool() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path());
    let responses = run_mcp_session(
        home.path(),
        &project,
        &[
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-06-18",
                    "capabilities": {},
                    "clientInfo": {"name": "llm-wiki-test", "version": "0.0.0"}
                }
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/list"
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "tools/call",
                "params": {
                    "name": mcp_tool_name("llm_wiki_read"),
                    "arguments": {"path": "wiki/index.md"}
                }
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 4,
                "method": "tools/call",
                "params": {
                    "name": mcp_tool_name("llm_wiki_read"),
                    "arguments": {"path": "raw"}
                }
            }),
        ],
    );

    let tools = responses
        .iter()
        .find(|response| response["id"] == 2)
        .expect("tools/list response");
    let tool_names: Vec<_> = tools["result"]["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .map(|tool| tool["name"].as_str().expect("tool name"))
        .collect();
    for expected in [
        mcp_tool_name("llm_wiki_read"),
        mcp_tool_name("llm_wiki_search"),
        mcp_tool_name("llm_wiki_search_all"),
        mcp_tool_name("llm_wiki_index"),
        mcp_tool_name("llm_wiki_register"),
        mcp_tool_name("llm_wiki_status"),
    ] {
        assert!(tool_names.contains(&expected), "missing tool {expected}");
    }

    let initialize = responses
        .iter()
        .find(|response| response["id"] == 1)
        .expect("initialize response");
    let instructions = initialize["result"]["instructions"]
        .as_str()
        .expect("server instructions");
    assert!(instructions.contains("wiki/index.md"));
    assert!(instructions.contains(mcp_tool_name("llm_wiki_read")));

    let read = responses
        .iter()
        .find(|response| response["id"] == 3)
        .expect("read response");
    let payload = tool_payload(read);
    assert_eq!(
        payload["project_root"].as_str().expect("project root"),
        fs::canonicalize(&project)
            .expect("canonical project")
            .to_string_lossy()
    );
    assert_eq!(payload["path"], "wiki/index.md");
    assert_eq!(payload["tree"], "wiki");
    assert_eq!(payload["scope"], "wiki");
    assert_eq!(payload["encoding"], "utf-8");
    assert_eq!(payload["byte_len"], 13);
    assert_eq!(payload["content"], "# Wiki Index\n");
    assert!(payload["sha256"].as_str().expect("sha256").len() == 64);

    let read_directory = responses
        .iter()
        .find(|response| response["id"] == 4)
        .expect("read directory response");
    let payload = tool_payload(read_directory);
    assert_eq!(payload["path"], "raw");
    assert_eq!(payload["tree"], "raw");
    assert_eq!(payload["scope"], "raw");
    assert_eq!(payload["encoding"], "directory");
    assert_eq!(payload["content_omitted"], "directory listing");
    let entries = payload["entries"].as_array().expect("directory entries");
    let source = entries
        .iter()
        .find(|entry| entry["path"] == "raw/source.md")
        .expect("raw source entry");
    assert_eq!(source["tree"], "raw");
    assert_eq!(source["kind"], "file");
    assert_eq!(source["bytes"], 7);
}

#[test]
fn mcp_notifications_receive_no_response() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path());

    let responses = run_mcp_session(
        home.path(),
        &project,
        &[
            // A bare notification (no `id`) — must produce no reply.
            json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
            // A known method with no `id` is still a notification — must not be answered.
            json!({"jsonrpc": "2.0", "method": "tools/list"}),
            // A normal request that must be answered.
            json!({"jsonrpc": "2.0", "id": 7, "method": "tools/list"}),
        ],
    );

    assert_eq!(
        responses.len(),
        1,
        "only the id-bearing request should get a response, got: {responses:?}"
    );
    assert_eq!(responses[0]["id"], 7);
    assert!(responses[0]["result"]["tools"].is_array());
}

#[test]
fn cli_read_returns_same_schema_for_wiki_file() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path());

    let output = llm_wiki(home.path())
        .current_dir(&project)
        .args(["read", "wiki/index.md"])
        .output()
        .expect("read output");
    assert!(
        output.status.success(),
        "read failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let payload: Value = serde_json::from_slice(&output.stdout).expect("read json");
    assert_eq!(payload["path"], "wiki/index.md");
    assert_eq!(payload["tree"], "wiki");
    assert_eq!(payload["content"], "# Wiki Index\n");
}

#[test]
fn mcp_status_returns_structured_payload() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path());

    let responses = run_mcp_session(
        home.path(),
        &project,
        &[
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/call",
                "params": {
                    "name": mcp_tool_name("llm_wiki_register"),
                    "arguments": {
                        "path": project,
                        "id": "fixture",
                        "name": "Fixture"
                    }
                }
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/call",
                "params": {
                    "name": mcp_tool_name("llm_wiki_status"),
                    "arguments": {"project": "fixture"}
                }
            }),
        ],
    );

    let payload = tool_payload(
        responses
            .iter()
            .find(|response| response["id"] == 2)
            .expect("status response"),
    );
    assert_eq!(payload["installed"], false);
    assert!(
        payload["manifest_path"]
            .as_str()
            .expect("manifest path")
            .ends_with(&format!("{}/manifest.json", managed_home_dir_name()))
    );
    assert_eq!(payload["binary_stem"], binary_stem());
    let project_payload = &payload["project"];
    assert_eq!(project_payload["requested_project"], "fixture");
    assert_eq!(project_payload["registered"], true);
    assert_eq!(project_payload["project_id"], "fixture");
    assert_eq!(project_payload["project_name"], "Fixture");
    assert_eq!(
        project_payload["project_root"]
            .as_str()
            .expect("project root"),
        fs::canonicalize(&project)
            .expect("canonical project")
            .to_string_lossy()
    );
    assert_eq!(project_payload["index_present"], false);
    assert!(project_payload["index_freshness"].is_null());
    assert_eq!(project_payload["index_state"], "missing");
    assert_eq!(project_payload["indexed_files"], 0);
    assert_eq!(project_payload["lexical_ready"], false);
    assert_eq!(project_payload["semantic_ready"], false);
    assert_eq!(project_payload["hybrid_ready"], false);
    assert_eq!(project_payload["readiness_reason"], "missing");
}

#[test]
fn mcp_status_does_not_treat_semantic_files_as_readiness() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path());
    fs::write(
        project.join("wiki/index.md"),
        "# Wiki Index\n\nsearchable status token\n",
    )
    .expect("index");
    install_without_llm_search(home.path());

    run_mcp_session(
        home.path(),
        &project,
        &[
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/call",
                "params": {
                    "name": mcp_tool_name("llm_wiki_register"),
                    "arguments": {
                        "path": project,
                        "id": "fixture",
                        "name": "Fixture"
                    }
                }
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/call",
                "params": {
                    "name": mcp_tool_name("llm_wiki_index"),
                    "arguments": {"project": "fixture"}
                }
            }),
        ],
    );

    let semantic_dir = home
        .path()
        .join(managed_home_dir_name())
        .join("indexes")
        .join("fixture");
    fs::create_dir_all(&semantic_dir).expect("semantic dir");
    fs::write(semantic_dir.join("semantic-index.json"), "{}").expect("semantic metadata");
    fs::write(semantic_dir.join("semantic-vectors.json"), "{}").expect("semantic vectors");

    let responses = run_mcp_session(
        home.path(),
        &project,
        &[json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": mcp_tool_name("llm_wiki_status"),
                "arguments": {"project": "fixture"}
            }
        })],
    );
    let payload = tool_payload(responses.first().expect("status response"));
    let project_payload = &payload["project"];
    assert_eq!(project_payload["index_present"], true);
    assert_eq!(project_payload["lexical_ready"], true);
    assert_eq!(project_payload["semantic_ready"], false);
    assert_eq!(project_payload["hybrid_ready"], false);
    assert_eq!(project_payload["readiness_reason"], "llm_search_disabled");
}

#[test]
fn mcp_resources_list_and_read_canonical_project_material() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path());

    let responses = run_mcp_session(
        home.path(),
        &project,
        &[
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "resources/list"
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "resources/read",
                "params": {"uri": "llm-wiki://project/wiki/index.md"}
            }),
        ],
    );

    let resources = responses
        .iter()
        .find(|response| response["id"] == 1)
        .expect("resources/list response");
    let uris: Vec<_> = resources["result"]["resources"]
        .as_array()
        .expect("resources")
        .iter()
        .map(|resource| resource["uri"].as_str().expect("uri"))
        .collect();
    assert!(uris.contains(&"llm-wiki://project/wiki/index.md"));
    assert!(uris.contains(&"llm-wiki://project/AGENTS.md"));

    let read = responses
        .iter()
        .find(|response| response["id"] == 2)
        .expect("resources/read response");
    assert_eq!(read["result"]["contents"][0]["text"], "# Wiki Index\n");
}

#[test]
fn mcp_resources_accept_uppercase_agents_filename() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path());
    fs::remove_file(project.join("AGENTS.md")).expect("remove lowercase agents");
    fs::write(
        project.join("AGENTS.MD"),
        "# Uppercase Agent Instructions\n",
    )
    .expect("agents");

    let responses = run_mcp_session(
        home.path(),
        &project,
        &[
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "resources/list"
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "resources/read",
                "params": {"uri": "llm-wiki://project/AGENTS.md"}
            }),
        ],
    );

    let resources = responses
        .iter()
        .find(|response| response["id"] == 1)
        .expect("resources/list response");
    let uris: Vec<_> = resources["result"]["resources"]
        .as_array()
        .expect("resources")
        .iter()
        .map(|resource| resource["uri"].as_str().expect("uri"))
        .collect();
    assert!(uris.contains(&"llm-wiki://project/AGENTS.md"));

    let read = responses
        .iter()
        .find(|response| response["id"] == 2)
        .expect("resources/read response");
    assert_eq!(
        read["result"]["contents"][0]["text"],
        "# Uppercase Agent Instructions\n"
    );
}

#[test]
fn mcp_prompts_list_and_get_operation_guidance() {
    let home = TempDir::new().expect("home");
    let project = fixture_project(home.path());

    let responses = run_mcp_session(
        home.path(),
        &project,
        &[
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "prompts/list"
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "prompts/get",
                "params": {
                    "name": "wiki_query",
                    "arguments": {"query": "what changed?"}
                }
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "prompts/get",
                "params": {"name": "wiki_lint"}
            }),
        ],
    );

    let list = responses
        .iter()
        .find(|response| response["id"] == 1)
        .expect("prompts/list response");
    let prompt_names: Vec<_> = list["result"]["prompts"]
        .as_array()
        .expect("prompts")
        .iter()
        .map(|prompt| prompt["name"].as_str().expect("prompt name"))
        .collect();
    assert!(prompt_names.contains(&"wiki_query"));
    assert!(prompt_names.contains(&"wiki_lint"));

    let get = responses
        .iter()
        .find(|response| response["id"] == 2)
        .expect("prompts/get response");
    let messages = get["result"]["messages"].as_array().expect("messages");
    assert_eq!(messages[0]["role"].as_str(), Some("user"));
    let text = messages[0]["content"]["text"].as_str().expect("text");
    assert!(text.contains("what changed?"));
    assert!(text.contains("wiki/index.md"));
    assert!(text.contains(mcp_tool_name("llm_wiki_search")));
    assert!(text.contains(mcp_tool_name("llm_wiki_read")));

    // Choice 7 of the poman deadline type: lint leaves deadline files to poman.
    let lint = responses
        .iter()
        .find(|response| response["id"] == 3)
        .expect("wiki_lint prompts/get response");
    let text = lint["result"]["messages"][0]["content"]["text"]
        .as_str()
        .expect("text");
    assert!(text.contains(
        "Leave deadline files (`wiki/deadlines/<slug>.deadline.md`) to `poman check`: \
         they carry poman's fields only, with no metadata block, and the index points \
         to their folder, not to each file, so they are not orphans."
    ));
}

#[test]
fn mcp_resources_expose_operation_specs_when_present() {
    let home = TempDir::new().expect("home");
    let project = fixture_project(home.path());
    fs::create_dir_all(project.join("wiki/specs")).expect("specs dir");
    fs::write(
        project.join("wiki/specs/documentation-model.spec.md"),
        "# Documentation Model\n",
    )
    .expect("documentation model spec");
    fs::write(
        project.join("wiki/specs/wiki-query-skill.spec.md"),
        "# Wiki Query\n",
    )
    .expect("query spec");

    let responses = run_mcp_session(
        home.path(),
        &project,
        &[
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "resources/list"
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "resources/read",
                "params": {"uri": "llm-wiki://project/wiki/specs/documentation-model.spec.md"}
            }),
        ],
    );

    let list = responses
        .iter()
        .find(|response| response["id"] == 1)
        .expect("resources/list response");
    let uris: Vec<_> = list["result"]["resources"]
        .as_array()
        .expect("resources")
        .iter()
        .map(|resource| resource["uri"].as_str().expect("uri"))
        .collect();
    assert!(uris.contains(&"llm-wiki://project/wiki/specs/documentation-model.spec.md"));
    assert!(uris.contains(&"llm-wiki://project/wiki/specs/wiki-query-skill.spec.md"));

    let read = responses
        .iter()
        .find(|response| response["id"] == 2)
        .expect("resources/read response");
    let contents = read["result"]["contents"].as_array().expect("contents");
    assert!(
        contents[0]["text"]
            .as_str()
            .expect("text")
            .contains("Documentation Model")
    );
}

#[test]
fn mcp_register_tool_updates_project_registry() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path());

    let responses = run_mcp_session(
        home.path(),
        &project,
        &[json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": mcp_tool_name("llm_wiki_register"),
                "arguments": {
                    "path": project,
                    "id": "fixture",
                    "name": "Fixture"
                }
            }
        })],
    );

    let payload = tool_payload(responses.first().expect("register response"));
    assert!(
        payload["stdout"]
            .as_str()
            .expect("stdout")
            .contains("fixture")
    );

    let output = llm_wiki(home.path())
        .args(["projects", "--format", "json"])
        .output()
        .expect("projects output");
    assert!(output.status.success());
    let projects: Value = serde_json::from_slice(&output.stdout).expect("projects json");
    assert_eq!(projects["projects"][0]["id"], "fixture");
    assert_eq!(projects["projects"][0]["name"], "Fixture");
}

#[test]
fn mcp_register_index_and_search_round_trip() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path());
    fs::write(
        project.join("wiki/index.md"),
        "# Wiki Index\n\nreciprocal rank fusion needle\n",
    )
    .expect("index");
    install_without_llm_search(home.path());

    let responses = run_mcp_session(
        home.path(),
        &project,
        &[
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/call",
                "params": {
                    "name": mcp_tool_name("llm_wiki_register"),
                    "arguments": {
                        "path": project,
                        "id": "fixture",
                        "name": "Fixture"
                    }
                }
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/call",
                "params": {
                    "name": mcp_tool_name("llm_wiki_index"),
                    "arguments": {"project": "fixture", "force": true}
                }
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "tools/call",
                "params": {
                    "name": mcp_tool_name("llm_wiki_search"),
                    "arguments": {
                        "query": "reciprocal rank fusion",
                        "project": "fixture",
                        "mode": "lexical",
                        "limit": 5
                    }
                }
            }),
        ],
    );

    assert!(responses.iter().any(|response| response["id"] == 1));
    assert!(responses.iter().any(|response| response["id"] == 2));
    let search = tool_payload(
        responses
            .iter()
            .find(|response| response["id"] == 3)
            .expect("search response"),
    );
    assert_eq!(search["requested_mode"], "lexical");
    assert_eq!(search["selected_mode"], "lexical");
    assert!(
        search["results"]
            .as_array()
            .expect("results")
            .iter()
            .any(|result| result.to_string().contains("wiki/index.md"))
    );
}

#[test]
fn mcp_search_on_a_stale_index_carries_the_warning() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path());
    install_without_llm_search(home.path());
    let register = llm_wiki(home.path())
        .args(["register", "--id", "fixture", "--name", "Fixture"])
        .arg(&project)
        .output()
        .expect("register");
    assert!(register.status.success());
    let index = llm_wiki(home.path())
        .args(["index", "--project", "fixture", "--force"])
        .output()
        .expect("index");
    assert!(index.status.success());
    fs::write(
        project.join("wiki/index.md"),
        "# Wiki Index\n\nfreshly edited\n",
    )
    .expect("edit index");
    // With the rebuild turned off, search answers stale.
    fs::create_dir_all(project.join(".llm_wiki")).expect("project manifest dir");
    fs::write(
        project.join(".llm_wiki/search.toml"),
        "schema_version = 1\nupdated_at = \"2026-10-07T00:00:00Z\"\n\n[project]\nllm_search_enabled = false\nrebuild_stale_index = false\nconfigured_at = \"2026-10-07T00:00:00Z\"\nconfigured_by_version = \"test\"\n",
    )
    .expect("project search profile");

    let responses = run_mcp_session(
        home.path(),
        &project,
        &[json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": mcp_tool_name("llm_wiki_search"),
                "arguments": {"query": "wiki index", "project": "fixture"}
            }
        })],
    );

    let search = tool_payload(&responses[0]);
    let warnings = search["warnings"].as_array().expect("warnings array");
    assert_eq!(warnings.len(), 1, "{search}");
    assert_eq!(
        warnings[0]["message"],
        format!(
            "search index stale for project fixture; run `{} index --project fixture` (about a second for a word-match index) and search again",
            binary_stem()
        )
    );
}

#[test]
fn mcp_cli_tool_failure_returns_tool_error_result() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path());
    install_without_llm_search(home.path());

    let responses = run_mcp_session(
        home.path(),
        &project,
        &[json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": mcp_tool_name("llm_wiki_search"),
                "arguments": {
                    "query": "wiki",
                    "project": "missing-project"
                }
            }
        })],
    );

    let response = responses.first().expect("search response");
    assert_eq!(response["id"], 1);
    assert!(response.get("error").is_none());
    let message = tool_error_message(response);
    assert!(
        message.contains("llm-wiki search") || message.contains("missing-project"),
        "unexpected tool error message: {message}"
    );
}

#[test]
fn mcp_wiki_read_rejects_paths_outside_wiki_and_raw() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path());

    let responses = run_mcp_session(
        home.path(),
        &project,
        &[json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": mcp_tool_name("llm_wiki_read"),
                "arguments": {"path": "AGENTS.md"}
            }
        })],
    );

    let response = responses.first().expect("response");
    assert_eq!(response["id"], 1);
    let message = tool_error_message(response);
    assert!(
        message.contains("wiki/ or raw/"),
        "unexpected tool error message: {message}"
    );
}

#[test]
fn mcp_unknown_tool_returns_json_rpc_error() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path());

    let responses = run_mcp_session(
        home.path(),
        &project,
        &[json!({
            "jsonrpc": "2.0",
            "id": 99,
            "method": "tools/call",
            "params": {
                "name": "llm_wiki_missing",
                "arguments": {}
            }
        })],
    );

    let response = responses.first().expect("response");
    assert_eq!(response["id"], 99);
    assert_eq!(response["error"]["code"], -32602);
    assert!(
        response["error"]["message"]
            .as_str()
            .expect("error message")
            .contains("unknown tool llm_wiki_missing")
    );
}

#[test]
fn mcp_ping_returns_empty_result() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path());

    let responses = run_mcp_session(
        home.path(),
        &project,
        &[json!({"jsonrpc": "2.0", "id": 42, "method": "ping"})],
    );

    let response = responses.first().expect("ping response");
    assert_eq!(response["id"], 42);
    assert!(
        response.get("error").is_none(),
        "ping must not return an error: {response:?}"
    );
    assert!(
        response["result"].is_object(),
        "ping must return an (empty) object result: {response:?}"
    );
}

#[test]
fn mcp_wiki_read_omits_binary_and_oversized_content() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path());
    fs::write(project.join("raw/binary.dat"), [0, 159, 146, 150]).expect("binary");
    fs::write(project.join("wiki/large.md"), "x".repeat(1_048_577)).expect("large");

    let responses = run_mcp_session(
        home.path(),
        &project,
        &[
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/call",
                "params": {
                    "name": mcp_tool_name("llm_wiki_read"),
                    "arguments": {"path": "raw/binary.dat"}
                }
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/call",
                "params": {
                    "name": mcp_tool_name("llm_wiki_read"),
                    "arguments": {"path": "wiki/large.md"}
                }
            }),
        ],
    );

    let binary = tool_payload(
        responses
            .iter()
            .find(|response| response["id"] == 1)
            .expect("binary response"),
    );
    assert_eq!(binary["tree"], "raw");
    assert_eq!(binary["encoding"], "binary");
    assert_eq!(binary["byte_len"], 4);
    assert!(binary.get("content").is_none());
    assert_eq!(binary["content_omitted"], "binary");

    let large = tool_payload(
        responses
            .iter()
            .find(|response| response["id"] == 2)
            .expect("large response"),
    );
    assert_eq!(large["tree"], "wiki");
    assert_eq!(large["encoding"], "utf-8");
    assert_eq!(large["byte_len"], 1_048_577);
    assert!(large.get("content").is_none());
    assert!(
        large["content_omitted"]
            .as_str()
            .expect("large omitted reason")
            .contains("1048576")
    );
}

#[test]
fn mcp_wiki_read_rejects_parent_escape() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path());
    fs::write(workspace.path().join("secret.md"), "secret\n").expect("secret");

    let responses = run_mcp_session(
        home.path(),
        &project,
        &[json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": mcp_tool_name("llm_wiki_read"),
                "arguments": {"path": "wiki/../../secret.md"}
            }
        })],
    );

    let message = tool_error_message(&responses[0]);
    assert!(
        message.contains("wiki/ or raw/"),
        "unexpected tool error message: {message}"
    );
}

#[cfg(unix)]
#[test]
fn mcp_wiki_read_rejects_symlink_escape() {
    use std::os::unix::fs::symlink;

    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path());
    let secret = workspace.path().join("secret.md");
    fs::write(&secret, "secret\n").expect("secret");
    symlink(&secret, project.join("wiki/escape.md")).expect("symlink");

    let responses = run_mcp_session(
        home.path(),
        &project,
        &[json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": mcp_tool_name("llm_wiki_read"),
                "arguments": {"path": "wiki/escape.md"}
            }
        })],
    );

    let message = tool_error_message(&responses[0]);
    assert!(
        message.contains("wiki/ or raw/"),
        "unexpected tool error message: {message}"
    );
}

#[test]
fn mcp_search_tools_describe_compact_paging() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path());
    let responses = run_mcp_session(
        home.path(),
        &project,
        &[
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-06-18",
                    "capabilities": {},
                    "clientInfo": {"name": "llm-wiki-test", "version": "0.0.0"}
                }
            }),
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
        ],
    );

    let tools = responses
        .iter()
        .find(|response| response["id"] == 2)
        .expect("tools/list response");
    let tools = tools["result"]["tools"].as_array().expect("tools array");
    for name in [
        mcp_tool_name("llm_wiki_search"),
        mcp_tool_name("llm_wiki_search_all"),
    ] {
        let tool = tools
            .iter()
            .find(|tool| tool["name"] == name)
            .unwrap_or_else(|| panic!("missing tool {name}"));
        let description = tool["description"].as_str().expect("tool description");
        for field in ["next_offset", "has_more"] {
            assert!(
                description.contains(field),
                "{name} description does not mention {field}: {description}"
            );
        }
        let properties = &tool["inputSchema"]["properties"];
        for field in ["limit", "page_size", "offset"] {
            assert!(
                properties[field]["description"].is_string(),
                "{name} {field} has no description"
            );
        }
        assert_eq!(properties["limit"]["default"], 10, "{name} limit default");
        let page_size = properties["page_size"]["description"]
            .as_str()
            .expect("page_size description");
        assert!(
            page_size.contains("limit"),
            "{name} page_size description does not give its default: {page_size}"
        );
    }
}

#[test]
fn mcp_search_accepts_compact_pagination_args() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path());
    fs::write(
        project.join("wiki/index.md"),
        "# Wiki Index\n\nreciprocal rank fusion appears in compact MCP search.\n",
    )
    .expect("index");
    install_without_llm_search(home.path());

    let responses = run_mcp_session(
        home.path(),
        &project,
        &[
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/call",
                "params": {
                    "name": mcp_tool_name("llm_wiki_register"),
                    "arguments": {
                        "path": project,
                        "id": "fixture",
                        "name": "Fixture"
                    }
                }
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/call",
                "params": {
                    "name": mcp_tool_name("llm_wiki_index"),
                    "arguments": {"project": "fixture", "force": true}
                }
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "tools/call",
                "params": {
                    "name": mcp_tool_name("llm_wiki_search"),
                    "arguments": {
                        "query": "reciprocal rank fusion",
                        "project": "fixture",
                        "mode": "lexical",
                        "compact": true,
                        "page_size": 1,
                        "offset": 0
                    }
                }
            }),
        ],
    );

    let response = responses
        .iter()
        .find(|response| response["id"] == 3)
        .expect("search response");
    let payload = tool_payload(response);
    assert_eq!(payload["result_count"], 1, "payload: {payload:#}");
    assert_eq!(payload["page_size"], 1);
    assert_eq!(payload["offset"], 0);
    assert!(payload["next_offset"].is_null());
    assert_eq!(payload["has_more"], false);
    let results = payload["results"].as_array().expect("results");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["path"], "wiki/index.md");
    assert!(results[0].get("snippet").is_none());
    assert!(results[0].get("backend").is_none());
}

fn run_mcp_session(home: &Path, project: &Path, requests: &[Value]) -> Vec<Value> {
    let mut child = Command::new(support::llm_wiki_bin())
        .args(["mcp", "serve"])
        .current_dir(project)
        .env("HOME", home)
        .env_remove("RUST_LOG")
        .env_remove("XDG_CACHE_HOME")
        .env_remove("XDG_DATA_HOME")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn llm-wiki mcp serve");

    {
        let stdin = child.stdin.as_mut().expect("stdin");
        for request in requests {
            writeln!(stdin, "{request}").expect("write request");
        }
    }

    let output = child.wait_with_output().expect("wait for server");
    assert!(
        output.status.success(),
        "server failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout)
        .expect("utf8 stdout")
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("json response"))
        .collect()
}

fn llm_wiki(home: &Path) -> Command {
    let mut command = Command::new(support::llm_wiki_bin());
    command
        .env("HOME", home)
        .env_remove("RUST_LOG")
        .env_remove("XDG_CACHE_HOME")
        .env_remove("XDG_DATA_HOME");
    command
}

fn install_without_llm_search(home: &Path) {
    let output = llm_wiki(home)
        .args(["install", "--skip-path-guidance", "--disable-llm-search"])
        .output()
        .expect("install output");
    assert!(
        output.status.success(),
        "install failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn tool_payload(response: &Value) -> Value {
    let text = response["result"]["content"][0]["text"]
        .as_str()
        .expect("tool text payload");
    serde_json::from_str(text).expect("json tool payload")
}

fn tool_error_message(response: &Value) -> String {
    assert_eq!(response["result"]["isError"], true);
    tool_payload(response)["error"]["message"]
        .as_str()
        .expect("tool error message")
        .to_string()
}

fn fixture_project(root: &Path) -> std::path::PathBuf {
    let project = root.join("project");
    fs::create_dir_all(project.join("wiki")).expect("wiki");
    fs::create_dir_all(project.join("raw")).expect("raw");
    fs::write(project.join("wiki/index.md"), "# Wiki Index\n").expect("index");
    fs::write(project.join("wiki/log.md"), "# Wiki Log\n").expect("log");
    fs::write(project.join("raw/source.md"), "source\n").expect("raw source");
    fs::write(project.join("AGENTS.md"), "# Agent Instructions\n").expect("agents");
    project
}
