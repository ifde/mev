// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {Test} from "forge-std/Test.sol";
import {IStateView} from "@uniswap/v4-periphery/src/interfaces/IStateView.sol";
import {IV4Quoter} from "@uniswap/v4-periphery/src/interfaces/IV4Quoter.sol";
import {PathKey} from "@uniswap/v4-periphery/src/libraries/PathKey.sol";
import {Currency} from "@uniswap/v4-core/src/types/Currency.sol";
import {IHooks} from "@uniswap/v4-core/src/interfaces/IHooks.sol";
import {PoolId} from "@uniswap/v4-core/src/types/PoolId.sol";

contract HistoricalBackrunQuoteTest is Test {
    IStateView internal constant STATE_VIEW =
        IStateView(0x7fFE42C4a5DEeA5b0feC41C94C136Cf115597227);

    IV4Quoter internal constant V4_QUOTER =
        IV4Quoter(0x99bd42B2B22e531f283b7F8a86dFf631EB16A9D1);

    address internal constant USDC = 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48;

    address internal constant WETH = 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2;

    PoolId internal constant PROXY_POOL_ID =
        PoolId.wrap(
            0x5a5c7cab5f55c7ea020e97d4fa6dd5d99270e56ce76afa61d8cbddec0af92060
        );

    bytes32 internal constant VICTIM_TX_HASH =
        0x0016e97aacae336f0fdb247755d575b19596ed2d4fb5f8d0cd12558e17bb5863;

    function test_quotesBackrunAfterHistoricalProxySwap() external {
        vm.createSelectFork(vm.envString("MAINNET_RPC_URL"), VICTIM_TX_HASH);

        (uint160 sqrtPriceBefore, , , ) = STATE_VIEW.getSlot0(PROXY_POOL_ID);

        vm.transact(VICTIM_TX_HASH);

        (uint160 sqrtPriceAfter, , , ) = STATE_VIEW.getSlot0(PROXY_POOL_ID);

        assertTrue(sqrtPriceAfter != sqrtPriceBefore);

        PathKey[] memory path = new PathKey[](2);

        // USDC -> WETH в proxy-пуле
        path[0] = PathKey({
            intermediateCurrency: Currency.wrap(WETH),
            fee: 200,
            tickSpacing: 4,
            hooks: IHooks(address(0)),
            hookData: bytes("")
        });

        // WETH -> USDC в reference-пуле
        path[1] = PathKey({
            intermediateCurrency: Currency.wrap(USDC),
            fee: 500,
            tickSpacing: 10,
            hooks: IHooks(address(0)),
            hookData: bytes("")
        });

        uint128[3] memory amounts = [
            uint128(200_000), // 0.2 USDC
            uint128(2_000_000), // 2 USDC
            uint128(20_000_000) // 20 USDC
        ];

        for (uint256 index = 0; index < amounts.length; ++index) {
            uint128 amountIn = amounts[index];

            (uint256 amountOut, uint256 quoterGasEstimate) = V4_QUOTER
                .quoteExactInput(
                    IV4Quoter.QuoteExactParams({
                        exactCurrency: Currency.wrap(USDC),
                        path: path,
                        exactAmount: amountIn
                    })
                );

            emit log_named_uint("grid_index", index);
            emit log_named_uint("amount_in_wei", amountIn);
            emit log_named_uint("amount_out_wei", amountOut);
            emit log_named_int(
                "gross_profit_wei",
                int256(amountOut) - int256(uint256(amountIn))
            );
            emit log_named_uint("quoter_gas_estimate", quoterGasEstimate);
        }
    }
}
