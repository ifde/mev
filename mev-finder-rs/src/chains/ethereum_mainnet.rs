// Исторические holder-адреса используются только для локального финансирования
// direct replay в Anvil. Они не используются как sender пользовательского swap.

use alloy::primitives::{Address, address};

const USDC: Address = address!("A0b86991c6218b36c1d19d4a2e9eb0ce3606eb48");
const WETH: Address = address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");

const USDC_HOLDER: Address = address!("28C6c06298d514Db089934071355E5743bf21d60");
const WETH_HOLDER: Address = address!("4d5F47FA6A74757f35C14fD3a6Ef8E3C9BC514E8");

pub fn funding_holder_for_token(token: Address) -> Option<Address> {
    if token == USDC {
        Some(USDC_HOLDER)
    } else if token == WETH {
        Some(WETH_HOLDER)
    } else {
        None
    }
}
