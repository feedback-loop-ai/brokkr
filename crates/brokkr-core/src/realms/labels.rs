//! A realms map's labels, judged in the order the loader has always
//! judged them (#288): the boundary word before the shape is read, then
//! each realm's own fields in table order, then the crossings against
//! the rest of the world. [`super::RealmMap::of`] keeps the one entry
//! point and the one refusal wrapper; these helpers only answer whether
//! a label is usable, and why not.

use serde_json::Value;

use super::{
    grants, is_name, is_repository_relative, older_than, Boundary, Realm, RealmMap, RealmsError,
    Unusable, Word, SCHEMAS,
};

/// The boundary word, judged before the shape is read, so a refusal can
/// name the realm that wrote it and the five words it may write
/// (decision 0046 ruling 1) rather than serde's anonymous "unknown
/// variant". Only a string is judged here; any other shape is the
/// malformed-map refusal, as for every other field.
pub(super) fn judge_boundaries(content: &Value) -> Result<(), Unusable> {
    for realm in content
        .get("realms")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if let Some(word) = realm.get("boundary").and_then(Value::as_str) {
            if let Err(error) = word.parse::<Boundary>() {
                return Err(Unusable::Boundary {
                    realm: realm
                        .get("name")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    error,
                });
            }
        }
    }
    Ok(())
}

/// The map as serde reads it, with its own account of why it does not.
pub(super) fn read_map(path: &str, content: &Value) -> Result<RealmMap, RealmsError> {
    serde_json::from_value(content.clone()).map_err(|error| RealmsError::Malformed {
        path: path.to_string(),
        detail: error.to_string(),
    })
}

/// The map must call itself a version this build reads.
pub(super) fn check_schema(map: &RealmMap) -> Result<(), Unusable> {
    if !SCHEMAS.contains(&map.schema.as_str()) {
        return Err(Unusable::UnknownSchema(map.schema.clone()));
    }
    Ok(())
}

/// The v6 word, held to its version BEFORE its shape is read, and held
/// even when written empty (decision 0065 ruling 3): presence is what a
/// version gate judges, so `capabilities: {}` under an older label is
/// refused rather than shrugged at. Under v6 the grants are judged here,
/// once, and every reader takes them from [`Realm::grants`].
pub(super) fn load_grants(map: &mut RealmMap) -> Result<(), Unusable> {
    let schema = map.schema.clone();
    for realm in &mut map.realms {
        let Some(capabilities) = &realm.capabilities else {
            continue;
        };
        if older_than(&schema, Word::Capabilities.introduced()) {
            return Err(unversioned(realm, Word::Capabilities, &schema));
        }
        realm.grants =
            grants::parse_grants(&realm.name, &schema, capabilities).map_err(Unusable::from)?;
    }
    Ok(())
}

/// The world itself must hold a realm and a journal.
pub(super) fn check_world(map: &RealmMap) -> Result<(), Unusable> {
    if map.realms.is_empty() {
        return Err(Unusable::NoRealms);
    }
    if map.journal.trim().is_empty() {
        return Err(Unusable::EmptyJournal);
    }
    Ok(())
}

/// Every realm's own labels, in table order.
pub(super) fn judge_realms(map: &RealmMap) -> Result<(), Unusable> {
    for index in 0..map.realms.len() {
        judge_realm(map, index)?;
    }
    Ok(())
}

/// One realm: its identity, its paths and every versioned word it wrote,
/// in the order the map has always checked them.
fn judge_realm(map: &RealmMap, index: usize) -> Result<(), Unusable> {
    let realm = &map.realms[index];
    let name = realm.name.clone();
    if !is_name(&realm.name) {
        return Err(Unusable::Name { index, name });
    }
    if realm.path.trim().is_empty() {
        return Err(Unusable::NoPath(name));
    }
    if realm.default_branch.trim().is_empty() {
        return Err(Unusable::NoBranch(name));
    }
    if map.realms[..index].iter().any(|e| e.name == realm.name) {
        return Err(Unusable::NamedTwice(name));
    }
    judge_journal(map, realm)?;
    // The v4 word, held to its version exactly as the v2 and v3 words
    // are held to theirs (decision 0046 ruling 1).
    if realm.boundary.is_some() && older_than(&map.schema, Word::Boundary.introduced()) {
        return Err(unversioned(realm, Word::Boundary, &map.schema));
    }
    judge_house_and_dialect(map, realm)?;
    judge_crossing_lists(map, realm)?;
    judge_published(realm)?;
    judge_consumed(realm)?;
    Ok(())
}

/// The one new word, held to the version that introduced it.
/// `deny_unknown_fields` cannot do this job any more — the field is
/// known to the reader now — so the refusal is written out, and it names
/// the version that would admit it rather than merely saying no.
fn judge_journal(map: &RealmMap, realm: &Realm) -> Result<(), Unusable> {
    match &realm.journal {
        Some(_) if older_than(&map.schema, Word::Journal.introduced()) => {
            Err(unversioned(realm, Word::Journal, &map.schema))
        }
        Some(journal) if journal.trim().is_empty() => Err(Unusable::Empty {
            realm: realm.name.clone(),
            word: Word::Journal,
        }),
        _ => Ok(()),
    }
}

/// The two v3 words, each held to its version exactly as the words before
/// them are held to theirs.
fn judge_house_and_dialect(map: &RealmMap, realm: &Realm) -> Result<(), Unusable> {
    for (word, value) in [(Word::House, &realm.house), (Word::Dialect, &realm.dialect)] {
        match value {
            Some(_) if older_than(&map.schema, word.introduced()) => {
                return Err(unversioned(realm, word, &map.schema))
            }
            Some(value) if value.trim().is_empty() => {
                return Err(Unusable::Empty {
                    realm: realm.name.clone(),
                    word,
                })
            }
            Some(value) if !is_repository_relative(value) => {
                return Err(Unusable::Outside {
                    realm: realm.name.clone(),
                    word,
                })
            }
            _ => {}
        }
    }
    Ok(())
}

/// The two v5 lists, held to their version exactly as every word before
/// them is held to its own (decision 0057 ruling 3). Presence is what is
/// judged: a realm that draws no crossing is a realm as it was before the
/// word existed — and a realm that WROTE the word as `null` wrote it,
/// which is why [`super::CrossingList`] keeps the two apart. A written
/// `null` is refused on its own terms after the version gate has had its
/// say, because under an older label the deeper fault is the word itself.
fn judge_crossing_lists(map: &RealmMap, realm: &Realm) -> Result<(), Unusable> {
    for (word, written, null) in [
        (
            Word::Publishes,
            realm.publishes.is_written(),
            realm.publishes.is_null(),
        ),
        (
            Word::Consumes,
            realm.consumes.is_written(),
            realm.consumes.is_null(),
        ),
    ] {
        if written && older_than(&map.schema, word.introduced()) {
            return Err(unversioned(realm, word, &map.schema));
        }
        if null {
            return Err(Unusable::NullList {
                realm: realm.name.clone(),
                word,
            });
        }
    }
    Ok(())
}

/// What a realm publishes is judged on its own terms: a name that can be
/// read back out of evidence, a file inside the realm that owns it, and
/// no name used twice.
fn judge_published(realm: &Realm) -> Result<(), Unusable> {
    let owner = realm.name.clone();
    let published = realm.published();
    for (index, entry) in published.iter().enumerate() {
        let crossing = entry.name.clone();
        if !is_name(&entry.name) {
            return Err(Unusable::CrossingName {
                realm: owner,
                crossing,
            });
        }
        if entry.path.trim().is_empty() {
            return Err(Unusable::NoCrossingPath {
                realm: owner,
                crossing,
            });
        }
        if !is_repository_relative(&entry.path) {
            return Err(Unusable::CrossingOutside {
                realm: owner,
                crossing,
            });
        }
        if published[..index].iter().any(|e| e.name == crossing) {
            return Err(Unusable::PublishedTwice {
                realm: owner,
                crossing,
            });
        }
    }
    Ok(())
}

/// What a realm consumes is judged on its own terms here — the pin's
/// shape, the crossing being between realms, and one name meaning one
/// file. Against the rest of the world is [`judge_publishers`]'s work,
/// once every realm's own shape is known.
fn judge_consumed(realm: &Realm) -> Result<(), Unusable> {
    let owner = realm.name.clone();
    let consumed = realm.consumed();
    for (index, entry) in consumed.iter().enumerate() {
        let crossing = entry.name.clone();
        if !crate::canonical::is_sha256_hex(&entry.sha256) {
            let pin = entry.sha256.clone();
            return Err(Unusable::Pin {
                realm: owner,
                crossing,
                pin,
            });
        }
        if entry.realm == owner {
            return Err(Unusable::ConsumesItself {
                realm: owner,
                crossing,
            });
        }
        if consumed[..index].iter().any(|e| e.name == crossing) {
            return Err(Unusable::ConsumedTwice {
                realm: owner,
                crossing,
            });
        }
    }
    Ok(())
}

/// Every crossing consumed is a crossing published, by a realm this world
/// holds. Judged after each realm's own shape stands, so a name is
/// resolved only against realms that already passed [`judge_realm`].
pub(super) fn judge_publishers(map: &RealmMap) -> Result<(), Unusable> {
    for realm in &map.realms {
        for crossing in realm.consumed() {
            let (consumer, publisher, name) = (
                realm.name.clone(),
                crossing.realm.clone(),
                crossing.name.clone(),
            );
            let Some(publishing) = map.realms.iter().find(|e| e.name == publisher) else {
                return Err(Unusable::NoPublisher {
                    realm: consumer,
                    crossing: name,
                    publisher,
                });
            };
            if !publishing.published().iter().any(|e| e.name == name) {
                return Err(Unusable::Unpublished {
                    realm: consumer,
                    crossing: name,
                    publisher,
                });
            }
        }
    }
    Ok(())
}

/// The refusal for a versioned word the map's label predates.
fn unversioned(realm: &Realm, word: Word, schema: &str) -> Unusable {
    Unusable::Unversioned {
        realm: realm.name.clone(),
        word,
        schema: schema.to_string(),
    }
}
