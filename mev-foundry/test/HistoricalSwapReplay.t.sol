// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {Test} from "forge-std/Test.sol";
import {IStateView} from "@uniswap/v4-periphery/src/interfaces/IStateView.sol";
import {PoolId} from "@uniswap/v4-core/src/types/PoolId.sol";

/**
 * @title Проверка, что мы можем получить стейт после конкретного свопа
 * @notice A hook implementing AMM-Based dynamic fees.
 */
contract HistoricalSwapReplayTest is Test {
    IStateView internal constant STATE_VIEW =
        IStateView(0x7fFE42C4a5DEeA5b0feC41C94C136Cf115597227); // уже развернутый контракт в UniswapV4
    // обертка над storage PoolManager
    // нужна для чтения состояния конкретного v4-пула по PoolId

    // наш WETH / USDC пул
    PoolId internal constant PROXY_POOL_ID =
        PoolId.wrap(
            0x5a5c7cab5f55c7ea020e97d4fa6dd5d99270e56ce76afa61d8cbddec0af92060
        );

    // одна из исторических транзакций в нашем пуле
    // лучше выбирать транзакцию, которая была в начале блоке (иначе createSelectFork будет очень долго висеть, исполняя все транзакции)
    // Конкретно здесь 13-я транзакция : и она считалась 2 минуты
    bytes32 internal constant TX_HASH =
        0x0016e97aacae336f0fdb247755d575b19596ed2d4fb5f8d0cd12558e17bb5863;

    // Вот 83-я транзакция для примера: 0xead0ff99b8d7b474a52cd7ffecb1bb0ffb140d0f3830570e5116b94b7ca46800;

    // вот самый ранни своп для теста
    // block=26310583
    // tx_index=0
    // tx=0x3d0cce2fae33dd7b0e8503a432f30dcb5eb403e6dfeec91dd6c3cc6bd981832a

    function test_replaysHistoricalProxySwap() external {
        // в .env должен лежать RPC URL
        emit log("1. creating fork before historical transaction");
        vm.createSelectFork(vm.envString("MAINNET_RPC_URL"), TX_HASH);
        emit log("2. fork created");

        // читаем состояние пула ДО транзакции
        (uint160 sqrtPriceBefore, int24 tickBefore, , ) = STATE_VIEW.getSlot0(
            PROXY_POOL_ID
        );

        emit log("3. replaying historical transaction");
        vm.transact(TX_HASH);
        emit log("4. transaction replayed");

        (uint160 sqrtPriceAfter, int24 tickAfter, , ) = STATE_VIEW.getSlot0(
            PROXY_POOL_ID
        );

        assertTrue(sqrtPriceAfter != sqrtPriceBefore);
        assertTrue(tickAfter != tickBefore);
    }

    function test_createsForkBeforeTransaction() external {
        vm.createSelectFork(vm.envString("MAINNET_RPC_URL"), TX_HASH);

        assertEq(block.chainid, 1);
    }
}
