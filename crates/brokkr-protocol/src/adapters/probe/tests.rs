use super::{grammar, AdapterKind};

#[test]
fn every_adapter_reads_its_short_model_flag_from_its_grammar() {
    assert_eq!(AdapterKind::Codex.model_flag(), Some("-m"));
    for &kind in AdapterKind::ALL {
        let wire_name = kind.command_name();
        let alias = grammar::grammar(wire_name)
            .and_then(|table| {
                table
                    .options
                    .iter()
                    .find(|spec| spec.canonical == "--model")
            })
            .and_then(|spec| spec.aliases.first().copied());
        assert_eq!(kind.model_flag(), alias, "{wire_name}");
    }
}

#[test]
fn every_grammar_model_option_has_at_most_one_alias() {
    for table in grammar::TABLES {
        for spec in table
            .options
            .iter()
            .filter(|spec| spec.canonical == "--model")
        {
            assert!(
                spec.aliases.len() <= 1,
                "{} declares multiple --model aliases: {:?}",
                table.harness,
                spec.aliases
            );
        }
    }
}
