use super::*;
use crate::seat::row::tests::journals;
use crate::summary::Account;

fn words(words: &[&str]) -> Vec<String> {
    words.iter().map(|word| word.to_string()).collect()
}

/// A sequence's steps read in the order first finished over every
/// attempt, a stale attempt's step after the retry's by its seq though
/// its row was opened first, each step once and one with no name passed
/// over; its members' tags in the order of each one's first record of
/// another step, a finished step's member tagging none.
#[test]
fn a_sequence_s_steps_and_tags_read_in_the_order_first_recorded() {
    let rows = Account::whole("run", &journals::sequenced())
        .seats()
        .to_vec();
    assert_eq!(
        inner(&rows, "s1"),
        (
            words(&["draft", "review", "stale"]),
            words(&["review:a", "review:b", "solo"]),
        )
    );
    assert_eq!(inner(&rows, "s2"), (Vec::new(), Vec::new()));
    assert_eq!(inner(&rows, "none"), (Vec::new(), Vec::new()));
}

/// A visit's models: every one its requests' checkpoints and their
/// concluding terminal reported, each once, a stale terminal's passed
/// over and another visit's apart.
#[test]
fn a_visit_s_models_are_its_requests_rows_reported_each_once() {
    let rows = Account::whole("run", &journals::sequenced())
        .seats()
        .to_vec();
    let set = |models: &[&str]| words(models).into_iter().collect::<BTreeSet<String>>();
    assert_eq!(models(&rows, 1, 17), set(&["m-1", "m-2", "m-3", "m-4"]));
    assert_eq!(models(&rows, 17, u64::MAX), set(&["m-5"]));
    assert_eq!(models(&rows, 0, 1), set(&[]));
    assert_eq!(models(&rows, 2, 2), set(&[]));
    assert_eq!(models(&rows, 2, 3), set(&["m-1", "m-2", "m-3", "m-4"]));
}

/// Records before their effect's request, read as the rail and the
/// dashboard read them: the step and the member they list come first, the
/// early model none; a terminal naming no attempt lists no model, as the
/// start before the request left its attempt open for the effect; and
/// that attempt's terminal lists its model on the effect's latest request,
/// the second visit's where the effect is asked again. The first request's
/// own row keeps what was listed before it, each member once; the second's
/// keeps only that model, as a record after the first request is its row's.
#[test]
fn records_before_a_request_list_and_its_open_attempt_concludes_per_effect() {
    let set = |models: &[&str]| words(models).into_iter().collect::<BTreeSet<String>>();
    let rows = Account::whole("run", &journals::unasked()).seats().to_vec();
    let listed = (words(&["plan", "draft"]), words(&["review:a"]));
    assert_eq!(inner(&rows, "u1"), listed);
    assert_eq!(models(&rows, 1, u64::MAX), set(&["m-late", "m-work"]));
    let again = journals::journal(journals::unasked_events(true));
    let rows = Account::whole("run", &again).seats().to_vec();
    assert_eq!(inner(&rows, "u1"), listed);
    assert_eq!(models(&rows, 1, 8), set(&["m-work"]));
    assert_eq!(models(&rows, 8, u64::MAX), set(&["m-late"]));
    let own = |asked: u64| {
        let mut owns = rows.iter().filter(|row| row.attempt_id.is_none());
        owns.find(|row| row.request_seq == asked)
            .map(|row| &row.listed)
    };
    let first = Listed {
        steps: vec![(2, "plan".to_string())],
        early: vec![(3, "review:a".to_string())],
        ..Listed::default()
    };
    assert_eq!(own(6), Some(&first));
    let models = words(&["m-late"]);
    assert_eq!(
        own(9),
        Some(&Listed {
            models,
            ..Listed::default()
        })
    );
}

/// A row lists each model it is served once, in the order first served,
/// and none for a record that reports none.
#[test]
fn a_row_lists_each_model_once_in_the_order_first_served() {
    let mut listed = Listed::default();
    for model in [Some("b"), None, Some("a"), Some("b"), Some("a")] {
        listed.served(model);
    }
    assert_eq!(listed.models, words(&["b", "a"]));
}

/// Each list a writer writes reads back as written: none, one entry, an
/// empty name or model, a seq past `u32`.
#[test]
fn a_list_reads_back_from_its_columns_as_written() {
    let pairs = |pairs: &[(u64, &str)]| -> Vec<(u64, String)> {
        let each = pairs.iter().map(|(seq, name)| (*seq, name.to_string()));
        each.collect()
    };
    let listed = |steps: &[(u64, &str)], tagged, models: &[&str], early: &[(u64, &str)]| Listed {
        steps: pairs(steps),
        tagged,
        models: words(models),
        early: pairs(early),
    };
    let lists = [
        listed(&[], None, &[], &[]),
        listed(
            &[(3, "draft"), (u64::MAX, "")],
            Some(2),
            &["m", ""],
            &[(1, "b")],
        ),
        listed(&[(1, "naïve:step")], None, &["a:b"], &[(2, "x:y"), (9, "")]),
    ];
    for list in lists {
        let (steps, models) = (list.steps_kept(), list.models_kept());
        let read = Listed::of_kept(steps, list.tagged, models, list.early_kept());
        assert_eq!(read.as_ref(), Some(&list), "{list:?}");
    }
    assert_eq!(
        listed(&[(3, "draft"), (12, "review")], None, &["m-1", "gpt"], &[]).steps_kept(),
        Some("1:35:draft2:126:review".to_string())
    );
    assert_eq!(
        listed(&[], None, &["m-1", "gpt"], &[]).models_kept(),
        Some("3:m-13:gpt".to_string())
    );
    assert_eq!(
        listed(&[], None, &[], &[(3, "review:a")]).early_kept(),
        Some("1:38:review:a".to_string())
    );
}

/// A list no writer writes is none: no entry, an entry cut short or past
/// its length, a length not as written, a char cut in two, a step with
/// no seq or a seq that is none, a step or a model twice, and steps out
/// of their seqs' order or two at one seq.
#[test]
fn a_list_no_writer_writes_reads_as_none() {
    let steps = [
        "",
        "1:3",
        "1:35:draf",
        "1:35:draftx",
        "01:35:draft",
        "+1:35:draft",
        "1:x5:draft",
        "1:35:draft1:4",
        "20:184467440737095516165:draft",
        "1:-5:draft",
        "1:35:draft1:45:draft",
        "1:45:draft1:36:review",
        "1:35:draft1:36:review",
        "1:31:\u{e9}",
    ];
    for text in steps {
        let read = Listed::of_kept(Some(text.to_string()), None, None, None);
        assert_eq!(read, None, "steps {text:?}");
        let read = Listed::of_kept(None, None, None, Some(text.to_string()));
        assert_eq!(read, None, "early {text:?}");
    }
    for text in ["", "3:m-13:m-1", "2:m", "x:m", "1:m1:"] {
        let read = Listed::of_kept(None, None, Some(text.to_string()), None);
        assert_eq!(read, None, "{text:?}");
    }
}
