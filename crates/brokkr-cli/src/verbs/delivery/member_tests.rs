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
    let args = MemberRetryArgs {
        repo: Some("repo".into()),
        delivery: DeliveryArgs {
            recipe: Some("recipe".into()),
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(
        validate("retry", &args).unwrap_err().to_string(),
        "--repo, --recipe belong to 'retry --member' alone"
    );
    let expected = format!("--findings, --by-run, --by-seq and --by-realm belong to 'supersede'; 'retry' takes {retry}");
    for members in [vec![], vec!["x".into()]] {
        let args = OperatorArgs {
            run: "unused".into(),
            command: "retry".into(),
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
