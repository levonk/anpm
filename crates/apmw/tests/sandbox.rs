//! Test target for the sandbox harness module.
//!
//! Declares [`mod sandbox`] so that `cargo check --tests` compiles the harness.
//! The actual migration of integration tests to use the harness is story 02-001.

#[path = "sandbox/mod.rs"]
mod sandbox;

#[test]
fn sandbox_harness_smoke() {
  // default_profile produces a profile with the tempdir in fs_write.
  let profile = sandbox::default_profile(std::path::Path::new("/tmp/apmw-example"));
  assert!(profile.fs_write.contains(&"/tmp/apmw-example".to_string()));
  assert!(profile
    .net_allow
    .contains(&"registry.npmjs.org".to_string()));
  assert!(profile.net_deny.contains(&"*".to_string()));
  assert!(profile.env.contains(&"PATH".to_string()));
  assert!(profile.env.contains(&"HOME".to_string()));
  assert!(profile.env.contains(&"CARGO_HOME".to_string()));
  assert!(profile.env.contains(&"RUSTUP_HOME".to_string()));

  // sandbox_tempdir creates a deterministic dir under /tmp/apmw-tests/.
  let td = sandbox::sandbox_tempdir("sandbox_harness_smoke");
  assert!(td.path().exists());

  // write_profile serializes the profile to JSON.
  let prof_path = td.path().join("test-profile.json");
  sandbox::write_profile(&profile, &prof_path).expect("failed to write profile");
  assert!(prof_path.exists());

  // sandboxed_command builds a Command (does not execute it). It also
  // exercises ensure_nono_or_skip internally.
  let (td_cmd, _cmd) = sandbox::sandboxed_command("sandbox_harness_smoke_cmd");

  // sandboxed_command_in builds a Command for an existing TempDir (used by
  // tests that need to write fixtures before invoking apmw).
  let _cmd_in = sandbox::sandboxed_command_in(&td_cmd, "sandbox_harness_smoke_in");
}
