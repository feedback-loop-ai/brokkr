use super::{known_powers, unready, Controls, Failure, Inventory, Piece};

/// The provider's known-power floor (design D6): every capability its
/// launch can reach natively is held or switched off by a plan resolved
/// for it from a measured inventory, or nothing is composed.
pub(super) fn floor(provider: &str, controls: &Controls) -> Result<(), Failure> {
    for capability in known_powers(provider) {
        if let Inventory::Unmeasured(reason) = &controls.inventory {
            return Err(unready(
                provider,
                capability,
                vec![
                    Piece::Words("declares the provider's inventory unmeasured ("),
                    Piece::Reason(reason),
                    Piece::Words(")"),
                ],
            )
            .into());
        }
        if controls.harness != provider {
            return Err(unready(
                provider,
                capability,
                vec![
                    Piece::Words("was resolved for "),
                    Piece::Harness(&controls.harness),
                ],
            )
            .into());
        }
        let answered = |names: &[String]| names.iter().any(|name| name == capability);
        if !answered(&controls.held) && !answered(&controls.denied) {
            return Err(unready(
                provider,
                capability,
                vec![Piece::Words("neither holds it nor switches it off")],
            )
            .into());
        }
    }
    Ok(())
}
