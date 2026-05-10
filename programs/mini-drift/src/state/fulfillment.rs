use anchor_lang::solana_program::pubkey::Pubkey;

#[derive(Debug, PartialEq, Eq)]
pub enum PerpFulfillmentMethod {
    AMM(Option<u64>),
    Match(Pubkey, u16, u64),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perp_fulfillment_method_amm_some_stores_stop_price() {
        let method = PerpFulfillmentMethod::AMM(Some(10));
        assert_eq!(method, PerpFulfillmentMethod::AMM(Some(10)))
    }

    #[test]
    fn perp_fulfillment_method_amm_none_stores_no_stop_price() {
        let method = PerpFulfillmentMethod::AMM(None);
        assert_eq!(method, PerpFulfillmentMethod::AMM(None))
    }

    #[test]
    fn perp_fulfillment_method_match_stores_maker_identity_order_id_and_price() {
        let method = PerpFulfillmentMethod::Match(Pubkey::default(), 12, 12);
        assert_eq!(
            method,
            PerpFulfillmentMethod::Match(Pubkey::default(), 12, 12)
        )
    }
}
