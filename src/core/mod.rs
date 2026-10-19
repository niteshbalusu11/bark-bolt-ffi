pub mod notification;
pub mod onchain;
pub mod wallet;

use anyhow::Context;

use crate::error::BarkError;

/// Parse a Bitcoin address and require it to belong to `network`.
///
/// `assume_checked` would discard the network, and `script_pubkey` derives
/// from the payload alone, so a wrong-network address still builds a valid
/// output on the wallet's real network, paying a key nobody meant to use there.
pub(crate) fn parse_address(
    address: &str,
    network: bitcoin::Network,
) -> Result<bitcoin::Address, BarkError> {
    let parsed = address
        .trim()
        .parse::<bitcoin::Address<bitcoin::address::NetworkUnchecked>>()
        .context("invalid bitcoin address")?;

    parsed
        .require_network(network)
        .with_context(|| format!("bitcoin address is not valid on {}", network))
        .map_err(BarkError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::Network;

    /// Mainnet vs the testnet family is the split a user actually hits.
    const MAINNET_P2WPKH: &str = "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4";
    const REGTEST_P2WPKH: &str = "bcrt1qw508d6qejxtdg4y5r3zarvary0c5xw7kygt080";

    #[test]
    fn accepts_an_address_for_the_wallets_network() {
        let addr = parse_address(MAINNET_P2WPKH, Network::Bitcoin).unwrap();
        assert_eq!(addr.to_string(), MAINNET_P2WPKH);
    }

    #[test]
    fn surrounding_whitespace_is_tolerated() {
        let addr = parse_address(&format!("  {}\n", MAINNET_P2WPKH), Network::Bitcoin).unwrap();
        assert_eq!(addr.to_string(), MAINNET_P2WPKH);
    }

    #[test]
    fn refuses_an_address_from_another_network() {
        let err = parse_address(REGTEST_P2WPKH, Network::Bitcoin)
            .expect_err("a regtest address must not be accepted by a mainnet wallet");
        assert!(err.message().contains("not valid on"), "{}", err.message());

        let err = parse_address(MAINNET_P2WPKH, Network::Regtest)
            .expect_err("a mainnet address must not be accepted by a regtest wallet");
        assert!(err.message().contains("not valid on"), "{}", err.message());
    }

    /// The old path turned a regtest address into a spendable mainnet script.
    #[test]
    fn a_wrong_network_address_no_longer_yields_a_spendable_script() {
        let leaked = REGTEST_P2WPKH
            .parse::<bitcoin::Address<bitcoin::address::NetworkUnchecked>>()
            .unwrap()
            .assume_checked()
            .script_pubkey();
        assert!(leaked.is_p2wpkh(), "old path produced a spendable script");
        assert!(parse_address(REGTEST_P2WPKH, Network::Bitcoin).is_err());
    }

    #[test]
    fn refuses_a_malformed_address() {
        let err = parse_address("not-an-address", Network::Bitcoin)
            .expect_err("a malformed address should be refused");
        assert!(
            err.message().contains("invalid bitcoin address"),
            "{}",
            err.message()
        );
    }
}
pub mod state_changes;
