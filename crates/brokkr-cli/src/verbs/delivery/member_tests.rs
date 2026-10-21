use super::*;

#[test]
fn member_arguments_are_refused_before_the_existing_command_checks() {
    let retry = "--run, --reason, --realms, --db and --member, and with --member also --repo, --bundle, --recipe, --recipes-dir and --secrets-file";
    assert_eq!(accepted_arguments("retry"), retry);
    assert_eq!(
        accepted_arguments("stop"),
        "--run, --reason, --realms and --db"
    );
    for command in ["stop", "bogus", "acknowledge", "supersede"] {
        let args = MemberRetryArgs {
            member: vec!["x".into()],
            ..Default::default()
        };
        assert_eq!(
            validate(command, &args).unwrap_err().to_string(),
            "--member belongs to 'retry' alone"
        );
    }
    for (args, named) in [
        (
            MemberRetryArgs {
                repo: Some("repo".into()),
                delivery: DeliveryArgs {
                    recipe: Some("recipe".into()),
                    ..Default::default()
                },
                ..Default::default()
            },
            "--repo, --recipe",
        ),
        (
            MemberRetryArgs {
                delivery: DeliveryArgs {
                    secrets_file: Some("keys.env".into()),
                    ..Default::default()
                },
                ..Default::default()
            },
            "--secrets-file",
        ),
    ] {
        assert_eq!(
            validate("retry", &args).unwrap_err().to_string(),
            format!("{named} belong to 'retry --member' alone")
        );
    }
    let supersede_only = |command: &str, takes: &str| {
        format!("--findings, --by-run, --by-seq and --by-realm belong to 'supersede'; '{command}' takes {takes}")
    };
    for (command, members, expected) in [
        ("retry", vec![], supersede_only("retry", retry)),
        ("retry", vec!["x".into()], supersede_only("retry", retry)),
        (
            "stop",
            vec![],
            supersede_only("stop", "--run, --reason, --realms and --db"),
        ),
    ] {
        let args = OperatorArgs {
            run: "unused".into(),
            command: command.into(),
            reason: "test".into(),
            findings: vec![1],
            by_run: None,
            by_seq: None,
            by_realm: None,
            journal: JournalArgs {
                realms: None,
                db: None,
            },
            member: MemberRetryArgs {
                member: members,
                ..Default::default()
            },
        };
        assert_eq!(
            operator(Path::new("."), args).unwrap_err().to_string(),
            expected
        );
    }
}

#[test]
fn the_member_form_operates_the_folder_root_from_a_subdirectory_unless_repo_is_typed() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    for entry in [".git", ".brokkr", "nested/deeper"] {
        std::fs::create_dir_all(root.join(entry)).unwrap();
    }
    let layout = Layout::discover(&root.join("nested/deeper")).unwrap();
    let journal = (root.join("forge.db"), true);
    let request = |repo: Option<PathBuf>| {
        let delivery = DeliveryArgs {
            bundle: Some("bundle".into()),
            ..Default::default()
        };
        super::request(&layout, delivery, journal.clone(), repo).unwrap()
    };
    assert_eq!(request(None).repo, Some(root.clone()));
    assert_eq!(
        request(Some("elsewhere".into())).repo,
        Some(root.join("nested/deeper/elsewhere"))
    );
}
