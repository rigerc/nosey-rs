use super::{BTreeSet, ChangeRequest, InstallReason, State, TestResult, fixture, fs, id};

#[test]
fn target_sync_previews_global_migration_and_preserves_unmanaged_paths() -> TestResult {
    use nosey_core::targets::{AgentTarget, InstallScope, TargetConfig};
    let mut f = fixture()?;
    let home = tempfile::tempdir()?;
    for target in [".agents/skills", ".claude/skills"] {
        fs::create_dir_all(home.path().join(target))?;
    }
    let scope = InstallScope::Global {
        home: home.path().to_path_buf(),
    };
    let plan = f.library.plan(
        &f.state,
        &[ChangeRequest::InstallScoped {
            scope,
            target: None,
            artifact: id("alpha"),
            reason: InstallReason::Manual,
        }],
    )?;
    f.library.apply_plan(&f.state, &plan)?;
    let other = ChangeRequest::InstallScoped {
        scope: InstallScope::Global {
            home: home.path().to_path_buf(),
        },
        target: None,
        artifact: id("alpha"),
        reason: InstallReason::Recommendation,
    };
    f.library
        .apply_plan(&f.state, &f.library.plan(&f.state, &[other])?)?;
    let old = home.path().join(".agents/skills/alpha");
    let new = home.path().join(".claude/skills/alpha");
    let unmanaged = home.path().join(".agents/skills/beta");
    fs::create_dir(&unmanaged)?;
    f.state.set_targets(TargetConfig {
        agents: BTreeSet::from([AgentTarget::Claude]),
        custom: BTreeSet::new(),
    })?;
    let plan = f.library.plan_target_sync(&f.state)?;
    assert_eq!(plan.eligible(), 4);
    assert!(old.is_symlink());
    assert!(!new.is_symlink());
    f.state.set_targets(TargetConfig::default())?;
    assert!(f.library.apply_plan(&f.state, &plan).is_err());
    f.state.set_targets(TargetConfig {
        agents: BTreeSet::from([AgentTarget::Claude]),
        custom: BTreeSet::new(),
    })?;
    fs::create_dir(&new)?;
    let blocked = f.library.plan_target_sync(&f.state)?;
    f.library.apply_plan(&f.state, &blocked)?;
    assert!(old.is_symlink());
    fs::remove_dir(&new)?;
    f.library.apply_plan(&f.state, &plan)?;
    assert!(new.is_symlink());
    assert_eq!(f.library.audit_links(&f.state)?[0].reasons.len(), 2);
    assert!(!old.is_symlink());
    assert!(unmanaged.is_dir());
    Ok(())
}

#[test]
fn preset_sync_preserves_overlapping_reasons_and_rejects_old_membership_plan() -> TestResult {
    use nosey_core::{
        presets::{Preset, PresetApplication},
        targets::InstallScope,
    };
    let mut f = fixture()?;
    let state_path = f.sources.join("state.json");
    fs::write(&state_path, r#"{"projects":{}}"#)?;
    assert!(State::load(&state_path)?.preset("a").is_none());
    fs::create_dir_all(f.root.join(".agents/skills"))?;
    let scope = InstallScope::Project {
        root: f.root.clone(),
    };
    let manual = ChangeRequest::InstallScoped {
        scope: scope.clone(),
        target: None,
        artifact: id("alpha"),
        reason: InstallReason::Manual,
    };
    f.library
        .apply_plan(&f.state, &f.library.plan(&f.state, &[manual])?)?;
    for preset in ["a", "b"] {
        f.state.set_preset(Preset {
            id: preset.into(),
            name: preset.into(),
            members: BTreeSet::from([id("alpha")]),
        })?;
        f.state.set_preset_application(
            PresetApplication {
                preset: preset.into(),
                scope: scope.clone(),
            },
            true,
        )?;
    }
    let pending = f.library.plan_preset_sync(&f.state)?;
    assert_eq!(pending.eligible(), 2);
    assert_eq!(f.library.audit_links(&f.state)?[0].reasons.len(), 1);
    f.library.apply_plan(&f.state, &pending)?;
    assert_eq!(f.library.audit_links(&f.state)?[0].reasons.len(), 3);
    f.state.set_preset(Preset {
        id: "a".into(),
        name: "A".into(),
        members: BTreeSet::from([id("beta")]),
    })?;
    assert!(f.library.apply_plan(&f.state, &pending).is_err());
    f.state.set_preset_application(
        PresetApplication {
            preset: "b".into(),
            scope,
        },
        false,
    )?;
    f.state.save(&state_path)?;
    let loaded = State::load(&state_path)?;
    let pending = f.library.plan_preset_sync(&loaded)?;
    assert_eq!(pending.eligible(), 3);
    f.library.apply_plan(&loaded, &pending)?;
    let links = f.library.audit_links(&loaded)?;
    assert_eq!(links[0].reasons, BTreeSet::from([InstallReason::Manual]));
    assert_eq!(
        links[1].reasons,
        BTreeSet::from([InstallReason::Preset("a".into())])
    );
    assert_eq!(f.library.plan_preset_sync(&loaded)?.changes, Vec::new());
    Ok(())
}
